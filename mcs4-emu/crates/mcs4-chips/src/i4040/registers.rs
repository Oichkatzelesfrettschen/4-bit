// 4040 register file with bank switching
#[derive(Default)]
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

#[cfg(test)]
mod tests {
    use super::RegFile;

    #[test]
    fn get_set_low_nibble_masking() {
        let mut rf = RegFile::new();
        rf.set(0, 0x1F);
        assert_eq!(rf.get(0), 0x0F);
    }

    #[test]
    fn bank_switch_maps_r0_r7() {
        let mut rf = RegFile::new();
        // Bank 0: R0 maps to index 0
        rf.set(0, 0x3);
        assert_eq!(rf.get(0), 0x3);
        // Switch to bank 1: R0 maps to index 16
        rf.db1();
        assert_eq!(rf.get(0), 0x0); // default value at index 16
        rf.set(0, 0x7);
        assert_eq!(rf.get(0), 0x7);
        // Back to bank 0: original value preserved at index 0
        rf.db0();
        assert_eq!(rf.get(0), 0x3);
    }

    #[test]
    fn r8_r15_unaffected_by_bank() {
        let mut rf = RegFile::new();
        rf.set(8, 0x9);
        rf.db1();
        assert_eq!(rf.get(8), 0x9);
        rf.db0();
        assert_eq!(rf.get(8), 0x9);
    }

    #[test]
    fn pair_helpers_work() {
        let mut rf = RegFile::new();
        rf.set_pair(0, 0xA, 0x5);
        assert_eq!(rf.get_pair(0), (0xA, 0x5));
        rf.db1();
        // Bank switch should change where P0 points; values are distinct per bank
        assert_eq!(rf.get_pair(0), (0x0, 0x0));
        rf.set_pair(0, 0x1, 0x2);
        assert_eq!(rf.get_pair(0), (0x1, 0x2));
        rf.db0();
        assert_eq!(rf.get_pair(0), (0xA, 0x5));
    }
}
