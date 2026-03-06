//! Intel 2101 256x4 Static RAM
//!
//! The 2101 is a standalone 256x4-bit static RAM in a standard TTL-compatible
//! package. Unlike the MCS-40-native 4101, the 2101 uses standard address and
//! data pins rather than the MCS-40 bus protocol, making it suitable for use
//! with the 4289 Standard Memory Interface in expanded MCS-40 systems.
//!
//! Pin-compatible with industry standard 2101 / 9101 / 7489 SRAMs.

use mcs4_bus::BusCycle;

/// Intel 2101: 256x4 static RAM (standard interface)
#[derive(Clone, Debug)]
pub struct I2101 {
    /// 256 x 4-bit memory array
    memory: [u8; 256],
    /// 8-bit address input
    address: u8,
    /// 4-bit data input
    data_in: u8,
    /// Chip enable (active low)
    ce_n: bool,
    /// Write enable (active low)
    we_n: bool,
    /// Output enable (active low)
    oe_n: bool,
}

impl Default for I2101 {
    fn default() -> Self {
        Self::new()
    }
}

impl I2101 {
    pub fn new() -> Self {
        Self {
            memory: [0; 256],
            address: 0,
            data_in: 0,
            ce_n: true, // disabled
            we_n: true, // read mode
            oe_n: true, // output disabled
        }
    }

    /// Set address input (8 bits)
    pub fn set_address(&mut self, addr: u8) {
        self.address = addr;
    }

    /// Set data input (4-bit, lower nibble)
    pub fn set_data_in(&mut self, data: u8) {
        self.data_in = data & 0x0F;
    }

    /// Set chip enable (active low)
    pub fn set_ce_n(&mut self, state: bool) {
        self.ce_n = state;
    }

    /// Set write enable (active low)
    pub fn set_we_n(&mut self, state: bool) {
        let was_writing = !self.ce_n && !self.we_n;
        self.we_n = state;
        let now_writing = !self.ce_n && !self.we_n;

        // Write occurs when CE and WE are both active
        if now_writing && !was_writing {
            self.memory[self.address as usize] = self.data_in;
        }
    }

    /// Set output enable (active low)
    pub fn set_oe_n(&mut self, state: bool) {
        self.oe_n = state;
    }

    /// Read data output (tri-state: returns None if output disabled)
    pub fn data_out(&self) -> Option<u8> {
        if !self.ce_n && !self.oe_n && self.we_n {
            Some(self.memory[self.address as usize] & 0x0F)
        } else {
            None // tri-state (high-Z)
        }
    }

    /// Direct read for debugging
    pub fn read_direct(&self, addr: u8) -> u8 {
        self.memory[addr as usize] & 0x0F
    }

    /// Direct write for debugging
    pub fn write_direct(&mut self, addr: u8, data: u8) {
        self.memory[addr as usize] = data & 0x0F;
    }

    /// Perform a write cycle (convenience method)
    pub fn write_cycle(&mut self, addr: u8, data: u8) {
        self.set_address(addr);
        self.set_data_in(data);
        self.set_ce_n(false);
        self.set_we_n(false); // triggers write
        self.set_we_n(true); // end write
        self.set_ce_n(true);
    }

    /// Perform a read cycle (convenience method)
    pub fn read_cycle(&mut self, addr: u8) -> u8 {
        self.set_address(addr);
        self.set_ce_n(false);
        self.set_oe_n(false);
        let val = self.data_out().unwrap_or(0x0F);
        self.set_oe_n(true);
        self.set_ce_n(true);
        val
    }
}

impl super::Chip for I2101 {
    fn name(&self) -> &'static str {
        "2101"
    }

    fn reset(&mut self) {
        self.address = 0;
        self.data_in = 0;
        self.ce_n = true;
        self.we_n = true;
        self.oe_n = true;
        // Memory contents persist through reset (static RAM)
    }

    fn tick(&mut self, _phase: BusCycle) {
        // Asynchronous SRAM: no clock-driven behavior
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_back() {
        let mut ram = I2101::new();
        ram.write_cycle(0x42, 0xA);
        assert_eq!(ram.read_cycle(0x42), 0xA);
    }

    #[test]
    fn all_addresses_independent() {
        let mut ram = I2101::new();
        for addr in 0..=255u8 {
            ram.write_cycle(addr, addr & 0x0F);
        }
        for addr in 0..=255u8 {
            assert_eq!(ram.read_cycle(addr), addr & 0x0F);
        }
    }

    #[test]
    fn data_masks_to_4_bits() {
        let mut ram = I2101::new();
        ram.write_cycle(0, 0xFF);
        assert_eq!(ram.read_cycle(0), 0x0F);
    }

    #[test]
    fn tristate_when_disabled() {
        let mut ram = I2101::new();
        ram.write_direct(10, 0x05);

        // CE disabled
        ram.set_address(10);
        ram.set_ce_n(true);
        ram.set_oe_n(false);
        assert_eq!(ram.data_out(), None);

        // OE disabled
        ram.set_ce_n(false);
        ram.set_oe_n(true);
        assert_eq!(ram.data_out(), None);

        // Both enabled, read mode
        ram.set_oe_n(false);
        ram.set_we_n(true);
        assert_eq!(ram.data_out(), Some(0x05));
    }

    #[test]
    fn tristate_during_write() {
        let mut ram = I2101::new();
        ram.set_address(0);
        ram.set_ce_n(false);
        ram.set_we_n(false);
        ram.set_oe_n(false);
        // Output disabled during write even if OE is low
        assert_eq!(ram.data_out(), None);
    }

    #[test]
    fn reset_preserves_memory() {
        use crate::Chip;
        let mut ram = I2101::new();
        ram.write_cycle(5, 0x0C);
        ram.reset();
        assert_eq!(ram.read_direct(5), 0x0C);
    }

    #[test]
    fn direct_access() {
        let mut ram = I2101::new();
        ram.write_direct(100, 0x07);
        assert_eq!(ram.read_direct(100), 0x07);
    }

    #[test]
    fn chip_enable_required_for_write() {
        let mut ram = I2101::new();
        ram.set_address(20);
        ram.set_data_in(0x09);
        ram.set_ce_n(true); // disabled
        ram.set_we_n(false);
        ram.set_we_n(true);
        assert_eq!(ram.read_direct(20), 0x00); // no write occurred
    }

    #[test]
    fn chip_trait_name() {
        use crate::Chip;
        let ram = I2101::new();
        assert_eq!(ram.name(), "2101");
    }
}
