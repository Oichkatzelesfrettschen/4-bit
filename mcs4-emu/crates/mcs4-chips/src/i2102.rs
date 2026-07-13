//! Intel 2102 1024-bit static RAM.
//!
//! The 2102 has ten address inputs, active-low chip enable, a read/write
//! control, and a tri-state output.  A static RAM retains data across reset,
//! but power-on contents are not specified by the source record.  This model
//! represents that distinction instead of silently treating uninitialized
//! storage as zero.

use mcs4_bus::BusCycle;

/// One externally visible 2102 data output state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum I2102Output {
    /// The device does not drive the bus.
    HighImpedance,
    /// The selected cell has no established post-power-cycle value.
    Unknown,
    /// The device drives logic zero.
    Zero,
    /// The device drives logic one.
    One,
}

/// Intel 2102: 1024 words by one bit static RAM.
#[derive(Clone, Debug)]
pub struct I2102 {
    bits: [u8; 128],
    known: [u8; 128],
    address: u16,
    data_in: bool,
    ce_n: bool,
    rw: bool,
}

impl Default for I2102 {
    fn default() -> Self {
        Self::new()
    }
}

impl I2102 {
    /// Construct a device with unknown retained contents and inactive control.
    pub fn new() -> Self {
        Self {
            bits: [0; 128],
            known: [0; 128],
            address: 0,
            data_in: false,
            ce_n: true,
            rw: true,
        }
    }

    /// Construct a known-zero device for deterministic tests.
    pub fn zeroed() -> Self {
        Self {
            bits: [0; 128],
            known: [0xff; 128],
            address: 0,
            data_in: false,
            ce_n: true,
            rw: true,
        }
    }

    /// Construct a known-filled device for deterministic tests.
    pub fn filled(value: bool) -> Self {
        Self {
            bits: [u8::from(value) * 0xff; 128],
            known: [0xff; 128],
            address: 0,
            data_in: false,
            ce_n: true,
            rw: true,
        }
    }

    /// Select one of the 1024 storage words.
    pub fn set_address(&mut self, address: u16) {
        self.address = address & 0x03ff;
    }

    /// Drive the one-bit data input.
    pub fn set_data_in(&mut self, data: bool) {
        self.data_in = data;
    }

    /// Set the active-low chip-enable input.
    pub fn set_ce_n(&mut self, inactive: bool) {
        self.ce_n = inactive;
    }

    /// Set read/write high for read and low for write.
    ///
    /// The 2102 write-cycle waveform retains the selected input bit at the
    /// rising edge that ends the active-low write pulse. The caller presents
    /// address, data, and chip enable before ending that pulse.
    pub fn set_rw(&mut self, read: bool) {
        let closing_write_pulse = !self.ce_n && !self.rw && read;
        self.rw = read;
        if closing_write_pulse {
            self.write_selected_cell(self.data_in);
        }
    }

    /// Read the tri-state data output.
    pub fn data_out(&self) -> I2102Output {
        if self.ce_n || !self.rw {
            I2102Output::HighImpedance
        } else {
            self.read_selected_cell()
        }
    }

    /// Read one word without asserting a board-level interface.
    pub fn read_direct(&self, address: u16) -> I2102Output {
        self.read_cell(address & 0x03ff)
    }

    /// Write one word without asserting a board-level interface.
    pub fn write_direct(&mut self, address: u16, data: bool) {
        self.write_cell(address & 0x03ff, data);
    }

    /// Execute one explicit card-local write cycle.
    pub fn write_cycle(&mut self, address: u16, data: bool) {
        self.set_address(address);
        self.set_data_in(data);
        self.set_ce_n(false);
        self.set_rw(false);
        self.set_rw(true);
        self.set_ce_n(true);
    }

    /// Execute one explicit card-local read cycle.
    pub fn read_cycle(&mut self, address: u16) -> I2102Output {
        self.set_address(address);
        self.set_ce_n(false);
        self.set_rw(true);
        let output = self.data_out();
        self.set_ce_n(true);
        output
    }

