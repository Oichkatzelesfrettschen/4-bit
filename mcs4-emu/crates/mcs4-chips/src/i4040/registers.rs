// 4040 register file with bank switching
pub struct RegFile {
    regs: [u8; 24], // 4-bit values in low nibble
    pub bank: u8,   // 0 or 1, affects R0-R7 mapping
}

impl RegFile {
    pub fn new() -> Self { Self { regs: [0; 24], bank: 0 } }
    #[inline]
    fn map_index(&self, r: usize) -> usize {
        if r < 8 { r + (self.bank as usize) * 16 } else { r }
    }
    pub fn get(&self, r: usize) -> u8 { self.regs[self.map_index(r)] & 0x0F }
    pub fn set(&mut self, r: usize, val: u8) { self.regs[self.map_index(r)] = val & 0x0F; }
}
