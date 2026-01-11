//! Intel 4040 CPU Implementation
//!
//! The 4040 is an enhanced version of the 4004 with:
//! - 14 new instructions (60 total)
//! - 24 registers (2 banks)
//! - 7-level stack
//! - Interrupt support
//! - Single-step mode (HALT/STOP)

mod instruction_decode;
mod interrupt;
mod registers;
mod stack;

use instruction_decode::{Instruction, InstructionDecoder};
use interrupt::InterruptCtrl;
use mcs4_bus::prelude::*;
use registers::RegFile;
use stack::CallStack;

use crate::i4004::{Alu, TimingIo};

/// Intel 4040 CPU
pub struct I4040 {
    /// ALU (Arithmetic Logic Unit)
    pub alu: Alu,

    /// Register file (24 registers, 2 banks)
    pub registers: RegFile,

    /// Call stack (7 levels)
    pub stack: CallStack,

    /// Interrupt controller
    pub intr: InterruptCtrl,

    /// Instruction decoder
    pub decoder: InstructionDecoder,

    /// Timing and I/O control
    pub timing: TimingIo,

    /// Current cycle state
    cycle: CycleState,

    /// Fetched instruction (OPR:OPA)
    instruction_byte: u8,

    /// Currently selected RAM address (from SRC)
    ram_address: u8,

    /// Currently selected RAM chip
    ram_chip: u8,

    /// Currently selected RAM bank (set by DCL / SB0 / SB1)
    ram_bank: u8,

    /// Test pin input (directly readable via JCN)
    test_pin: bool,

    /// Pending I/O data
    io_data: u8,

    /// Decoded high-level I/O operation for the current instruction (used for phase-accurate control lines).
    decoded_io_op: Option<IoOp>,

    /// Halted state (HLT instruction)
    pub halted: bool,

    /// True if an instruction explicitly updated PC (taken branches, jumps, calls, returns).
    pc_modified: bool,

    /// Program Counter
    pc: u16,
}

impl I4040 {
    pub fn new() -> Self {
        Self {
            alu: Alu::new(),
            registers: RegFile::new(),
            stack: CallStack::new(),
            intr: InterruptCtrl::default(),
            decoder: InstructionDecoder::new(),
            timing: TimingIo::new(),
            cycle: CycleState::new(),
            instruction_byte: 0,
            ram_address: 0,
            ram_chip: 0,
            ram_bank: 0,
            test_pin: false,
            io_data: 0,
            decoded_io_op: None,
            halted: false,
            pc_modified: false,
            pc: 0,
        }
    }

    /// Get currently selected RAM bank (set by DCL / SB0 / SB1)
    pub fn ram_bank(&self) -> u8 {
        self.ram_bank
    }

    /// True if X3 should run CPU before peripherals (SRC drives the bus in X3).
    pub fn x3_cpu_drives_first(&self) -> bool {
        matches!(self.decoder.get_instruction(), Some(Instruction::Src { .. }))
    }

    /// True if the current instruction needs CM-RAM asserted during X2.
    pub fn x2_ram_bank_select(&self) -> bool {
        matches!(
            self.decoded_io_op,
            Some(IoOp::Src | IoOp::RamMainWrite | IoOp::RamPortWrite | IoOp::RamStatusWrite(_))
        )
    }

    /// True if the current instruction needs CM-RAM asserted during X3.
    pub fn x3_ram_bank_select(&self) -> bool {
        matches!(
            self.decoded_io_op,
            Some(IoOp::Src | IoOp::RamMainRead | IoOp::RamStatusRead(_))
        )
    }

    /// I/O op that peripherals should observe during X3 (before the CPU latches the bus).
    pub fn x3_peripheral_io_op(&self) -> Option<IoOp> {
        match self.decoded_io_op {
            Some(op @ (IoOp::RamMainRead | IoOp::RomPortRead | IoOp::RamStatusRead(_))) => Some(op),
            _ => None,
        }
    }

    /// Set the test pin state (used by JCN condition bit 0)
    pub fn set_test_pin(&mut self, state: bool) {
        self.test_pin = state;
    }

