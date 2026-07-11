//! Intel 4004 CPU Implementation
//!
//! The 4004 is the world's first commercial single-chip microprocessor.
//! This module provides a gate-level accurate implementation.
//!
//! ## Architecture
//! - 4-bit data path
//! - 12-bit address space (4KB ROM)
//! - 46 instructions
//! - 16 4-bit index registers (8 pairs)
//! - 3-level stack for subroutine calls
//! - 4-bit accumulator with carry flag

mod alu;
pub mod instruction_decode;
mod registers;
pub mod solver_bridge;
mod timing_io;

pub use alu::Alu;
pub use instruction_decode::{Instruction, InstructionDecoder};
use mcs4_bus::prelude::*;
use mcs4_core::SimulationFidelity;
pub use registers::Registers;
pub use timing_io::TimingIo;

/// CM-RAM line mask selecting DATA RAM bank 0 (the RESET default).
pub(crate) const CM_RAM0: u8 = 0b0001;

/// Decode the DCL bank selection from accumulator bits 2:0 into a CM-RAM
/// line mask: 000 asserts CM-RAM0; any other value shifts onto CM-RAM1..3
/// (001 -> CM-RAM1, 010 -> CM-RAM2, 100 -> CM-RAM3, combinations assert
/// multiple lines), addressing up to 8 DATA RAM banks.
pub(crate) fn decode_cm_ram_lines(acc: u8) -> u8 {
    let bank = acc & 0x07;
    if bank == 0 {
        CM_RAM0
    } else {
        bank << 1
    }
}

/// Intel 4004 CPU
pub struct I4004 {
    /// ALU (Arithmetic Logic Unit)
    pub alu: Alu,

    /// Register file
    pub registers: Registers,

    /// Instruction decoder
    pub decoder: InstructionDecoder,

    /// Timing and I/O control
    pub timing: TimingIo,

    /// Simulation fidelity level
    fidelity: SimulationFidelity,

    /// Current cycle state
    cycle: CycleState,

    /// Fetched instruction (OPR:OPA)
    instruction_byte: u8,

    /// Second byte of two-byte instruction
    operand: u8,

    /// Currently selected RAM address (from SRC)
    ram_address: u8,

    /// Currently selected RAM chip
    ram_chip: u8,

    /// CM-RAM line selection mask (set by DCL; reset selects CM-RAM0)
    ram_bank: u8,

    /// Test pin input (directly readable)
    test_pin: bool,

    /// Decoded high-level I/O operation for the current instruction (used for phase-accurate control lines).
    decoded_io_op: Option<IoOp>,

    /// True if an instruction explicitly updated PC (taken branches, jumps, calls, returns).
    pc_modified: bool,

    /// Register pair awaiting the FIN indirect ROM fetch during the second machine cycle.
    fin_pair: Option<u8>,
}

impl I4004 {
    /// Create a new 4004 CPU
    pub fn new() -> Self {
        Self {
            alu: Alu::new(),
            registers: Registers::new(),
            decoder: InstructionDecoder::new(),
            timing: TimingIo::new(),
            fidelity: SimulationFidelity::Behavioral,
            cycle: CycleState::new(),
            instruction_byte: 0,
            operand: 0,
            ram_address: 0,
            ram_chip: 0,
            ram_bank: CM_RAM0,
            test_pin: false,
            decoded_io_op: None,
            pc_modified: false,
            fin_pair: None,
        }
    }

    /// Set the test pin state
    pub fn set_test_pin(&mut self, state: bool) {
        self.test_pin = state;
    }

    /// Get the current test pin state
    pub fn test_pin(&self) -> bool {
        self.test_pin
    }

    /// Get currently selected RAM address
    pub fn ram_address(&self) -> u8 {
        self.ram_address
    }

    /// Get currently selected RAM chip
    pub fn ram_chip(&self) -> u8 {
        self.ram_chip
    }

    /// Get the CM-RAM line selection mask (bit i = CM-RAMi asserted).
    ///
    /// DCL decodes accumulator bits 2:0 into this mask; RESET selects CM-RAM0.
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

    /// Get program counter
    pub fn pc(&self) -> u16 {
        self.registers.pc()
    }

    /// Get accumulator value
    pub fn accumulator(&self) -> u8 {
        self.alu.accumulator()
    }

    /// Get carry flag
    pub fn carry(&self) -> bool {
        self.alu.carry()
    }

