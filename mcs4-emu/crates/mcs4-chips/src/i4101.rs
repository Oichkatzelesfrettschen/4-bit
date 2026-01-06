//! Intel 4101 256x4 Static RAM
//!
//! The 4101 is a 1024-bit static RAM organized as 256 words by 4 bits.
//! It is used in MCS-40 systems via the 4289 Standard Memory Interface.

use mcs4_bus::BusCycle;

/// Intel 4101: 256x4 Static RAM
#[derive(Clone, Debug)]
pub struct I4101 {
    /// 256 x 4-bit memory (stored in low nibbles of u8)
    memory: [u8; 256],

    /// Address latch (8 bits)
    latched_address: u8,

    /// Chip select signals
    cs: bool,
}

impl I4101 {
    pub fn new() -> Self {
        Self {
            memory: [0; 256],
            latched_address: 0,
            cs: false,
        }
    }

    /// Read data from current address
    pub fn read(&self) -> u8 {
        if self.cs {
            self.memory[self.latched_address as usize] & 0x0F
        } else {
            0x0F // Tri-state/pull-up behavior? Usually floats high on MCS-4 bus
        }
    }

    /// Write data to current address
    pub fn write(&mut self, value: u8) {
        if self.cs {
            self.memory[self.latched_address as usize] = value & 0x0F;
        }
    }

    /// Set current address
    pub fn set_address(&mut self, address: u8) {
        self.latched_address = address;
    }

    /// Set chip select
    pub fn set_cs(&mut self, selected: bool) {
        self.cs = selected;
    }
}

impl Default for I4101 {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Chip for I4101 {
    fn name(&self) -> &'static str {
        "4101"
    }

    fn reset(&mut self) {
        self.memory = [0; 256];
        self.latched_address = 0;
        self.cs = false;
    }

    fn tick(&mut self, _phase: BusCycle) {
        // Behavioral model - timing handled by 4289 or System
    }
}
