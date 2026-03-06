//! Intel 3404 6-Bit D-Type Latch + Dual NAND Buffer
//!
//! The 3404 contains a 6-bit D-type latch with a common clock input and
//! two independent 2-input NAND gates. Used as a TTL support chip in
//! MCS-4/MCS-40 system designs for address latching and control decoding.

use mcs4_bus::BusCycle;

/// Intel 3404: 6-bit D latch + dual NAND buffer
#[derive(Clone, Debug)]
pub struct I3404 {
    /// 6-bit D latch data (latched on clock rising edge)
    latch: u8,
    /// 6-bit D input
    d_input: u8,
    /// Clock state (previous)
    last_clock: bool,
    /// NAND gate A inputs
    nand_a: (bool, bool),
    /// NAND gate B inputs
    nand_b: (bool, bool),
}

impl Default for I3404 {
    fn default() -> Self {
        Self::new()
    }
}

impl I3404 {
    pub fn new() -> Self {
        Self {
            latch: 0,
            d_input: 0,
            last_clock: false,
            nand_a: (false, false),
            nand_b: (false, false),
        }
    }

    /// Set the 6-bit D input (only lower 6 bits used)
    pub fn set_d_input(&mut self, data: u8) {
        self.d_input = data & 0x3F;
    }

    /// Pulse the clock line. Latch captures D input on rising edge.
    pub fn set_clock(&mut self, state: bool) {
        if !self.last_clock && state {
            self.latch = self.d_input;
        }
        self.last_clock = state;
    }

    /// Get the latched 6-bit Q output
    pub fn q_output(&self) -> u8 {
        self.latch
    }

    /// Set NAND gate A inputs
    pub fn set_nand_a(&mut self, in1: bool, in2: bool) {
        self.nand_a = (in1, in2);
    }

    /// Set NAND gate B inputs
    pub fn set_nand_b(&mut self, in1: bool, in2: bool) {
        self.nand_b = (in1, in2);
    }

    /// Get NAND gate A output
    pub fn nand_a_output(&self) -> bool {
        !(self.nand_a.0 && self.nand_a.1)
    }

    /// Get NAND gate B output
    pub fn nand_b_output(&self) -> bool {
        !(self.nand_b.0 && self.nand_b.1)
    }
}

impl super::Chip for I3404 {
    fn name(&self) -> &'static str {
        "3404"
    }

    fn reset(&mut self) {
        self.latch = 0;
        self.d_input = 0;
        self.last_clock = false;
        self.nand_a = (false, false);
        self.nand_b = (false, false);
    }

    fn tick(&mut self, _phase: BusCycle) {
        // Clock is driven externally
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latch_captures_on_rising_edge() {
        let mut ic = I3404::new();
        ic.set_d_input(0x2A);
        ic.set_clock(false);
        assert_eq!(ic.q_output(), 0);

        ic.set_clock(true); // rising edge
        assert_eq!(ic.q_output(), 0x2A);
    }

    #[test]
    fn latch_holds_after_d_changes() {
        let mut ic = I3404::new();
        ic.set_d_input(0x15);
        ic.set_clock(true);
        assert_eq!(ic.q_output(), 0x15);

        // Change D with clock still high -- no re-latch
        ic.set_d_input(0x3F);
        ic.set_clock(true); // no rising edge
        assert_eq!(ic.q_output(), 0x15);
    }

    #[test]
    fn latch_masks_to_6_bits() {
        let mut ic = I3404::new();
        ic.set_d_input(0xFF);
        ic.set_clock(true);
        assert_eq!(ic.q_output(), 0x3F);
    }

    #[test]
    fn nand_gate_a_truth_table() {
        let mut ic = I3404::new();
        ic.set_nand_a(false, false);
        assert!(ic.nand_a_output());

        ic.set_nand_a(true, false);
        assert!(ic.nand_a_output());

        ic.set_nand_a(false, true);
        assert!(ic.nand_a_output());

        ic.set_nand_a(true, true);
        assert!(!ic.nand_a_output());
    }

    #[test]
    fn nand_gate_b_truth_table() {
        let mut ic = I3404::new();
        ic.set_nand_b(true, true);
        assert!(!ic.nand_b_output());

        ic.set_nand_b(false, true);
        assert!(ic.nand_b_output());
    }

    #[test]
    fn chip_trait_name() {
        use crate::Chip;
        let ic = I3404::new();
        assert_eq!(ic.name(), "3404");
    }
}