    /// Invalidate all retained bits to model a power cycle.
    pub fn power_cycle(&mut self) {
        self.bits = [0; 128];
        self.known = [0; 128];
        self.reset_controls();
    }

    fn read_selected_cell(&self) -> I2102Output {
        self.read_cell(self.address)
    }

    fn read_cell(&self, address: u16) -> I2102Output {
        let byte_index = usize::from(address >> 3);
        let bit_mask = 1u8 << (address & 0x07);
        if self.known[byte_index] & bit_mask == 0 {
            I2102Output::Unknown
        } else if self.bits[byte_index] & bit_mask == 0 {
            I2102Output::Zero
        } else {
            I2102Output::One
        }
    }

    fn write_selected_cell(&mut self, data: bool) {
        self.write_cell(self.address, data);
    }

    fn write_cell(&mut self, address: u16, data: bool) {
        let byte_index = usize::from(address >> 3);
        let bit_mask = 1u8 << (address & 0x07);
        if data {
            self.bits[byte_index] |= bit_mask;
        } else {
            self.bits[byte_index] &= !bit_mask;
        }
        self.known[byte_index] |= bit_mask;
    }

    fn reset_controls(&mut self) {
        self.address = 0;
        self.data_in = false;
        self.ce_n = true;
        self.rw = true;
    }
}

impl super::Chip for I2102 {
    fn name(&self) -> &'static str {
        "2102"
    }

    fn reset(&mut self) {
        self.reset_controls();
    }

    fn tick(&mut self, _phase: BusCycle) {}
}

#[cfg(test)]
mod tests {
    use super::{I2102Output, I2102};
    use crate::Chip;

    #[test]
    fn power_on_state_is_unknown() {
        let ram = I2102::new();
        assert_eq!(ram.read_direct(0x123), I2102Output::Unknown);
    }

    #[test]
    fn stores_one_bit_at_each_of_1024_addresses() {
        let mut ram = I2102::zeroed();
        ram.write_cycle(0x000, true);
        ram.write_cycle(0x3ff, true);

        assert_eq!(ram.read_cycle(0x000), I2102Output::One);
        assert_eq!(ram.read_cycle(0x3ff), I2102Output::One);
        assert_eq!(ram.read_cycle(0x001), I2102Output::Zero);
    }

    #[test]
    fn deselected_and_write_cycle_outputs_are_high_impedance() {
        let mut ram = I2102::zeroed();
        ram.set_address(0x123);
        ram.set_ce_n(true);
        assert_eq!(ram.data_out(), I2102Output::HighImpedance);

        ram.set_ce_n(false);
        ram.set_rw(false);
        assert_eq!(ram.data_out(), I2102Output::HighImpedance);
    }

    #[test]
    fn rising_rw_edge_retains_data_after_the_active_low_write_pulse() {
        let mut ram = I2102::zeroed();
        ram.set_address(0x123);
        ram.set_ce_n(false);
        ram.set_data_in(false);
        ram.set_rw(false);
        ram.set_data_in(true);

        assert_eq!(ram.read_direct(0x123), I2102Output::Zero);

        ram.set_rw(true);
        ram.set_ce_n(true);
        assert_eq!(ram.read_direct(0x123), I2102Output::One);
    }

    #[test]
    fn reset_preserves_static_memory_contents() {
        let mut ram = I2102::zeroed();
        ram.write_direct(0x2aa, true);
        ram.reset();

        assert_eq!(ram.read_direct(0x2aa), I2102Output::One);
    }

    #[test]
    fn power_cycle_invalidates_contents() {
        let mut ram = I2102::zeroed();
        ram.write_direct(0x2aa, true);
        ram.power_cycle();

        assert_eq!(ram.read_direct(0x2aa), I2102Output::Unknown);
    }

    #[test]
    fn address_masks_to_ten_bits() {
        let mut ram = I2102::zeroed();
        ram.write_direct(0x000, true);

        assert_eq!(ram.read_direct(0x400), I2102Output::One);
    }
}
