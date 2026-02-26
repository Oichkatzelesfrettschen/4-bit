//! 7-Segment LED Display Driver
//!
//! WHY: MCS-4 systems commonly drove 7-segment LED displays through 4003
//! shift register cascades. The CPU writes serial data via WMP to the
//! 4002 output port, which clocks the 4003 shift register chain. The
//! 4003 parallel outputs drive individual LED segments.
//!
//! WHAT: Multi-digit 7-segment display controller that decodes shift
//! register output into human-readable digit representations. Supports
//! raw segment control (a-g + dp) and BCD-to-segment decode.
//!
//! HOW: Each digit uses 8 bits from the shift register chain (7 segments
//! plus decimal point). For N digits, N*8 bits are shifted through cascaded
//! 4003 chips. Each 4003 holds 10 bits, so 2 digits per chip with 4 spare
//! bits, or they can be packed tightly depending on wiring.

/// Segment bit positions within an 8-bit segment byte.
///
/// Standard segment labeling:
/// ```text
///    aaa
///   f   b
///   f   b
///    ggg
///   e   c
///   e   c
///    ddd  .dp
/// ```
pub mod segment {
    /// Top horizontal segment.
    pub const A: u8 = 0x01;
    /// Upper-right vertical segment.
    pub const B: u8 = 0x02;
    /// Lower-right vertical segment.
    pub const C: u8 = 0x04;
    /// Bottom horizontal segment.
    pub const D: u8 = 0x08;
    /// Lower-left vertical segment.
    pub const E: u8 = 0x10;
    /// Upper-left vertical segment.
    pub const F: u8 = 0x20;
    /// Middle horizontal segment.
    pub const G: u8 = 0x40;
    /// Decimal point.
    pub const DP: u8 = 0x80;
}

/// BCD to 7-segment decode table (0-F, no decimal point).
const BCD_TO_SEGMENTS: [u8; 16] = [
    // 0: a b c d e f
    segment::A | segment::B | segment::C | segment::D | segment::E | segment::F,
    // 1: b c
    segment::B | segment::C,
    // 2: a b d e g
    segment::A | segment::B | segment::D | segment::E | segment::G,
    // 3: a b c d g
    segment::A | segment::B | segment::C | segment::D | segment::G,
    // 4: b c f g
    segment::B | segment::C | segment::F | segment::G,
    // 5: a c d f g
    segment::A | segment::C | segment::D | segment::F | segment::G,
    // 6: a c d e f g
    segment::A | segment::C | segment::D | segment::E | segment::F | segment::G,
    // 7: a b c
    segment::A | segment::B | segment::C,
    // 8: a b c d e f g
    segment::A | segment::B | segment::C | segment::D | segment::E | segment::F | segment::G,
    // 9: a b c d f g
    segment::A | segment::B | segment::C | segment::D | segment::F | segment::G,
    // A: a b c e f g
    segment::A | segment::B | segment::C | segment::E | segment::F | segment::G,
    // b: c d e f g
    segment::C | segment::D | segment::E | segment::F | segment::G,
    // C: a d e f
    segment::A | segment::D | segment::E | segment::F,
    // d: b c d e g
    segment::B | segment::C | segment::D | segment::E | segment::G,
    // E: a d e f g
    segment::A | segment::D | segment::E | segment::F | segment::G,
    // F: a e f g
    segment::A | segment::E | segment::F | segment::G,
];

/// Decode a BCD nibble (0-15) to 7-segment pattern.
pub fn bcd_to_segments(nibble: u8) -> u8 {
    BCD_TO_SEGMENTS[(nibble & 0x0F) as usize]
}

/// Multi-digit 7-segment display driven by shift register data.
///
/// Each digit occupies 8 bits of the shift register chain:
/// bits [0..6] = segments a-g, bit 7 = decimal point.
///
/// Digit ordering: digit 0 is the rightmost (least significant)
/// position in the shift chain. Digit N-1 is leftmost.
#[derive(Clone, Debug)]
pub struct SevenSegDisplay {
    /// Per-digit raw segment data (8 bits each: segments a-g + dp).
    digits: Vec<u8>,

    /// Whether BCD decode mode is active.
    /// When true, only the lower 4 bits of each digit byte are used
    /// as a BCD value, and the display decodes them to segments.
    bcd_mode: bool,
}

impl SevenSegDisplay {
    /// Create a display with `num_digits` digits, all blank.
    pub fn new(num_digits: usize) -> Self {
        Self {
            digits: vec![0; num_digits],
            bcd_mode: false,
        }
    }

    /// Number of digits in the display.
    pub fn num_digits(&self) -> usize {
        self.digits.len()
    }

