//! Intel 4040 CPU Implementation
//!
//! The 4040 is a backward-compatible enhancement of the 4004 with:
//! - 24 index registers (vs 16) with bank switching
//! - 7-level stack (vs 3)
//! - 14 new instructions (60 total)
//! - Single-level interrupt support
//! - Halt mode

pub mod registers;
pub mod interrupt;
pub mod instruction_decode;

pub use registers::Registers;
pub use interrupt::InterruptController;
pub use instruction_decode::{Instruction, I4040Instruction, decode_4040_specific};

// Note: 4040 uses 4004's ALU (no extensions needed)

use mcs4_bus::prelude::*;
use crate::i4004;

/// Intel 4040 CPU (stub implementation)
///
/// Full implementation deferred - this provides type compatibility
pub struct I4040 {
    /// ALU (from 4004 base)
    pub alu: i4004::Alu,
    /// 4040 registers
    pub registers: Registers,
    /// 4004 registers (temporary for base compatibility)
    pub registers_4004: i4004::Registers,
    /// Interrupt controller
    pub intr: InterruptController,
    /// Instruction decoder
    pub decoder: i4004::InstructionDecoder,
    /// Halted state
    halted: bool,
}

impl I4040 {
    pub fn new() -> Self {
        Self {
            alu: i4004::Alu::new(),
            registers: Registers::new(),
            registers_4004: i4004::Registers::new(),
            intr: InterruptController::new(),
            decoder: i4004::InstructionDecoder::new(),
            halted: false,
        }
    }

    pub fn pc(&self) -> u16 {
        self.registers.pc()
    }

    pub fn accumulator(&self) -> u8 {
        self.alu.accumulator()
    }

    pub fn carry(&self) -> bool {
        self.alu.carry()
    }

    pub fn halted(&self) -> bool {
        self.halted
    }

    pub fn tick(&mut self, phase: BusCycle, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // Simplified tick for compatibility
        let _ = (phase, bus, ctrl);
    }

    // Compatibility methods from 4004
    pub fn set_test_pin(&mut self, state: bool) {
        let _ = state;
    }

    pub fn ram_address(&self) -> u8 {
        0
    }

    pub fn ram_chip(&self) -> u8 {
        0
    }

    pub fn ram_bank(&self) -> u8 {
        0
    }

    pub fn x3_cpu_drives_first(&self) -> bool {
        false
    }

    pub fn x2_ram_bank_select(&self) -> bool {
        false
    }

    pub fn x3_ram_bank_select(&self) -> bool {
        false
    }

    pub fn x3_peripheral_io_op(&self) -> Option<IoOp> {
        None
    }
}

impl Default for I4040 {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::Chip for I4040 {
    fn name(&self) -> &'static str {
        "4040"
    }

    fn reset(&mut self) {
        self.alu = i4004::Alu::new();
        self.registers = Registers::new();
        self.registers_4004 = i4004::Registers::new();
        self.intr = InterruptController::new();
        self.decoder = i4004::InstructionDecoder::new();
        self.halted = false;
    }

    fn tick(&mut self, phase: BusCycle) {
        // Simplified tick without bus access
        let _ = phase;
    }
}
