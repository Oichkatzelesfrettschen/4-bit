//! Matrix Keyboard Scanner
//!
//! WHY: The Intellec-4 and many MCS-4 systems used 4x4 or larger matrix
//! keyboards for hex data entry. The keyboard is scanned by driving
//! rows via ROM I/O output (WRR) and reading columns via ROM I/O
//! input (RDR).
//!
//! WHAT: A configurable matrix keyboard (up to 4 rows x 4 columns for
//! a single ROM I/O port) with debounce, key state tracking, and
//! event generation.
//!
//! HOW: The CPU writes a row select pattern to the ROM output port
//! (one-hot encoding). The keyboard returns the column state on the
//! ROM input port. Software scans all rows to detect key presses.
//! This module provides the physical keyboard state and the
//! electrical interface to the ROM I/O port.

/// Key state for debounce tracking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyState {
    /// Key is not pressed.
    Released,
    /// Key is pressed.
    Pressed,
}

/// A key event produced by the scanner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    /// Row index (0-3)
    pub row: u8,
    /// Column index (0-3)
    pub col: u8,
    /// Key state (pressed or released)
    pub state: KeyState,
}

/// 4x4 matrix keyboard with debounce.
///
/// Physical layout (hex keypad):
/// ```text
///        Col 0  Col 1  Col 2  Col 3
/// Row 0:  0      1      2      3
/// Row 1:  4      5      6      7
/// Row 2:  8      9      A      B
/// Row 3:  C      D      E      F
/// ```
///
/// Electrical interface:
/// - Row drive: 4-bit output from ROM port (WRR), active-low one-hot
/// - Column read: 4-bit input to ROM port (RDR), active-low when pressed
#[derive(Clone, Debug)]
pub struct MatrixKeyboard {
    /// Key state matrix [row][col]: true = physically pressed
    keys: [[bool; 4]; 4],

    /// Debounced key state [row][col]
    debounced: [[KeyState; 4]; 4],

    /// Debounce counter per key (counts consecutive same-state scans)
    debounce_count: [[u8; 4]; 4],

    /// Number of consecutive matching scans required for debounce
    debounce_threshold: u8,

    /// Currently driven row pattern (active-low one-hot from ROM WRR)
    row_drive: u8,

    /// Pending key events
    events: Vec<KeyEvent>,
}

impl MatrixKeyboard {
    /// Create a new keyboard with the given debounce threshold.
    ///
    /// `debounce_scans` is the number of consecutive matching scans
    /// before a key state change is accepted. Use 1 for no debounce
    /// (useful in tests), or 3-5 for realistic debounce.
    pub fn new(debounce_scans: u8) -> Self {
        Self {
            keys: [[false; 4]; 4],
            debounced: [[KeyState::Released; 4]; 4],
            debounce_count: [[0; 4]; 4],
            debounce_threshold: debounce_scans.max(1),
            row_drive: 0x0F, // all rows inactive (active-low: 1 = inactive)
            events: Vec::new(),
        }
    }

    /// Simulate a physical key press.
    pub fn press(&mut self, row: u8, col: u8) {
        if row < 4 && col < 4 {
            self.keys[row as usize][col as usize] = true;
        }
    }

    /// Simulate a physical key release.
    pub fn release(&mut self, row: u8, col: u8) {
        if row < 4 && col < 4 {
            self.keys[row as usize][col as usize] = false;
        }
    }

    /// Check if a key is physically pressed (before debounce).
    pub fn is_pressed(&self, row: u8, col: u8) -> bool {
        if row < 4 && col < 4 {
            self.keys[row as usize][col as usize]
        } else {
            false
        }
    }

    /// Check the debounced state of a key.
    pub fn debounced_state(&self, row: u8, col: u8) -> KeyState {
        if row < 4 && col < 4 {
            self.debounced[row as usize][col as usize]
        } else {
            KeyState::Released
        }
    }

    /// Set the row drive pattern from ROM output port (WRR).
    ///
    /// Active-low one-hot: bit 0 = row 0, etc.
    /// A low bit means that row is being driven (scanned).
    pub fn set_row_drive(&mut self, pattern: u8) {
        self.row_drive = pattern & 0x0F;
    }

    /// Read the column state for ROM input port (RDR).
    ///
    /// Returns 4-bit active-low column state for the currently
    /// driven row(s). If no row is driven (all bits high), returns 0xF.
    /// If a key is pressed at the intersection of a driven row and a
    /// column, that column bit goes low (0).
    pub fn read_columns(&self) -> u8 {
        let mut cols: u8 = 0x0F; // all high = no key pressed

        for row in 0..4u8 {
            // Check if this row is being driven (active-low)
            if self.row_drive & (1 << row) == 0 {
                for col in 0..4u8 {
                    if self.keys[row as usize][col as usize] {
                        // Key pressed: pull column low
                        cols &= !(1 << col);
                    }
                }
            }
        }
        cols
    }

    /// Run one debounce scan cycle.
    ///
    /// Call this once per keyboard scan iteration (typically after
    /// scanning all rows). Updates debounced state and generates
    /// key events for state transitions.
    pub fn scan_debounce(&mut self) {
        for row in 0..4u8 {
            for col in 0..4u8 {
                let r = row as usize;
                let c = col as usize;
                let physical = self.keys[r][c];
                let target = if physical {
                    KeyState::Pressed
                } else {
                    KeyState::Released
                };

                if target == self.debounced[r][c] {
                    // State matches; reset counter
                    self.debounce_count[r][c] = 0;
                } else {
                    // State differs; increment counter
                    self.debounce_count[r][c] += 1;
                    if self.debounce_count[r][c] >= self.debounce_threshold {
                        // Debounce threshold reached; accept new state
                        self.debounced[r][c] = target;
                        self.debounce_count[r][c] = 0;
                        self.events.push(KeyEvent {
                            row,
                            col,
                            state: target,
                        });
                    }
                }
            }
        }
    }