    /// Process one bus phase
    pub fn tick(&mut self, phase: BusCycle, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        tracing::trace!(?phase, pc=%self.pc(), "CPU 4040 Tick");
        // Handle interrupt request at specific phases if needed,
        // but typically 4040 samples INT at the instruction fetch boundary.
        // We check it at A1 of the first cycle.

        match phase {
            BusCycle::A1 => self.phase_a1(bus, ctrl),
            BusCycle::A2 => self.phase_a2(bus, ctrl),
            BusCycle::A3 => self.phase_a3(bus, ctrl),
            BusCycle::M1 => self.phase_m1(bus),
            BusCycle::M2 => self.phase_m2(bus),
            BusCycle::X1 => self.phase_x1(bus, ctrl),
            BusCycle::X2 => self.phase_x2(bus, ctrl),
            BusCycle::X3 => self.phase_x3(bus, ctrl),
        }
        self.cycle.advance();
    }

    pub fn pc(&self) -> u16 {
        self.pc
    }

    pub fn ram_address(&self) -> u8 {
        self.ram_address & 0x0F
    }

    pub fn ram_chip(&self) -> u8 {
        self.ram_chip & 0x0F
    }

    fn phase_a1(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        ctrl.clear_io_op();
        ctrl.deselect_ram(0);
        // Interrupt Handling (Start of instruction cycle)
        // If not already in second cycle of a 2-byte instruction...
        if !self.cycle.second_cycle && !self.cycle.two_cycle {
            if ctrl.interrupt_pending() {
                self.intr.request();
            }
            if let Some(vector) = self.intr.service(self.current_src()) {
                // Push current PC to stack
                // Note: The PC has not been incremented for the NEXT instruction yet,
                // but we are at the start of fetching the NEXT instruction.
                // So pushing `self.pc` is correct (it points to the instruction we were about to fetch).
                if self.stack.push(self.pc).is_err() {
                    // Stack overflow handling? Real 4040 wraps or corrupts?
                    // For now, we just proceed (maybe log error)
                }
                self.pc = vector;
                self.halted = false; // Interrupt wakes up HLT
            }
        }

        if self.halted {
            // If halted, we don't fetch/execute, but we might keep the bus idle?
            // Real 4040 performs "dummy" cycles or just stops.
            // For this emulator, we'll just return.
            return;
        }

        // Output address bits 0-3 and assert SYNC
        let addr = self.pc;
        bus.write((addr & 0x0F) as u8);
        ctrl.assert_sync(0);
    }

    fn phase_a2(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        if self.halted {
            return;
        }
        // Output address bits 4-7, deassert SYNC
        let addr = self.pc;
        bus.write(((addr >> 4) & 0x0F) as u8);
        ctrl.deassert_sync(0);
    }

    fn phase_a3(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        if self.halted {
            return;
        }
        // Output address bits 8-11, select ROM bank
        let addr = self.pc;
        bus.write(((addr >> 8) & 0x0F) as u8);
        ctrl.select_rom((addr >> 8) as u8 & 0x0F, 0);
    }

    fn phase_m1(&mut self, bus: &mut DataBus) {
        if self.halted {
            return;
        }
        // Read instruction OPA (bits 0-3)
        let opa = bus.read();
        self.instruction_byte = (self.instruction_byte & 0xF0) | (opa & 0x0F);
    }

    fn phase_m2(&mut self, bus: &mut DataBus) {
        if self.halted {
            return;
        }
        // Read instruction OPR (bits 4-7)
        let opr = bus.read();
        self.instruction_byte = (self.instruction_byte & 0x0F) | ((opr & 0x0F) << 4);
    }

