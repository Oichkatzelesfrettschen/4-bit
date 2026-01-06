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

    /// Pending I/O data
    io_data: u8,

    /// Halted state (HLT instruction)
    pub halted: bool,

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
            io_data: 0,
            halted: false,
            pc: 0,
        }
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

    fn phase_a1(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // Interrupt Handling (Start of instruction cycle)
        // If not already in second cycle of a 2-byte instruction...
        if !self.cycle.second_cycle
            && !self.cycle.two_cycle
            && let Some(vector) = self.intr.service(self.current_src())
        {
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

        if self.halted {
            // If halted, we don't fetch/execute, but we might keep the bus idle?
            // Real 4040 performs "dummy" cycles or just stops.
            // For this emulator, we'll just return.
            return;
        }

        // Output address bits 0-3 and assert SYNC
        let addr = self.pc;
        bus.write((addr & 0x0F) as u8);
        ctrl.assert_sync(0); // TODO: pass chip select info?
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

    fn phase_x1(&mut self, _bus: &mut DataBus, _ctrl: &mut ControlSignals) {
        if self.halted {
            return;
        }
        // Decode the instruction
        if self.cycle.second_cycle {
            self.decoder.decode_second(self.instruction_byte);
        } else {
            self.decoder.decode_first(self.instruction_byte);
        }
    }

    fn phase_x2(&mut self, bus: &mut DataBus, _ctrl: &mut ControlSignals) {
        if self.halted {
            return;
        }
        // Execute instruction (for single-cycle instructions)
        if !self.decoder.needs_second_byte()
            && let Some(instr) = self.decoder.get_instruction()
        {
            self.execute(instr, bus);
        }
    }

    fn phase_x3(&mut self, _bus: &mut DataBus, _ctrl: &mut ControlSignals) {
        if self.halted {
            return;
        }
        // Increment PC after execution
        if let Some(instr) = self.decoder.get_instruction() {
            // For two-byte instructions, only increment after second cycle
            if instr.length() == 1 || self.cycle.second_cycle {
                self.pc = (self.pc + 1) & 0xFFF;
            }
            // Set up for second cycle if needed
            if instr.length() == 2 && !self.cycle.second_cycle {
                self.cycle.two_cycle = true;
                self.cycle.second_cycle = true;
                self.pc = (self.pc + 1) & 0xFFF;
            } else {
                self.cycle.two_cycle = false;
                self.cycle.second_cycle = false;
            }
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
                // Select RAM Bank 0
            }
            Instruction::Sb1 => {
                // Select RAM Bank 1
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
            }
            Instruction::Jms { addr_high, addr_low } => {
                if self.stack.push(self.pc).is_ok() {
                    self.pc = ((addr_high as u16) << 8) | (addr_low as u16);
                }
            }
            Instruction::Bbl { data } => {
                if let Ok(ret) = self.stack.pop() {
                    self.pc = ret;
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
                if test_test { /* TODO read test pin */ }

                if invert {
                    cond = !cond;
                }

                if cond {
                    self.pc = (self.pc & 0xF00) | (addr_low as u16);
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
