//! Intel 3404 high-speed six-bit latch.
//!
//! Intel's 1975 data catalog describes the 3404 as six inverting storage
//! latches. Active-low write enable W1 controls D1 through D4; active-low W2
//! controls D5 and D6. An enabled group is transparent and inverts its data
//! inputs. A rising write-enable edge retains the inverted values. The device
//! has no reset pin, so power-on latch contents remain unknown.

use mcs4_bus::BusCycle;

/// One observable 3404 inverted-output state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum I3404Output {
    /// The stored latch state is not established after construction or power cycle.
    Unknown,
    /// The active-low output is asserted.
    Low,
    /// The active-low output is deasserted.
    High,
}

/// Intel 3404 source-bound six-bit latch.
#[derive(Clone, Debug)]
pub struct I3404 {
    data_inputs: u8,
    latched_outputs: u8,
    known_latched_outputs: u8,
    write_enable_1_n: bool,
    write_enable_2_n: bool,
}

impl Default for I3404 {
    fn default() -> Self {
        Self::new()
    }
}

impl I3404 {
    const GROUP_1_MASK: u8 = 0x0f;
    const GROUP_2_MASK: u8 = 0x30;
    const ALL_BITS_MASK: u8 = Self::GROUP_1_MASK | Self::GROUP_2_MASK;

    /// Construct a powered device with unknown stored outputs and both latch groups closed.
    pub const fn new() -> Self {
        Self {
            data_inputs: 0,
            latched_outputs: 0,
            known_latched_outputs: 0,
            write_enable_1_n: true,
            write_enable_2_n: true,
        }
    }

    /// Present all six data inputs D1 through D6. Bits above bit five are ignored.
    pub fn set_data_inputs(&mut self, inputs: u8) {
        self.data_inputs = inputs & Self::ALL_BITS_MASK;
    }

    /// Set the active-low W1 input that controls D1 through D4.
    pub fn set_write_enable_1_n(&mut self, state: bool) {
        self.capture_on_write_disable(self.write_enable_1_n, state, Self::GROUP_1_MASK);
        self.write_enable_1_n = state;
    }

    /// Set the active-low W2 input that controls D5 and D6.
    pub fn set_write_enable_2_n(&mut self, state: bool) {
        self.capture_on_write_disable(self.write_enable_2_n, state, Self::GROUP_2_MASK);
        self.write_enable_2_n = state;
    }

    /// Return one active-low output, indexed from zero for O1 through O6.
    pub fn output(&self, bit: u8) -> I3404Output {
        assert!(bit < 6, "3404 output bit must be below six");
        let mask = 1u8 << bit;
        let group_is_transparent = if bit < 4 {
            !self.write_enable_1_n
        } else {
            !self.write_enable_2_n
        };
        if group_is_transparent {
            return Self::output_from_bit((!self.data_inputs & mask) != 0);
        }
        if self.known_latched_outputs & mask == 0 {
            return I3404Output::Unknown;
        }
        Self::output_from_bit(self.latched_outputs & mask != 0)
    }

    /// Return all six active-low outputs as a bit vector and knownness mask.
    ///
    /// Unknown output bits are zero in `value` and clear in `known_mask`.
    pub fn outputs(&self) -> (u8, u8) {
        let mut value = 0u8;
        let mut known_mask = 0u8;
        for bit in 0..6 {
            match self.output(bit) {
                I3404Output::Unknown => {}
                I3404Output::Low => known_mask |= 1 << bit,
                I3404Output::High => {
                    value |= 1 << bit;
                    known_mask |= 1 << bit;
                }
            }
        }
        (value, known_mask)
    }

    /// Invalidate all stored values after an explicit device power cycle.
    pub fn power_cycle(&mut self) {
        self.data_inputs = 0;
        self.latched_outputs = 0;
        self.known_latched_outputs = 0;
        self.write_enable_1_n = true;
        self.write_enable_2_n = true;
    }

    fn capture_on_write_disable(&mut self, previous_state: bool, new_state: bool, mask: u8) {
        if !previous_state && new_state {
            self.latched_outputs = (self.latched_outputs & !mask) | (!self.data_inputs & mask);
            self.known_latched_outputs |= mask;
        }
    }

    const fn output_from_bit(high: bool) -> I3404Output {
        if high {
            I3404Output::High
        } else {
            I3404Output::Low
        }
    }
}

impl super::Chip for I3404 {
    fn name(&self) -> &'static str {
        "3404"
    }

    fn reset(&mut self) {
        // The 3404 has no reset input. Reset does not alter retained latches.
    }

    fn tick(&mut self, _phase: BusCycle) {
        // Write-enable inputs drive transparent and hold behavior externally.
    }
}

#[cfg(test)]
mod tests {
    use super::{I3404Output, I3404};
    use crate::Chip;

    #[test]
    fn constructed_latches_are_unknown_while_write_is_disabled() {
        let latch = I3404::new();
        assert_eq!(latch.outputs(), (0, 0));
        assert_eq!(latch.output(0), I3404Output::Unknown);
        assert_eq!(latch.output(5), I3404Output::Unknown);
    }

    #[test]
    fn first_group_is_transparent_and_inverting_while_w1_is_low() {
        let mut latch = I3404::new();
        latch.set_data_inputs(0b00_1010);
        latch.set_write_enable_1_n(false);

        assert_eq!(latch.outputs(), (0b00_0101, 0b00_1111));

        latch.set_data_inputs(0b00_0011);
        assert_eq!(latch.outputs(), (0b00_1100, 0b00_1111));
    }

    #[test]
    fn rising_w1_latches_the_inverted_first_group_value() {
        let mut latch = I3404::new();
        latch.set_data_inputs(0b00_1010);
        latch.set_write_enable_1_n(false);
        latch.set_write_enable_1_n(true);
        latch.set_data_inputs(0b00_0011);

        assert_eq!(latch.outputs(), (0b00_0101, 0b00_1111));
    }

    #[test]
    fn second_group_is_independent_and_inverting_while_w2_is_low() {
        let mut latch = I3404::new();
        latch.set_data_inputs(0b11_0000);
        latch.set_write_enable_2_n(false);

        assert_eq!(latch.outputs(), (0, 0b11_0000));
        assert_eq!(latch.output(0), I3404Output::Unknown);

        latch.set_data_inputs(0b01_0000);
        assert_eq!(latch.outputs(), (0b10_0000, 0b11_0000));
    }

    #[test]
    fn reset_preserves_the_documented_latch_state() {
        let mut latch = I3404::new();
        latch.set_data_inputs(0b00_0001);
        latch.set_write_enable_1_n(false);
        latch.set_write_enable_1_n(true);
        latch.reset();

        assert_eq!(latch.outputs(), (0b00_1110, 0b00_1111));
    }

    #[test]
    fn power_cycle_invalidates_all_latched_outputs() {
        let mut latch = I3404::new();
        latch.set_data_inputs(0b11_1111);
        latch.set_write_enable_1_n(false);
        latch.set_write_enable_2_n(false);
        latch.set_write_enable_1_n(true);
        latch.set_write_enable_2_n(true);
        latch.power_cycle();

        assert_eq!(latch.outputs(), (0, 0));
    }

    #[test]
    fn chip_trait_name() {
        let latch = I3404::new();
        assert_eq!(latch.name(), "3404");
    }
}
