//! Intel 4308 1024x8 ROM with I/O
//!
//! The 4308 contains 8192 bits of mask-programmable ROM organized as 1024 words by 8 bits.
//! It also includes I/O ports.

use mcs4_bus::BusCycle;

/// Intel 4308: 1024x8 ROM
#[derive(Clone, Debug)]
pub struct I4308 {
    /// 1024 x 8-bit memory
    memory: Vec<u8>,

    /// 4-bit I/O ports (often 4 ports of 4 bits)
    ports: [u8; 4],

    /// Chip ID (matches upper bits of address)
    pub chip_id: u8,
}

impl I4308 {
    pub fn new(chip_id: u8) -> Self {
        Self {
            memory: vec![0; 1024],
            ports: [0; 4],
            chip_id,
        }
    }

    pub fn load(&mut self, data: &[u8]) {
        let len = data.len().min(self.memory.len());
        self.memory[..len].copy_from_slice(&data[..len]);
    }

    pub fn read_rom(&self, address: u16) -> u8 {
        self.memory[(address & 0x3FF) as usize]
    }
}

impl super::Chip for I4308 {
    fn name(&self) -> &'static str {
        "4308"
    }

    fn reset(&mut self) {
        // ROM contents preserved
        self.ports = [0; 4];
    }

    fn tick(&mut self, _phase: BusCycle) {
        // TODO: Full bus protocol for 4308
    }
}
