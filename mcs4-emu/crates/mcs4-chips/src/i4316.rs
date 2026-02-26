//! Intel 4316 LCD Driver
//!
//! The 4316 drives LCD segments via a multiplexed backplane scheme.
//! It latches segment data from the MCS-40 bus and generates the
//! alternating backplane waveform needed for LCD operation.
//!
//! Organization: multiple segment registers with configurable duty cycle.

use mcs4_bus::BusCycle;

/// Intel 4316: LCD Segment Driver
///
/// Drives LCD segments with multiplexed backplane waveform generation.
#[derive(Clone, Debug)]
pub struct I4316 {
    /// Segment data registers (up to 16 segments, 4 bits each)
    segments: [u8; 16],

    /// Backplane phase (toggles for AC drive)
    backplane_phase: bool,

    /// Duty cycle numerator (1-4 for 1/1 to 1/4 multiplex)
    duty_cycle: u8,

    /// Current digit being driven (for multiplexed operation)
    active_digit: u8,

    /// Number of digits (segments groups)
    num_digits: u8,

    /// Chip ID for bus addressing
    pub chip_id: u8,
}

impl I4316 {
    pub fn new(chip_id: u8, num_digits: u8) -> Self {
        Self {
            segments: [0; 16],
            backplane_phase: false,
            duty_cycle: 1,
            active_digit: 0,
            num_digits: num_digits.min(16),
            chip_id,
        }
    }

    /// Write segment data for a specific digit position
    pub fn write_segment(&mut self, digit: u8, data: u8) {
        if (digit as usize) < self.segments.len() {
            self.segments[digit as usize] = data & 0x0F;
        }
    }

    /// Read segment data for a specific digit position
    pub fn read_segment(&self, digit: u8) -> u8 {
        if (digit as usize) < self.segments.len() {
            self.segments[digit as usize] & 0x0F
        } else {
            0
        }
    }

    /// Toggle backplane phase (called periodically for AC drive)
    pub fn toggle_backplane(&mut self) {
        self.backplane_phase = !self.backplane_phase;
    }

    /// Current backplane phase
    pub fn backplane_phase(&self) -> bool {
        self.backplane_phase
    }

    /// Advance to next digit in multiplex sequence
    pub fn advance_digit(&mut self) {
        self.active_digit = (self.active_digit + 1) % self.num_digits.max(1);
    }

    /// Currently active digit in multiplex sequence
    pub fn active_digit(&self) -> u8 {
        self.active_digit
    }

    /// Set multiplex duty cycle (1=static, 2=1/2, 3=1/3, 4=1/4)
    pub fn set_duty_cycle(&mut self, duty: u8) {
        self.duty_cycle = duty.clamp(1, 4);
    }

    /// Get the segment output for the currently active digit,
    /// XORed with backplane phase for LCD AC drive.
    pub fn segment_output(&self) -> u8 {
        let data = self.segments[self.active_digit as usize] & 0x0F;
        if self.backplane_phase {
            (!data) & 0x0F // invert for AC drive
        } else {
            data
        }
    }
}

impl Default for I4316 {
    fn default() -> Self {
        Self::new(0, 4)
    }
}

impl super::Chip for I4316 {
    fn name(&self) -> &'static str {
        "4316"
    }

    fn reset(&mut self) {
        self.segments = [0; 16];
        self.backplane_phase = false;
        self.active_digit = 0;
    }

    fn tick(&mut self, _phase: BusCycle) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_segments() {
        let mut lcd = I4316::new(0, 4);
        lcd.write_segment(0, 0xA);
        lcd.write_segment(1, 0x5);
        assert_eq!(lcd.read_segment(0), 0xA);
        assert_eq!(lcd.read_segment(1), 0x5);
    }

    #[test]
    fn data_masked_to_4_bits() {
        let mut lcd = I4316::new(0, 4);
        lcd.write_segment(0, 0xFF);
        assert_eq!(lcd.read_segment(0), 0x0F);
    }

    #[test]
    fn backplane_toggles() {
        let mut lcd = I4316::new(0, 4);
        assert!(!lcd.backplane_phase());
        lcd.toggle_backplane();
        assert!(lcd.backplane_phase());
        lcd.toggle_backplane();
        assert!(!lcd.backplane_phase());
    }

    #[test]
    fn segment_output_inverts_with_backplane() {
        let mut lcd = I4316::new(0, 4);
        lcd.write_segment(0, 0xA); // 1010

        // Backplane low: output = data
        assert_eq!(lcd.segment_output(), 0xA);

        // Backplane high: output = ~data (for AC drive)
        lcd.toggle_backplane();
        assert_eq!(lcd.segment_output(), 0x5); // 0101
    }

    #[test]
    fn digit_advance_wraps() {
        let mut lcd = I4316::new(0, 3);
        assert_eq!(lcd.active_digit(), 0);
        lcd.advance_digit();
        assert_eq!(lcd.active_digit(), 1);
        lcd.advance_digit();
        assert_eq!(lcd.active_digit(), 2);
        lcd.advance_digit();
        assert_eq!(lcd.active_digit(), 0); // wraps
    }

    #[test]
    fn reset_clears_segments() {
        use crate::Chip;
        let mut lcd = I4316::new(0, 4);
        lcd.write_segment(0, 0xF);
        lcd.toggle_backplane();

        lcd.reset();

        assert_eq!(lcd.read_segment(0), 0);
        assert!(!lcd.backplane_phase());
    }

    #[test]
    fn chip_trait_name() {
        use crate::Chip;
        let lcd = I4316::new(0, 4);
        assert_eq!(lcd.name(), "4316");
    }
}
