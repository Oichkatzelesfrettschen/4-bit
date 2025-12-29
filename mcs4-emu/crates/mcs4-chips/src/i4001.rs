//! Intel 4001 ROM + I/O (stub)

use mcs4_bus::BusCycle;

/// Intel 4001: 256x8 ROM with 4-bit I/O port
#[derive(Clone, Debug)]
pub struct I4001 {
    /// ROM contents (256 bytes)
    rom: [u8; 256],
    /// I/O port value
    io_port: u8,
    /// Chip select (0-15)
    chip_select: u8,
}

impl I4001 {
    pub fn new(chip_select: u8) -> Self {
        Self {
            rom: [0; 256],
            io_port: 0,
            chip_select,
        }
    }

    /// Load ROM contents
    pub fn load(&mut self, data: &[u8]) {
        let len = data.len().min(256);
        self.rom[..len].copy_from_slice(&data[..len]);
    }

    /// Read ROM at address
    pub fn read(&self, addr: u8) -> u8 {
        self.rom[addr as usize]
    }
}

impl super::Chip for I4001 {
    fn name(&self) -> &'static str { "4001" }
    fn reset(&mut self) { self.io_port = 0; }
    fn tick(&mut self, _phase: BusCycle) {}
}
