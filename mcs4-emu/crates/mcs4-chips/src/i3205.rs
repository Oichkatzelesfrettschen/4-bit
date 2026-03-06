//! Intel 3205 1-of-8 Binary Decoder
//!
//! The 3205 is a TTL 1-of-8 decoder used for address decoding in MCS-4/MCS-40
//! systems. It accepts a 3-bit binary input and activates one of 8 active-low
//! outputs. Two active-low enable inputs (E1, E2) and one active-high enable
//! (E3) provide chip-select gating.

use mcs4_bus::BusCycle;

/// Intel 3205: 1-of-8 binary decoder
#[derive(Clone, Debug)]
pub struct I3205 {
    /// 3-bit address input (A0-A2)
    address: u8,
    /// Enable 1 (active low)
    enable1_n: bool,
    /// Enable 2 (active low)
    enable2_n: bool,
    /// Enable 3 (active high)
    enable3: bool,
}

impl Default for I3205 {
    fn default() -> Self {
        Self::new()
    }
}

impl I3205 {
    pub fn new() -> Self {
        Self {
            address: 0,
            enable1_n: true, // disabled by default
            enable2_n: true, // disabled by default
            enable3: false,  // disabled by default
        }
    }

    /// Set the 3-bit address input (only lower 3 bits used)
    pub fn set_address(&mut self, addr: u8) {
        self.address = addr & 0x07;
    }

    /// Set enable inputs
    pub fn set_enables(&mut self, e1_n: bool, e2_n: bool, e3: bool) {
        self.enable1_n = e1_n;
        self.enable2_n = e2_n;
        self.enable3 = e3;
    }

    /// Check if the decoder is enabled
    pub fn is_enabled(&self) -> bool {
        !self.enable1_n && !self.enable2_n && self.enable3
    }

    /// Get all 8 outputs as a byte (active-low: 0 = selected, 1 = not selected)
    pub fn outputs(&self) -> u8 {
        if !self.is_enabled() {
            return 0xFF; // all outputs high (inactive) when disabled
        }
        // Active-low: selected output is 0, all others are 1
        !(1u8 << self.address)
    }

    /// Get a specific output (0-7), returns true if active (low)
    pub fn output(&self, idx: u8) -> bool {
        if idx > 7 {
            return false;
        }
        self.is_enabled() && self.address == idx
    }
}

impl super::Chip for I3205 {
    fn name(&self) -> &'static str {
        "3205"
    }

    fn reset(&mut self) {
        self.address = 0;
        self.enable1_n = true;
        self.enable2_n = true;
        self.enable3 = false;
    }

    fn tick(&mut self, _phase: BusCycle) {
        // Combinational logic: no clock-driven behavior
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_disabled_all_high() {
        let dec = I3205::new();
        assert_eq!(dec.outputs(), 0xFF);
    }

    #[test]
    fn decode_each_output() {
        let mut dec = I3205::new();
        dec.set_enables(false, false, true);

        for addr in 0..8u8 {
            dec.set_address(addr);
            for out in 0..8u8 {
                if out == addr {
                    assert!(dec.output(out), "Output {} should be active for address {}", out, addr);
                } else {
                    assert!(
                        !dec.output(out),
                        "Output {} should be inactive for address {}",
                        out,
                        addr
                    );
                }
            }
        }
    }

    #[test]
    fn outputs_byte_active_low() {
        let mut dec = I3205::new();
        dec.set_enables(false, false, true);

        dec.set_address(0);
        assert_eq!(dec.outputs(), 0xFE); // bit 0 low

        dec.set_address(3);
        assert_eq!(dec.outputs(), 0xF7); // bit 3 low

        dec.set_address(7);
        assert_eq!(dec.outputs(), 0x7F); // bit 7 low
    }

    #[test]
    fn enable_gating() {
        let mut dec = I3205::new();
        dec.set_address(5);

        // E1=high disables
        dec.set_enables(true, false, true);
        assert!(!dec.output(5));

        // E2=high disables
        dec.set_enables(false, true, true);
        assert!(!dec.output(5));

        // E3=low disables
        dec.set_enables(false, false, false);
        assert!(!dec.output(5));

        // All enables correct
        dec.set_enables(false, false, true);
        assert!(dec.output(5));
    }

    #[test]
    fn address_masks_to_3_bits() {
        let mut dec = I3205::new();
        dec.set_enables(false, false, true);
        dec.set_address(0xFF);
        assert_eq!(dec.outputs(), 0x7F); // address 7 selected
    }

    #[test]
    fn out_of_range_output_returns_false() {
        let mut dec = I3205::new();
        dec.set_enables(false, false, true);
        dec.set_address(0);
        assert!(!dec.output(8));
        assert!(!dec.output(255));
    }

    #[test]
    fn chip_trait_name() {
        use crate::Chip;
        let dec = I3205::new();
        assert_eq!(dec.name(), "3205");
    }
}
