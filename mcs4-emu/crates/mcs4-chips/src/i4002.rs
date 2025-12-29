//! Intel 4002 RAM + Output (stub)

use mcs4_bus::BusCycle;

/// Intel 4002: 320-bit RAM with 4-bit output port
#[derive(Clone, Debug)]
pub struct I4002 {
    /// RAM: 4 registers x 16 nibbles + 4 status nibbles
    ram: [[u8; 16]; 4],
    status: [u8; 4],
    output: u8,
    chip_select: u8,
}

impl I4002 {
    pub fn new(chip_select: u8) -> Self {
        Self {
            ram: [[0; 16]; 4],
            status: [0; 4],
            output: 0,
            chip_select,
        }
    }

    pub fn read(&self, reg: u8, index: u8) -> u8 {
        self.ram[(reg & 3) as usize][(index & 0x0F) as usize] & 0x0F
    }

    pub fn write(&mut self, reg: u8, index: u8, value: u8) {
        self.ram[(reg & 3) as usize][(index & 0x0F) as usize] = value & 0x0F;
    }
}

impl super::Chip for I4002 {
    fn name(&self) -> &'static str { "4002" }
    fn reset(&mut self) { self.ram = [[0; 16]; 4]; self.status = [0; 4]; self.output = 0; }
    fn tick(&mut self, _phase: BusCycle) {}
}
