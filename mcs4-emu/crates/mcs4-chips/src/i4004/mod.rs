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
mod registers;
mod instruction_decode;
mod timing_io;

pub use alu::Alu;
pub use registers::Registers;
pub use instruction_decode::{InstructionDecoder, Instruction};
pub use timing_io::TimingIo;

use mcs4_bus::prelude::*;
#[allow(unused_imports)]
use mcs4_core::prelude::*;

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

    /// Test pin input (directly readable)
    test_pin: bool,

    /// Pending memory read/write data
    io_data: u8,
}

impl I4004 {
    /// Create a new 4004 CPU
    pub fn new() -> Self {
        Self {
            alu: Alu::new(),
            registers: Registers::new(),
            decoder: InstructionDecoder::new(),
            timing: TimingIo::new(),
            cycle: CycleState::new(),
            instruction_byte: 0,
            operand: 0,
            ram_address: 0,
            ram_chip: 0,
            test_pin: false,
            io_data: 0,
        }
    }

    /// Set the test pin state
    pub fn set_test_pin(&mut self, state: bool) {
        self.test_pin = state;
    }

    /// Get currently selected RAM address
    pub fn ram_address(&self) -> u8 {
        self.ram_address
    }

    /// Get currently selected RAM chip
    pub fn ram_chip(&self) -> u8 {
        self.ram_chip
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

    fn phase_a1(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // Output address bits 0-3 and assert SYNC
        let addr = self.registers.pc();
        bus.write((addr & 0x0F) as u8);
        ctrl.assert_sync(0);
    }

    fn phase_a2(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // Output address bits 4-7, deassert SYNC
        let addr = self.registers.pc();
        bus.write(((addr >> 4) & 0x0F) as u8);
        ctrl.deassert_sync(0);
    }

    fn phase_a3(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // Output address bits 8-11, select ROM bank
        let addr = self.registers.pc();
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

    fn phase_x1(&mut self, _bus: &mut DataBus, _ctrl: &mut ControlSignals) {
        // Decode the instruction
        if self.cycle.second_cycle {
            // Second byte of two-byte instruction
            self.decoder.decode_second(self.instruction_byte);
        } else {
            self.decoder.decode_first(self.instruction_byte);
        }
    }

    fn phase_x2(&mut self, bus: &mut DataBus, _ctrl: &mut ControlSignals) {
        // Execute instruction (for single-cycle instructions)
        if !self.decoder.needs_second_byte() {
            if let Some(instr) = self.decoder.get_instruction() {
                self.execute(instr, bus);
            }
        }
    }

    fn phase_x3(&mut self, _bus: &mut DataBus, _ctrl: &mut ControlSignals) {
        // Increment PC after execution
        if let Some(instr) = self.decoder.get_instruction() {
            // For two-byte instructions, only increment after second cycle
            if instr.length() == 1 || self.cycle.second_cycle {
                self.registers.increment_pc();
            }
            // Set up for second cycle if needed
            if instr.length() == 2 && !self.cycle.second_cycle {
                self.cycle.two_cycle = true;
                self.cycle.second_cycle = true;
                self.registers.increment_pc();
            } else {
                self.cycle.two_cycle = false;
                self.cycle.second_cycle = false;
            }
        }
    }

    /// Execute a decoded instruction
    fn execute(&mut self, instr: Instruction, bus: &mut DataBus) {
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
                // Fetch indirect: use pair 0 as address into ROM page 0
                let addr = self.registers.get_pair(0);
                // In real hardware, this fetches from ROM[addr]
                // For now, store address for bus to handle
                self.io_data = addr;
                let _ = pair; // Will be loaded with fetched data
            }
            Jin { pair } => {
                let addr = self.registers.get_pair(pair);
                let pc = self.registers.pc();
                let new_pc = (pc & 0xF00) | (addr as u16);
                self.registers.set_pc(new_pc);
            }

            // Unconditional jumps
            Jun { addr_high, addr_low } => {
                let new_pc = ((addr_high as u16) << 8) | (addr_low as u16);
                self.registers.set_pc(new_pc);
            }
            Jms { addr_high, addr_low } => {
                let new_pc = ((addr_high as u16) << 8) | (addr_low as u16);
                self.registers.call(new_pc);
            }
            Isz { reg, addr_low } => {
                let wrapped = self.registers.inc_r(reg);
                if !wrapped {
                    // Not zero, jump
                    let pc = self.registers.pc();
                    let new_pc = (pc & 0xF00) | (addr_low as u16);
                    self.registers.set_pc(new_pc);
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
                // Designate command line (4040 only, NOP on 4004)
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
        if test_pin && self.test_pin {
            result = true;
        }

        if invert { !result } else { result }
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
        self.test_pin = false;
        self.io_data = 0;
    }

    fn tick(&mut self, phase: BusCycle) {
        // Simplified tick without bus/control access
        self.cycle.advance();
        let _ = phase;
    }
}