    /// Enable or disable BCD decode mode.
    pub fn set_bcd_mode(&mut self, enabled: bool) {
        self.bcd_mode = enabled;
    }

    /// Whether BCD decode mode is active.
    pub fn bcd_mode(&self) -> bool {
        self.bcd_mode
    }

    /// Load segment data from a shift register chain's parallel output.
    ///
    /// `shift_data` contains the raw bits from cascaded 4003 chips.
    /// Each digit takes 8 consecutive bits starting from bit 0.
    /// Only `num_digits * 8` bits are consumed.
    pub fn load_from_shift_chain(&mut self, shift_data: &[u16]) {
        // Concatenate all shift register outputs into a bit stream.
        // Each 4003 contributes 10 bits; we pack them sequentially.
        let mut bits: u64 = 0;
        let mut total_bits: usize = 0;
        for &word in shift_data {
            bits |= (word as u64 & 0x3FF) << total_bits;
            total_bits += 10;
            if total_bits >= 64 {
                break;
            }
        }

        // Extract 8 bits per digit
        for (i, digit) in self.digits.iter_mut().enumerate() {
            let offset = i * 8;
            if offset + 8 <= total_bits {
                *digit = ((bits >> offset) & 0xFF) as u8;
            }
        }
    }

    /// Directly set raw segment data for a digit.
    pub fn set_raw(&mut self, digit_idx: usize, segments: u8) {
        if digit_idx < self.digits.len() {
            self.digits[digit_idx] = segments;
        }
    }

    /// Directly set a BCD value for a digit (with optional decimal point).
    pub fn set_bcd(&mut self, digit_idx: usize, value: u8, dp: bool) {
        if digit_idx < self.digits.len() {
            let segs = bcd_to_segments(value);
            self.digits[digit_idx] = segs | if dp { segment::DP } else { 0 };
        }
    }

    /// Get the active segment pattern for a digit.
    ///
    /// In BCD mode, decodes the lower nibble. In raw mode, returns
    /// the raw segment byte.
    pub fn segments(&self, digit_idx: usize) -> u8 {
        if digit_idx >= self.digits.len() {
            return 0;
        }
        let raw = self.digits[digit_idx];
        if self.bcd_mode {
            let dp = raw & segment::DP;
            bcd_to_segments(raw & 0x0F) | dp
        } else {
            raw
        }
    }

    /// Get raw stored byte for a digit (before decode).
    pub fn raw(&self, digit_idx: usize) -> u8 {
        if digit_idx >= self.digits.len() {
            return 0;
        }
        self.digits[digit_idx]
    }

    /// Check if a specific segment is lit on a digit.
    pub fn segment_lit(&self, digit_idx: usize, seg: u8) -> bool {
        self.segments(digit_idx) & seg != 0
    }

    /// Render the display as an ASCII string (one char per digit).
    ///
    /// Returns characters '0'-'9','A'-'F' for recognized patterns,
    /// ' ' for blank, '-' for just segment G, '_' for just segment D,
    /// and '?' for unrecognized patterns.
    pub fn render_ascii(&self) -> String {
        let mut s = String::with_capacity(self.digits.len());
        for i in (0..self.digits.len()).rev() {
            let segs = self.segments(i) & !segment::DP;
            let ch = match segs {
                0 => ' ',
                s if s == BCD_TO_SEGMENTS[0] => '0',
                s if s == BCD_TO_SEGMENTS[1] => '1',
                s if s == BCD_TO_SEGMENTS[2] => '2',
                s if s == BCD_TO_SEGMENTS[3] => '3',
                s if s == BCD_TO_SEGMENTS[4] => '4',
                s if s == BCD_TO_SEGMENTS[5] => '5',
                s if s == BCD_TO_SEGMENTS[6] => '6',
                s if s == BCD_TO_SEGMENTS[7] => '7',
                s if s == BCD_TO_SEGMENTS[8] => '8',
                s if s == BCD_TO_SEGMENTS[9] => '9',
                s if s == BCD_TO_SEGMENTS[0xA] => 'A',
                s if s == BCD_TO_SEGMENTS[0xB] => 'b',
                s if s == BCD_TO_SEGMENTS[0xC] => 'C',
                s if s == BCD_TO_SEGMENTS[0xD] => 'd',
                s if s == BCD_TO_SEGMENTS[0xE] => 'E',
                s if s == BCD_TO_SEGMENTS[0xF] => 'F',
                s if s == segment::G => '-',
                s if s == segment::D => '_',
                _ => '?',
            };
            s.push(ch);
        }
        s
    }

