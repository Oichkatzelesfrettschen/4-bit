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
pub use instruction_decode::InstructionDecoder;
pub use timing_io::TimingIo;

use mcs4_bus::prelude::*;
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
    instruction: u8,

    /// Second byte of two-byte instruction
    operand: u8,
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
            instruction: 0,
            operand: 0,
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
        // TODO: Drive bus with addr.nibble_a1()
        ctrl.assert_sync(0);
        let _ = bus; // Placeholder
    }

    fn phase_a2(&mut self, _bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // Output address bits 4-7, deassert SYNC
        ctrl.deassert_sync(0);
    }

    fn phase_a3(&mut self, _bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // Output address bits 8-11, select ROM bank
        let addr = self.registers.pc();
        ctrl.select_rom((addr >> 8) as u8 & 0x0F, 0);
    }

    fn phase_m1(&mut self, bus: &mut DataBus) {
        // Read instruction OPA (bits 0-3)
        let opa = bus.read();
        self.instruction = (self.instruction & 0xF0) | (opa & 0x0F);
    }

    fn phase_m2(&mut self, bus: &mut DataBus) {
        // Read instruction OPR (bits 4-7)
        let opr = bus.read();
        self.instruction = (self.instruction & 0x0F) | ((opr & 0x0F) << 4);
    }

    fn phase_x1(&mut self, _bus: &mut DataBus, _ctrl: &mut ControlSignals) {
        // Decode and begin execution
        self.decoder.decode(self.instruction);
    }

    fn phase_x2(&mut self, _bus: &mut DataBus, _ctrl: &mut ControlSignals) {
        // Continue execution
    }

    fn phase_x3(&mut self, _bus: &mut DataBus, _ctrl: &mut ControlSignals) {
        // Complete execution, increment PC
        if !self.cycle.two_cycle || self.cycle.second_cycle {
            self.registers.increment_pc();
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
        self.instruction = 0;
        self.operand = 0;
    }

    fn tick(&mut self, phase: BusCycle) {
        // Simplified tick without bus/control access
        self.cycle.advance();
        let _ = phase;
    }
}
