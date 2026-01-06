//! Intel 4003 10-bit Shift Register
//!
//! The 4003 is a 10-bit serial-in, parallel-out shift register.
//! Used for I/O expansion.

use mcs4_bus::BusCycle;

/// Intel 4003: 10-bit shift register
#[derive(Clone, Debug, Default)]
pub struct I4003 {
    /// Internal 10-bit register
    data: u16,

    /// Serial data input pin
    serial_in: bool,

    /// Clock pin state (prev)
    last_clock: bool,
}

impl I4003 {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set serial data input
    pub fn set_data_in(&mut self, state: bool) {
        self.serial_in = state;
    }

    /// Pulse the clock line
    pub fn set_clock(&mut self, state: bool) {
        if !self.last_clock && state {
            // Rising edge: shift in
            self.data = ((self.data << 1) | (self.serial_in as u16)) & 0x3FF;
        }
        self.last_clock = state;
    }

    /// Get current parallel output
    pub fn parallel_out(&self) -> u16 {
        self.data
    }

    /// Get serial data output (bit 9) for cascading
    pub fn serial_out(&self) -> bool {
        (self.data & 0x200) != 0
    }
}

impl super::Chip for I4003 {
    fn name(&self) -> &'static str {
        "4003"
    }
    fn reset(&mut self) {
        self.data = 0;
        self.last_clock = false;
    }
    fn tick(&mut self, _phase: BusCycle) {
        // Behavioral model: clock is driven by I/O instructions (WMP/WRR)
    }
}