    /// Reset all digits to blank.
    pub fn clear(&mut self) {
        for d in &mut self.digits {
            *d = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bcd_decode_all_digits() {
        // Verify BCD decode for 0-9 produces expected segment patterns
        assert_eq!(bcd_to_segments(0), 0x3F); // a+b+c+d+e+f
        assert_eq!(bcd_to_segments(1), 0x06); // b+c
        assert_eq!(bcd_to_segments(8), 0x7F); // all segments
        assert_eq!(bcd_to_segments(0xF), 0x71); // a+e+f+g
    }

    #[test]
    fn new_display_is_blank() {
        let disp = SevenSegDisplay::new(4);
        assert_eq!(disp.num_digits(), 4);
        for i in 0..4 {
            assert_eq!(disp.segments(i), 0);
            assert_eq!(disp.raw(i), 0);
        }
        assert_eq!(disp.render_ascii(), "    ");
    }

    #[test]
    fn set_raw_and_read_back() {
        let mut disp = SevenSegDisplay::new(2);
        disp.set_raw(0, segment::A | segment::B | segment::C);
        assert_eq!(disp.segments(0), 0x07);
        assert!(disp.segment_lit(0, segment::A));
        assert!(disp.segment_lit(0, segment::B));
        assert!(!disp.segment_lit(0, segment::D));
    }

    #[test]
    fn set_bcd_with_decimal_point() {
        let mut disp = SevenSegDisplay::new(3);
        disp.set_bcd(1, 5, true);
        // Segment pattern for '5' + DP
        let expected = bcd_to_segments(5) | segment::DP;
        assert_eq!(disp.segments(1), expected);
        assert!(disp.segment_lit(1, segment::DP));
    }

    #[test]
    fn bcd_mode_decodes_stored_nibble() {
        let mut disp = SevenSegDisplay::new(2);
        disp.set_bcd_mode(true);
        // Store raw nibble value 3 (not the segment pattern for 3)
        disp.set_raw(0, 3);
        // In BCD mode, segments() decodes the nibble
        assert_eq!(disp.segments(0), bcd_to_segments(3));
    }

    #[test]
    fn render_ascii_shows_digits() {
        let mut disp = SevenSegDisplay::new(4);
        disp.set_bcd(0, 2, false);
        disp.set_bcd(1, 0, false);
        disp.set_bcd(2, 4, false);
        disp.set_bcd(3, 1, false);
        // Render is MSB-first (digit 3 leftmost)
        assert_eq!(disp.render_ascii(), "1402");
    }

    #[test]
    fn load_from_single_shift_register() {
        let mut disp = SevenSegDisplay::new(1);
        // 4003 has 10 bits; we use lower 8 for one digit
        let shift_data = [bcd_to_segments(7) as u16]; // '7' pattern in bits 0-7
        disp.load_from_shift_chain(&shift_data);
        assert_eq!(disp.render_ascii(), "7");
    }

    #[test]
    fn load_from_cascaded_shift_registers() {
        let mut disp = SevenSegDisplay::new(2);
        // Two 4003s, 10 bits each = 20 bits total
        // Digit 0 = bits 0-7 of first register
        // Digit 1 = bits 8-9 of first register + bits 0-5 of second register
        let d0_pattern = bcd_to_segments(3) as u16; // 0x4F
        let d1_pattern = bcd_to_segments(9) as u16; // 0x6F

        // First register: bits 0-7 = digit 0, bits 8-9 = low 2 bits of digit 1
        let sr0 = d0_pattern | ((d1_pattern & 0x03) << 8);
        // Second register: bits 0-5 = remaining 6 bits of digit 1
        let sr1 = (d1_pattern >> 2) & 0x3F;

        disp.load_from_shift_chain(&[sr0, sr1]);
        assert_eq!(disp.segments(0), bcd_to_segments(3));
        assert_eq!(disp.segments(1), bcd_to_segments(9));
    }

    #[test]
    fn clear_resets_all_digits() {
        let mut disp = SevenSegDisplay::new(3);
        disp.set_bcd(0, 8, true);
        disp.set_bcd(1, 5, false);
        disp.set_bcd(2, 1, false);
        disp.clear();
        assert_eq!(disp.render_ascii(), "   ");
    }

    #[test]
    fn out_of_bounds_digit_returns_zero() {
        let disp = SevenSegDisplay::new(2);
        assert_eq!(disp.segments(5), 0);
        assert_eq!(disp.raw(99), 0);
        assert!(!disp.segment_lit(10, segment::A));
    }

    #[test]
    fn render_special_patterns() {
        let mut disp = SevenSegDisplay::new(3);
        disp.set_raw(0, segment::G); // minus sign
        disp.set_raw(1, segment::D); // underscore
        disp.set_raw(2, 0); // blank
        assert_eq!(disp.render_ascii(), " _-");
    }
}
