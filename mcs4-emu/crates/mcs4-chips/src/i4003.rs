//! Intel 4003 Shift Register (stub)

use mcs4_bus::BusCycle;

/// Intel 4003: 10-bit serial-in, parallel-out shift register
#[derive(Clone, Debug, Default)]
pub struct I4003 {
    data: u16, // 10 bits
}

impl I4003 {
    pub fn new() -> Self { Self::default() }
    pub fn shift_in(&mut self, bit: bool) {
        self.data = ((self.data << 1) | (bit as u16)) & 0x3FF;
    }
    pub fn parallel_out(&self) -> u16 { self.data }
}

impl super::Chip for I4003 {
    fn name(&self) -> &'static str { "4003" }
    fn reset(&mut self) { self.data = 0; }
    fn tick(&mut self, _phase: BusCycle) {}
}