    fn phase_x1(&mut self, _bus: &mut DataBus, ctrl: &mut ControlSignals) {
        if self.halted {
            return;
        }
        // Decode the instruction
        if self.cycle.second_cycle {
            self.decoder.decode_second(self.instruction_byte);
        } else {
            self.decoder.decode_first(self.instruction_byte);
        }
        self.pc_modified = false;

        // Phase-accurate I/O control lines: latch the decoded op in X1, but only
        // assert `ctrl.io_op` during the actual transfer phase (X2 write, X3 read).
        self.decoded_io_op = match self.decoder.get_instruction() {
            Some(Instruction::Src { .. }) => Some(IoOp::Src),
            Some(Instruction::Wrm) => Some(IoOp::RamMainWrite),
            Some(Instruction::Rdm | Instruction::Adm | Instruction::Sbm) => Some(IoOp::RamMainRead),
            Some(Instruction::Wmp) => Some(IoOp::RamPortWrite),
            Some(Instruction::Wrr) => Some(IoOp::RomPortWrite),
            Some(Instruction::Rdr) => Some(IoOp::RomPortRead),
            Some(Instruction::Wr0) => Some(IoOp::RamStatusWrite(0)),
            Some(Instruction::Wr1) => Some(IoOp::RamStatusWrite(1)),
            Some(Instruction::Wr2) => Some(IoOp::RamStatusWrite(2)),
            Some(Instruction::Wr3) => Some(IoOp::RamStatusWrite(3)),
            Some(Instruction::Rd0) => Some(IoOp::RamStatusRead(0)),
            Some(Instruction::Rd1) => Some(IoOp::RamStatusRead(1)),
            Some(Instruction::Rd2) => Some(IoOp::RamStatusRead(2)),
            Some(Instruction::Rd3) => Some(IoOp::RamStatusRead(3)),
            _ => None,
        };
        ctrl.clear_io_op();
    }

    fn phase_x2(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        if self.halted {
            return;
        }

        // Assert write-oriented I/O op only during X2 (SRC spans X2+X3).
        ctrl.clear_io_op();
        if let Some(op) = self.decoded_io_op {
            if matches!(op, IoOp::Src | IoOp::RamMainWrite | IoOp::RamPortWrite | IoOp::RomPortWrite | IoOp::RamStatusWrite(_))
            {
                ctrl.set_io_op(op);
            }
        }
        // Execute instruction (for single-cycle instructions).
        // Read-oriented ops are executed in X3 after peripherals drive the bus.
        if let Some(instr) = self.decoder.get_instruction() {
            let is_read = matches!(
                instr,
                Instruction::Rdm
                    | Instruction::Rdr
                    | Instruction::Rd0
                    | Instruction::Rd1
                    | Instruction::Rd2
                    | Instruction::Rd3
                    | Instruction::Adm
                    | Instruction::Sbm
            );
            if !is_read {
                self.execute(instr, bus);
            }
        }

        // SRC bus behavior (best-effort):
        // X2 outputs the chip+register nibble (chip in bits 0-1, reg in bits 2-3).
        if ctrl.io_op == Some(IoOp::Src) {
            bus.write(self.ram_chip & 0x0F);
        }
    }

    fn phase_x3(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        if self.halted {
            return;
        }

        // Assert read-oriented I/O op only during X3 (SRC spans X2+X3).
        ctrl.clear_io_op();
        if let Some(op) = self.decoded_io_op {
            if matches!(op, IoOp::Src | IoOp::RamMainRead | IoOp::RomPortRead | IoOp::RamStatusRead(_)) {
                ctrl.set_io_op(op);
            }
        }

        // SRC bus behavior (best-effort):
        // X3 outputs the character nibble to complete SRC latching in 4002.
        if ctrl.io_op == Some(IoOp::Src) {
            bus.write(self.ram_address & 0x0F);
        }

        // Execute read-oriented instructions after peripherals have driven the bus in X3.
        if let Some(instr) = self.decoder.get_instruction() {
            let is_read = matches!(
                instr,
                Instruction::Rdm
                    | Instruction::Rdr
                    | Instruction::Rd0
                    | Instruction::Rd1
                    | Instruction::Rd2
                    | Instruction::Rd3
                    | Instruction::Adm
                    | Instruction::Sbm
            );
            if is_read {
                self.execute(instr, bus);
            }
        }

        // Two-byte instruction: schedule operand fetch for next machine cycle.
        if self.decoder.needs_second_byte() && !self.cycle.second_cycle {
            self.cycle.set_two_cycle();
            self.pc = (self.pc + 1) & 0xFFF;
            return;
        }

        // Instruction complete: default is to advance PC by one byte unless the instruction
        // explicitly changed PC (taken branch/jump/call/return).
        if self.decoder.get_instruction().is_some() && !self.pc_modified {
            self.pc = (self.pc + 1) & 0xFFF;
        }
    }

