//! Intel 4702/4702A UV-Erasable PROM (EPROM)
//!
//! The 4702 provides 256 x 8-bit UV-erasable programmable ROM.
//! It uses the same bus protocol as the 4308 but with a smaller
//! address space (256 words vs 1024). Programming requires elevated
//! Vpp voltage.
//!
//! The 4702A is a speed-improved version with the same interface.

use mcs4_bus::BusCycle;

/// Intel 4702: 256x8 UV-Erasable PROM
#[derive(Clone, Debug)]
pub struct I4702 {
    /// 256 x 8-bit memory array
    memory: [u8; 256],

    /// Chip ID for bus addressing
    pub chip_id: u8,

    /// Programming mode active (Vpp applied)
    programming_mode: bool,

    /// Programming address latch
    program_address: u8,

    /// I/O ports (4 x 4-bit, like 4308)
    ports: [u8; 4],
}

impl I4702 {
    pub fn new(chip_id: u8) -> Self {
        Self {
            memory: [0xFF; 256], // unprogrammed EPROM reads as all 1s
            chip_id,
            programming_mode: false,
            program_address: 0,
            ports: [0; 4],
        }
    }

    /// Load data into the EPROM (simulates programming)
    pub fn load(&mut self, data: &[u8]) {
        let len = data.len().min(self.memory.len());
        self.memory[..len].copy_from_slice(&data[..len]);
    }

    /// Read from the EPROM at the given address
    pub fn read_rom(&self, address: u8) -> u8 {
        self.memory[address as usize]
    }

    /// Enter programming mode (Vpp applied externally)
    pub fn set_programming_mode(&mut self, enabled: bool) {
        self.programming_mode = enabled;
    }

    /// Whether programming mode is active
    pub fn is_programming(&self) -> bool {
        self.programming_mode
    }

    /// Set programming address
    pub fn set_program_address(&mut self, address: u8) {
        self.program_address = address;
    }

    /// Program one byte at the current address.
    /// EPROM programming can only clear bits (1->0), not set them.
    /// Returns true if programming succeeded.
    pub fn program_byte(&mut self, data: u8) -> bool {
        if self.programming_mode {
            // EPROM: can only go from 1 to 0 (AND with existing data)
            let addr = self.program_address as usize;
            self.memory[addr] &= data;
            true
        } else {
            false
        }
    }

    /// Erase entire EPROM (simulates UV exposure)
    pub fn uv_erase(&mut self) {
        self.memory = [0xFF; 256];
    }

    /// Check if a byte is fully erased
    pub fn is_erased(&self, address: u8) -> bool {
        self.memory[address as usize] == 0xFF
    }
}

impl Default for I4702 {
    fn default() -> Self {
        Self::new(0)
    }
}

impl super::Chip for I4702 {
    fn name(&self) -> &'static str {
        "4702"
    }

    fn reset(&mut self) {
        // EPROM contents preserved across reset
        self.programming_mode = false;
        self.program_address = 0;
        self.ports = [0; 4];
    }

    fn tick(&mut self, _phase: BusCycle) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unprogrammed_reads_all_ones() {
        let rom = I4702::new(0);
        for addr in 0..=255u8 {
            assert_eq!(rom.read_rom(addr), 0xFF);
        }
    }

    #[test]
    fn load_and_read_roundtrip() {
        let mut rom = I4702::new(0);
        let data: Vec<u8> = (0..=255).collect();
        rom.load(&data);

        for i in 0..=255u8 {
            assert_eq!(rom.read_rom(i), i);
        }
    }

    #[test]
    fn programming_only_clears_bits() {
        let mut rom = I4702::new(0);
        // Start erased (all 1s)
        assert_eq!(rom.read_rom(0), 0xFF);

        rom.set_programming_mode(true);
        rom.set_program_address(0);
        rom.program_byte(0xA5); // program specific bits to 0

        assert_eq!(rom.read_rom(0), 0xA5);

        // Try to program again: can only clear more bits, not set them
        rom.program_byte(0x0F);
        assert_eq!(rom.read_rom(0), 0x05); // 0xA5 & 0x0F = 0x05
    }

    #[test]
    fn programming_fails_without_vpp() {
        let mut rom = I4702::new(0);
        rom.set_program_address(0);
        let result = rom.program_byte(0x00);
        assert!(!result);
        assert_eq!(rom.read_rom(0), 0xFF); // unchanged
    }

    #[test]
    fn uv_erase_restores_all_ones() {
        let mut rom = I4702::new(0);
        rom.load(&[0x00; 256]); // all zeros

        rom.uv_erase();

        for addr in 0..=255u8 {
            assert!(rom.is_erased(addr));
            assert_eq!(rom.read_rom(addr), 0xFF);
        }
    }

    #[test]
    fn reset_preserves_contents() {
        use crate::Chip;
        let mut rom = I4702::new(0);
        rom.load(&[0xDE, 0xAD]);

        rom.reset();

        assert_eq!(rom.read_rom(0), 0xDE);
        assert_eq!(rom.read_rom(1), 0xAD);
        assert!(!rom.is_programming());
    }

    #[test]
    fn chip_id_stored() {
        let rom = I4702::new(7);
        assert_eq!(rom.chip_id, 7);
    }

    #[test]
    fn chip_trait_name() {
        use crate::Chip;
        let rom = I4702::new(0);
        assert_eq!(rom.name(), "4702");
    }
}