    /// Drain all pending key events.
    pub fn drain_events(&mut self) -> Vec<KeyEvent> {
        std::mem::take(&mut self.events)
    }

    /// Get the hex keycode (0-15) for a row/col position.
    ///
    /// Uses the standard Intellec-4 hex keypad layout:
    /// row 0: 0-3, row 1: 4-7, row 2: 8-B, row 3: C-F
    pub fn keycode(row: u8, col: u8) -> Option<u8> {
        if row < 4 && col < 4 {
            Some(row * 4 + col)
        } else {
            None
        }
    }

    /// Reset the keyboard: release all keys, clear debounce state.
    pub fn reset(&mut self) {
        self.keys = [[false; 4]; 4];
        self.debounced = [[KeyState::Released; 4]; 4];
        self.debounce_count = [[0; 4]; 4];
        self.row_drive = 0x0F;
        self.events.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_keyboard_no_keys_pressed() {
        let kb = MatrixKeyboard::new(1);
        for row in 0..4 {
            for col in 0..4 {
                assert!(!kb.is_pressed(row, col));
                assert_eq!(kb.debounced_state(row, col), KeyState::Released);
            }
        }
    }

    #[test]
    fn press_and_read_column() {
        let mut kb = MatrixKeyboard::new(1);
        kb.press(1, 2); // Row 1, Col 2

        // Drive row 1 (active-low: bit 1 = 0, others = 1)
        kb.set_row_drive(0b1101);
        let cols = kb.read_columns();
        // Col 2 should be low (bit 2 = 0)
        assert_eq!(cols, 0b1011);

        // Drive row 0 (no key pressed there)
        kb.set_row_drive(0b1110);
        assert_eq!(kb.read_columns(), 0x0F);
    }

    #[test]
    fn no_row_driven_returns_all_high() {
        let mut kb = MatrixKeyboard::new(1);
        kb.press(0, 0);
        kb.set_row_drive(0x0F); // all inactive
        assert_eq!(kb.read_columns(), 0x0F);
    }

    #[test]
    fn multiple_keys_same_row() {
        let mut kb = MatrixKeyboard::new(1);
        kb.press(2, 0);
        kb.press(2, 3);

        kb.set_row_drive(0b1011); // drive row 2
        let cols = kb.read_columns();
        // Cols 0 and 3 should be low
        assert_eq!(cols, 0b0110);
    }

    #[test]
    fn debounce_requires_threshold_scans() {
        let mut kb = MatrixKeyboard::new(3);
        kb.press(0, 0);

        // First two scans: not yet debounced
        kb.scan_debounce();
        assert_eq!(kb.debounced_state(0, 0), KeyState::Released);
        kb.scan_debounce();
        assert_eq!(kb.debounced_state(0, 0), KeyState::Released);

        // Third scan: debounce completes
        kb.scan_debounce();
        assert_eq!(kb.debounced_state(0, 0), KeyState::Pressed);
    }

    #[test]
    fn debounce_generates_events() {
        let mut kb = MatrixKeyboard::new(1);
        kb.press(1, 3);
        kb.scan_debounce();

        let events = kb.drain_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].row, 1);
        assert_eq!(events[0].col, 3);
        assert_eq!(events[0].state, KeyState::Pressed);

        // Release
        kb.release(1, 3);
        kb.scan_debounce();
        let events = kb.drain_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].state, KeyState::Released);
    }

    #[test]
    fn keycode_mapping() {
        assert_eq!(MatrixKeyboard::keycode(0, 0), Some(0));
        assert_eq!(MatrixKeyboard::keycode(0, 3), Some(3));
        assert_eq!(MatrixKeyboard::keycode(3, 3), Some(15));
        assert_eq!(MatrixKeyboard::keycode(2, 1), Some(9));
        assert_eq!(MatrixKeyboard::keycode(4, 0), None);
    }

    #[test]
    fn reset_clears_all_state() {
        let mut kb = MatrixKeyboard::new(1);
        kb.press(0, 0);
        kb.press(1, 1);
        kb.scan_debounce();
        assert_eq!(kb.debounced_state(0, 0), KeyState::Pressed);

        kb.reset();
        assert!(!kb.is_pressed(0, 0));
        assert_eq!(kb.debounced_state(0, 0), KeyState::Released);
        assert!(kb.drain_events().is_empty());
    }

    #[test]
    fn scan_multiple_rows_sequentially() {
        let mut kb = MatrixKeyboard::new(1);
        kb.press(0, 1); // key at row 0, col 1
        kb.press(3, 0); // key at row 3, col 0

        // Scan row 0
        kb.set_row_drive(0b1110);
        assert_eq!(kb.read_columns(), 0b1101); // col 1 low

        // Scan row 1
        kb.set_row_drive(0b1101);
        assert_eq!(kb.read_columns(), 0x0F); // nothing

        // Scan row 2
        kb.set_row_drive(0b1011);
        assert_eq!(kb.read_columns(), 0x0F); // nothing

        // Scan row 3
        kb.set_row_drive(0b0111);
        assert_eq!(kb.read_columns(), 0b1110); // col 0 low
    }

    #[test]
    fn out_of_bounds_access_is_safe() {
        let mut kb = MatrixKeyboard::new(1);
        kb.press(5, 5); // out of bounds, no-op
        assert!(!kb.is_pressed(5, 5));
        assert_eq!(kb.debounced_state(4, 4), KeyState::Released);
    }
}