    fn execute(&mut self, instr: Instruction, bus: &mut DataBus) {
        tracing::debug!(?instr, pc=%self.pc(), acc=%self.alu.accumulator(), "Execute 4040");
        match instr {
            // 4040 Specifics
            Instruction::Hlt => self.halted = true,
            Instruction::Bbs => {
                // Restore SRC from interrupt save (simplified)
                let saved_src = self.intr.bbs_restore();
                self.ram_address = saved_src & 0x0F;
                self.ram_chip = (saved_src >> 4) & 0x0F;

                // Pop stack
                if let Ok(ret_addr) = self.stack.pop() {
                    self.pc = ret_addr;
                    self.pc_modified = true;
                }
            }
            Instruction::Lcr => {
                // Load Command Register (select RAM bank for commands)
                // Behavioral model: usually toggles external lines
            }
            Instruction::Or4 => {
                let val = self.registers.get(4);
                let acc = self.alu.accumulator();
                self.alu.set_accumulator(acc | val);
            }
            Instruction::Or5 => {
                let val = self.registers.get(5);
                let acc = self.alu.accumulator();
                self.alu.set_accumulator(acc | val);
            }
            Instruction::An6 => {
                let val = self.registers.get(6);
                let acc = self.alu.accumulator();
                self.alu.set_accumulator(acc & val);
            }
            Instruction::An7 => {
                let val = self.registers.get(7);
                let acc = self.alu.accumulator();
                self.alu.set_accumulator(acc & val);
            }
            Instruction::Db0 => self.registers.db0(),
            Instruction::Db1 => self.registers.db1(),
            Instruction::Sb0 => {
                self.ram_bank = 0;
            }
            Instruction::Sb1 => {
                self.ram_bank = 1;
            }
            Instruction::Ein => self.intr.ein(),
            Instruction::Din => self.intr.din(),
            Instruction::Rpm => {
                // Read Program Memory
                // ROM byte at P0 -> ACC
                // For now, use io_data as a proxy for the ROM byte fetched by bus
                self.alu.set_accumulator(self.io_data & 0x0F);
            }

            // 4004 Standard (Delegated mainly to ALU/Registers)
            Instruction::Nop => {}
            Instruction::Ldm { data } => self.alu.load(data),
            Instruction::Fim { pair, data } => self.registers.set_pair(pair as usize, (data >> 4) & 0xF, data & 0xF),
            Instruction::Src { pair } => {
                let (hi, lo) = self.registers.get_pair(pair as usize);
                self.ram_address = lo;
                self.ram_chip = hi;
            }
            Instruction::Jun { addr_high, addr_low } => {
                self.pc = ((addr_high as u16) << 8) | (addr_low as u16);
                self.pc_modified = true;
            }
            Instruction::Jms { addr_high, addr_low } => {
                let return_addr = (self.pc + 1) & 0x0FFF;
                if self.stack.push(return_addr).is_ok() {
                    self.pc = ((addr_high as u16) << 8) | (addr_low as u16);
                    self.pc_modified = true;
                }
            }
            Instruction::Bbl { data } => {
                if let Ok(ret) = self.stack.pop() {
                    self.pc = ret;
                    self.pc_modified = true;
                }
                self.alu.load(data);
            }
            Instruction::Inc { reg } => {
                let val = self.registers.get(reg as usize);
                self.registers.set(reg as usize, val.wrapping_add(1));
            }
            Instruction::Isz { reg, addr_low } => {
                let val = self.registers.get(reg as usize).wrapping_add(1) & 0xF;
                self.registers.set(reg as usize, val);
                if val != 0 {
                    self.pc = (self.pc & 0xF00) | (addr_low as u16);
                    self.pc_modified = true;
                }
            }
            Instruction::Add { reg } => self.alu.add(self.registers.get(reg as usize)),
            Instruction::Sub { reg } => self.alu.sub(self.registers.get(reg as usize)),
            Instruction::Ld { reg } => self.alu.load(self.registers.get(reg as usize)),
            Instruction::Xch { reg } => {
                let r = self.registers.get(reg as usize);
                let a = self.alu.accumulator();
                self.registers.set(reg as usize, a);
                self.alu.set_accumulator(r);
            }
            Instruction::Clb => self.alu.clb(),
            Instruction::Clc => self.alu.set_carry(false),
            Instruction::Iac => self.alu.iac(),
            Instruction::Cmc => self.alu.cmc(),
            Instruction::Cma => self.alu.cma(),
            Instruction::Ral => self.alu.ral(),
            Instruction::Rar => self.alu.rar(),
            Instruction::Tcc => self.alu.tcc(),
            Instruction::Dac => self.alu.dac(),
            Instruction::Tcs => {
                let val = if self.alu.carry() { 10 } else { 9 };
                self.alu.set_accumulator(val);
                self.alu.set_carry(false);
            }
            Instruction::Stc => self.alu.stc(),
            Instruction::Daa => self.alu.daa(),
            Instruction::Kbp => self.alu.kbp(),
            Instruction::Dcl => {
                // Designate command line: selects CM-RAM lines for subsequent RAM operations.
                self.ram_bank = self.alu.accumulator() & 0x0F;
            }

            // I/O
            Instruction::Wrm => bus.write(self.alu.accumulator()),
            Instruction::Wmp => bus.write(self.alu.accumulator()),
            Instruction::Wrr => bus.write(self.alu.accumulator()),
            Instruction::Wpm => bus.write(self.alu.accumulator()),
            Instruction::Wr0 => bus.write(self.alu.accumulator()),
            Instruction::Wr1 => bus.write(self.alu.accumulator()),
            Instruction::Wr2 => bus.write(self.alu.accumulator()),
            Instruction::Wr3 => bus.write(self.alu.accumulator()),
            Instruction::Sbm => self.alu.sub(bus.read()),
            Instruction::Rdm => self.alu.load(bus.read()),
            Instruction::Rdr => self.alu.load(bus.read()),
            Instruction::Adm => self.alu.add(bus.read()),
            Instruction::Rd0 | Instruction::Rd1 | Instruction::Rd2 | Instruction::Rd3 => {
                self.alu.load(bus.read());
            }

            // JCN
            Instruction::Jcn { condition, addr_low } => {
                let invert = (condition & 0x08) != 0;
                let test_zero = (condition & 0x04) != 0;
                let test_carry = (condition & 0x02) != 0;
                let test_test = (condition & 0x01) != 0;

                let mut cond = false;
                if test_zero && self.alu.accumulator() == 0 {
                    cond = true;
                }
                if test_carry && self.alu.carry() {
                    cond = true;
                }
                if test_test && self.test_pin {
                    cond = true;
                }

                if invert {
                    cond = !cond;
                }

                if cond {
                    self.pc = (self.pc & 0xF00) | (addr_low as u16);
                    self.pc_modified = true;
                }
            }

            _ => {}
        }
    }

    fn current_src(&self) -> u8 {
        (self.ram_chip << 4) | self.ram_address
    }
}

impl Default for I4040 {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Chip for I4040 {
    fn name(&self) -> &'static str {
        "4040"
    }
    fn reset(&mut self) {
        *self = Self::new();
    }
    fn tick(&mut self, phase: BusCycle) {
        // This Chip trait method signature doesn't pass bus/ctrl
        // So we can't fully implement it here without changing the trait
        // or storing bus refs (not safe in Rust).
        // For now, the System loop calls the specific `tick` method above.
        let _ = phase;
    }
}
