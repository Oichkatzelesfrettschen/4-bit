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
    #[inline]
    pub fn get(&self, r: usize) -> u8 { self.regs[self.map_index(r)] & 0x0F }
    #[inline]
    pub fn set(&mut self, r: usize, val: u8) { self.regs[self.map_index(r)] = val & 0x0F; }

    // Register-pair helpers (P0..P7 map to (R0,R1)..(R14,R15) under current bank)
    #[inline]
    pub fn get_pair(&self, p: usize) -> (u8, u8) {
        let r = p * 2;
        (self.get(r), self.get(r + 1))
    }
    #[inline]
    pub fn set_pair(&mut self, p: usize, hi: u8, lo: u8) {
        let r = p * 2;
        self.set(r, hi);
        self.set(r + 1, lo);
    }

    // Bank control
    #[inline]
    pub fn db0(&mut self) { self.bank = 0; }
    #[inline]
    pub fn db1(&mut self) { self.bank = 1; }
}
