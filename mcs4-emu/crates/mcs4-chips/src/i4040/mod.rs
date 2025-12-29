//! Intel 4040 CPU Implementation (stub)
//!
//! The 4040 extends the 4004 with:
//! - Interrupt capability
//! - 24 index registers (vs 16)
//! - Halt instruction
//! - Additional instructions

use mcs4_bus::BusCycle;

/// Intel 4040 CPU (stub)
#[derive(Clone, Debug, Default)]
pub struct I4040 {
    // Placeholder - will extend I4004
    _placeholder: u8,
}

impl I4040 {
    pub fn new() -> Self {
        Self::default()
    }
}

impl super::Chip for I4040 {
    fn name(&self) -> &'static str {
        "4040"
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn tick(&mut self, _phase: BusCycle) {
        // TODO: Implement
    }
}