    /// Process one bus phase
    pub fn tick(&mut self, phase: BusCycle, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        tracing::trace!(?phase, pc=%self.registers.pc(), "CPU Tick");
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

    /// Address emitted during A1-A3.
    ///
    /// The FIN second cycle sends register pair 0 as the low 8 bits with the
    /// page taken from the program counter. The PC has already advanced past
    /// the FIN byte, so a FIN in the last location of a page fetches from the
    /// next page, matching the documented boundary behavior.
    fn fetch_address(&self) -> u16 {
        let pc = self.registers.pc();
        if self.fin_pair.is_some() && self.cycle.second_cycle {
            (pc & 0xF00) | u16::from(self.registers.get_pair(0))
        } else {
            pc
        }
    }

    fn phase_a1(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        ctrl.clear_io_op();
        ctrl.deselect_ram(0);
        // Output address bits 0-3 and assert SYNC
        let addr = self.fetch_address();
        bus.write((addr & 0x0F) as u8);
        ctrl.assert_sync(0);
    }

    fn phase_a2(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // Output address bits 4-7, deassert SYNC
        let addr = self.fetch_address();
        bus.write(((addr >> 4) & 0x0F) as u8);
        ctrl.deassert_sync(0);
    }

    fn phase_a3(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // Output address bits 8-11, select ROM bank
        let addr = self.fetch_address();
        bus.write(((addr >> 8) & 0x0F) as u8);
        ctrl.select_rom((addr >> 8) as u8 & 0x0F, 0);
    }

    fn phase_m1(&mut self, bus: &mut DataBus) {
        // Read instruction OPA (bits 0-3)
        let opa = bus.read();
        self.instruction_byte = (self.instruction_byte & 0xF0) | (opa & 0x0F);
    }

    fn phase_m2(&mut self, bus: &mut DataBus) {
        // Read instruction OPR (bits 4-7)
        let opr = bus.read();
        self.instruction_byte = (self.instruction_byte & 0x0F) | ((opr & 0x0F) << 4);
    }

    fn phase_x1(&mut self, _bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // FIN second cycle: M1/M2 latched the byte at the indirect address;
        // load it into the target register pair. The PC already points at the
        // next instruction, so the cycle completes without decode or PC change.
        if self.cycle.second_cycle {
            if let Some(pair) = self.fin_pair.take() {
                self.registers.set_pair(pair, self.instruction_byte);
                self.decoder.instruction = None;
                self.decoded_io_op = None;
                ctrl.clear_io_op();
                return;
            }
        }

        // Decode the instruction
        if self.cycle.second_cycle {
            // Second byte of two-byte instruction
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
        // Assert write-oriented I/O op only during X2 (SRC spans X2+X3).
        ctrl.clear_io_op();
        if let Some(op) = self.decoded_io_op {
            if matches!(
                op,
                IoOp::Src | IoOp::RamMainWrite | IoOp::RamPortWrite | IoOp::RomPortWrite | IoOp::RamStatusWrite(_)
            ) {
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
        // Assert read-oriented I/O op only during X3 (SRC spans X2+X3).
        ctrl.clear_io_op();
        if let Some(op) = self.decoded_io_op {
            if matches!(
                op,
                IoOp::Src | IoOp::RamMainRead | IoOp::RomPortRead | IoOp::RamStatusRead(_)
            ) {
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
                // SAFETY: system orders X3 as peripherals first, then CPU tick.
                // This call reads from bus and updates ALU accordingly.
                self.execute(instr, bus);
            }
        }

        // Two-byte instruction: schedule operand fetch for next machine cycle.
        if self.decoder.needs_second_byte() && !self.cycle.second_cycle {
            self.cycle.set_two_cycle();
            self.registers.increment_pc();
            return;
        }

        // FIN: schedule the indirect ROM fetch as a second machine cycle.
        // Advancing the PC now gives the second cycle's A phases the page of
        // the byte after the FIN (the documented page-boundary behavior).
        if self.fin_pair.is_some() && !self.cycle.second_cycle {
            self.cycle.set_two_cycle();
            self.registers.increment_pc();
            return;
        }

        // Instruction complete: default is to advance PC by one byte unless the instruction
        // explicitly changed PC (taken branch/jump/call/return).
        if self.decoder.get_instruction().is_some() && !self.pc_modified {
            self.registers.increment_pc();
        }
    }

    /// Execute a decoded instruction
    fn execute(&mut self, instr: Instruction, bus: &mut DataBus) {
        tracing::debug!(?instr, pc=%self.registers.pc(), acc=%self.alu.accumulator(), "Execute");
        use Instruction::*;
        match instr {
            // Machine control
            Nop => {}

            // Conditional jumps
            Jcn { condition, addr_low } => {
                let jump = self.evaluate_condition(condition);
                if jump {
                    let pc = self.registers.pc();
                    let new_pc = (pc & 0xF00) | (addr_low as u16);
                    self.registers.set_pc(new_pc);
                    self.pc_modified = true;
                }
            }

            // Register pair operations
            Fim { pair, data } => {
                self.registers.set_pair(pair, data);
            }
            Src { pair } => {
                let addr = self.registers.get_pair(pair);
                self.ram_address = addr & 0x0F;
                self.ram_chip = (addr >> 4) & 0x0F;
            }
            Fin { pair } => {
                // Fetch indirect: arm the second machine cycle, which sends
                // register pair 0 as the ROM address and loads the fetched
                // byte into the target pair.
                self.fin_pair = Some(pair);
            }
            Jin { pair } => {
                let addr = self.registers.get_pair(pair);
                let pc = self.registers.pc();
                let new_pc = (pc & 0xF00) | (addr as u16);
                self.registers.set_pc(new_pc);
                self.pc_modified = true;
            }

            // Unconditional jumps
            Jun { addr_high, addr_low } => {
                let new_pc = ((addr_high as u16) << 8) | (addr_low as u16);
                self.registers.set_pc(new_pc);
                self.pc_modified = true;
            }
            Jms { addr_high, addr_low } => {
                let new_pc = ((addr_high as u16) << 8) | (addr_low as u16);
                let return_addr = (self.registers.pc() + 1) & 0x0FFF;
                self.registers.call(return_addr, new_pc);
                self.pc_modified = true;
            }
            Isz { reg, addr_low } => {
                let wrapped = self.registers.inc_r(reg);
                if !wrapped {
                    // Not zero, jump
                    let pc = self.registers.pc();
                    let new_pc = (pc & 0xF00) | (addr_low as u16);
                    self.registers.set_pc(new_pc);
                    self.pc_modified = true;
                }
            }

            // Index register operations
            Inc { reg } => {
                self.registers.inc_r(reg);
            }
            Add { reg } => {
                let value = self.registers.get_r(reg);
                self.alu.add(value);
            }
            Sub { reg } => {
                let value = self.registers.get_r(reg);
                self.alu.sub(value);
            }
            Ld { reg } => {
                let value = self.registers.get_r(reg);
                self.alu.load(value);
            }
            Xch { reg } => {
                let reg_val = self.registers.get_r(reg);
                let old_acc = self.alu.xch(reg_val);
                self.registers.set_r(reg, old_acc);
            }
            Bbl { data } => {
                self.registers.ret();
                self.alu.load(data);
                self.pc_modified = true;
            }

            // Immediate operations
            Ldm { data } => {
                self.alu.load(data);
            }

            // I/O and RAM control - these interact with the bus
            Wrm => {
                bus.write(self.alu.accumulator());
            }
            Wmp | Wrr | Wpm => {
                bus.write(self.alu.accumulator());
            }
            Wr0 | Wr1 | Wr2 | Wr3 => {
                bus.write(self.alu.accumulator());
            }
            Sbm => {
                let value = bus.read();
                self.alu.sub(value);
            }
            Rdm => {
                let value = bus.read();
                self.alu.load(value);
            }
            Rdr => {
                let value = bus.read();
                self.alu.load(value);
            }
            Adm => {
                let value = bus.read();
                self.alu.add(value);
            }
            Rd0 | Rd1 | Rd2 | Rd3 => {
                let value = bus.read();
                self.alu.load(value);
            }

            // Accumulator group
            Clb => self.alu.clb(),
            Clc => self.alu.set_carry(false),
            Iac => self.alu.iac(),
            Cmc => self.alu.cmc(),
            Cma => self.alu.cma(),
            Ral => self.alu.ral(),
            Rar => self.alu.rar(),
            Tcc => self.alu.tcc(),
            Dac => self.alu.dac(),
            Tcs => {
                // Transfer carry subtract: ACC = 9 + CY
                let value = if self.alu.carry() { 10 } else { 9 };
                self.alu.set_accumulator(value);
                self.alu.set_carry(false);
            }
            Stc => self.alu.stc(),
            Daa => self.alu.daa(),
            Kbp => self.alu.kbp(),
            Dcl => {
                // Designate command line: decode accumulator bits 2:0 into the
                // CM-RAM line mask that selects the DATA RAM bank.
                self.ram_bank = decode_cm_ram_lines(self.alu.accumulator());
            }

            Invalid { opcode: _ } => {
                // Invalid instruction - no operation
            }
        }
    }

    /// Evaluate JCN condition
    fn evaluate_condition(&self, condition: u8) -> bool {
        let invert = (condition & 0x08) != 0;
        let test_acc_zero = (condition & 0x04) != 0;
        let test_carry = (condition & 0x02) != 0;
        let test_pin = (condition & 0x01) != 0;

        let mut result = false;

        if test_acc_zero && self.alu.accumulator() == 0 {
            result = true;
        }
        if test_carry && self.alu.carry() {
            result = true;
        }
        // The TEST condition is satisfied when the TEST pin is at 0.
        if test_pin && !self.test_pin {
            result = true;
        }

        if invert {
            !result
        } else {
            result
        }
    }
}

impl Default for I4004 {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Chip for I4004 {
    fn name(&self) -> &'static str {
        "4004"
    }

    fn reset(&mut self) {
        self.alu = Alu::new();
        self.registers = Registers::new();
        self.decoder = InstructionDecoder::new();
        self.cycle = CycleState::new();
        self.instruction_byte = 0;
        self.operand = 0;
        self.ram_address = 0;
        self.ram_chip = 0;
        self.ram_bank = CM_RAM0;
        self.test_pin = false;
        self.decoded_io_op = None;
        self.pc_modified = false;
        self.fin_pair = None;
    }

    fn tick(&mut self, phase: BusCycle) {
        // Simplified tick without bus/control access
        self.cycle.advance();
        let _ = phase;
    }
}
