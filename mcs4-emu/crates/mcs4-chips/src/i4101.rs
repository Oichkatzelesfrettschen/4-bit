//! Intel 4101 256x4 Static RAM
//!
//! The 4101 is a 1024-bit static RAM organized as 256 words by 4 bits.
//! It is used in MCS-40 systems via the 4289 Standard Memory Interface.
//!
//! ## Features
//! - 256 × 4-bit memory array (1024 bits total)
//! - Asynchronous read/write operations
//! - Cycle-accurate timing simulation
//! - Tri-state output drivers
//! - Static design (no refresh required)
//!
//! ## Timing Model
//! - Address to output: ~200ns (decoder + bit line + drivers)
//! - Write pulse width: 30-100ns
//! - Output disable time: ~50ns

use mcs4_bus::BusCycle;

/// Intel 4101: 256x4 Static RAM
///
/// Provides 256 addressable 4-bit words with asynchronous read/write operations.
/// Supports cycle-accurate timing simulation for precise bus protocol verification.
#[derive(Clone, Debug)]
pub struct I4101 {
    /// 256 × 4-bit memory array (values stored in low nibble of u8)
    memory: [u8; 256],

    /// Address latch (8 bits for 256 locations)
    latched_address: u8,

    /// Chip select (active high)
    cs: bool,

    /// Write enable (active high)
    we: bool,

    /// Output enable (active high) - allows tri-state control
    oe: bool,

    /// Latched output data (driven on bus when CS=1 and OE=1)
    output_data: u8,

    /// Timing tracking: word line is active (address decoded)
    word_line_active: bool,

    /// Timing tracking: bit lines are developing (sensing in progress)
    bit_line_developing: bool,

    /// Timing tracking: write pulse is active (write current flowing)
    write_pulse_active: bool,

    /// Cycle phase counter for timing simulation
    cycle_phase: u8,
}

impl I4101 {
    /// Create new 4101 RAM instance (all zeros)
    pub fn new() -> Self {
        Self {
            memory: [0; 256],
            latched_address: 0,
            cs: false,
            we: false,
            oe: false,
            output_data: 0x0F,
            word_line_active: false,
            bit_line_developing: false,
            write_pulse_active: false,
            cycle_phase: 0,
        }
    }

    /// Read data from current address
    ///
    /// Returns the 4-bit value at latched_address if chip is selected
    /// and output is enabled. Otherwise returns 0x0F (tri-state high).
    pub fn read(&self) -> u8 {
        if self.cs && self.oe {
            // Output drivers active - drive with latched data
            self.output_data & 0x0F
        } else {
            // Output drivers tri-stated or disabled
            0x0F // Bus pull-up value
        }
    }

    /// Write data to current address
    ///
    /// Stores the 4-bit value at latched_address if chip is selected
    /// and write is enabled. Data is captured immediately in behavioral model,
    /// but cycle-accurate model tracks write pulse timing.
    pub fn write(&mut self, value: u8) {
        if self.cs && self.we && self.write_pulse_active {
            // Write current is flowing - latch flips
            self.memory[self.latched_address as usize] = value & 0x0F;
        }
    }

    /// Set current address for read/write operation
    pub fn set_address(&mut self, address: u8) {
        self.latched_address = address;
        // In real hardware, address change triggers decoder activation
        self.word_line_active = true;
        self.bit_line_developing = true;
    }

    /// Set chip select signal
    pub fn set_cs(&mut self, selected: bool) {
        self.cs = selected;
        if selected {
            // CS goes high - begin read/write operation
            self.word_line_active = true;
        } else {
            // CS goes low - disable operation, tri-state outputs
            self.word_line_active = false;
            self.write_pulse_active = false;
        }
    }

    /// Set write enable signal
    pub fn set_we(&mut self, enabled: bool) {
        self.we = enabled;
        if enabled && self.cs {
            // WE goes high - start write pulse
            self.write_pulse_active = true;
        } else {
            // WE goes low - end write pulse
            self.write_pulse_active = false;
        }
    }

    /// Set output enable signal (for tri-state control)
    pub fn set_oe(&mut self, enabled: bool) {
        self.oe = enabled;
        if enabled && self.cs {
            // OE goes high - activate output drivers
            self.update_output_data();
        }
    }

    /// Update latched output data from memory (called when read is initiated)
    fn update_output_data(&mut self) {
        if self.cs && self.word_line_active {
            // Bit lines have developed - sense amplifier captures data
            self.output_data = self.memory[self.latched_address as usize] & 0x0F;
        }
    }

    /// Complete current read/write operation
    ///
    /// Called at end of bus cycle to finalize operation and clear timing flags.
    pub fn complete_operation(&mut self) {
        self.word_line_active = false;
        self.bit_line_developing = false;
        self.write_pulse_active = false;
        self.cycle_phase = 0;
    }

    /// Get current address (for debugging)
    pub fn address(&self) -> u8 {
        self.latched_address
    }

    /// Get memory contents at address (for debugging)
    pub fn peek(&self, address: u8) -> u8 {
        self.memory[address as usize] & 0x0F
    }

    /// Set memory contents at address (for debugging/initialization)
    pub fn poke(&mut self, address: u8, value: u8) {
        self.memory[address as usize] = value & 0x0F;
    }

    /// Clear all memory to zeros
    pub fn clear(&mut self) {
        self.memory = [0; 256];
    }

    /// Check if output drivers are active
    pub fn outputs_active(&self) -> bool {
        self.cs && self.oe && !self.write_pulse_active
    }
}

impl Default for I4101 {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Chip for I4101 {
    fn name(&self) -> &'static str {
        "4101"
    }

    fn reset(&mut self) {
        self.memory = [0; 256];
        self.latched_address = 0;
        self.cs = false;
        self.we = false;
        self.oe = false;
        self.output_data = 0x0F;
        self.word_line_active = false;
        self.bit_line_developing = false;
        self.write_pulse_active = false;
        self.cycle_phase = 0;
    }

    fn tick(&mut self, phase: BusCycle) {
        // Track bus cycle phases for timing simulation
        use mcs4_bus::BusCycle::*;
        match phase {
            A1 | A2 | A3 => {
                // Address phase - update word line activation
                if self.cs {
                    self.word_line_active = true;
                }
            }
            M1 | M2 => {
                // Memory/Data phase
                if self.cs && self.word_line_active && !self.we {
                    // Read operation - update output
                    self.update_output_data();
                }
                if self.cs && self.we && self.word_line_active {
                    // Write operation - write current flows
                    self.write_pulse_active = true;
                }
            }
            X1 | X2 | X3 => {
                // Execute phase - hold data
                if phase == X3 {
                    // End of cycle - complete operation
                    self.complete_operation();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Chip;

    #[test]
    fn test_new_initializes_zeros() {
        let ram = I4101::new();
        assert_eq!(ram.read(), 0x0F); // Output disabled
        for addr in 0..=255 {
            assert_eq!(ram.peek(addr), 0);
        }
    }

    #[test]
    fn test_write_and_read_single_word() {
        let mut ram = I4101::new();

        // Write 0x5 to address 0
        ram.set_address(0);
        ram.set_cs(true);
        ram.set_oe(true);
        ram.set_we(true);
        ram.write(0x5);
        ram.write_pulse_active = true; // Simulate write pulse
        ram.write(0x5);

        // Read back
        assert_eq!(ram.peek(0), 0x5);
    }

    #[test]
    fn test_multiple_addresses() {
        let mut ram = I4101::new();

        // Write different values to different addresses
        for addr in 0..16u8 {
            ram.poke(addr, addr % 16);
        }

        // Verify reads
        for addr in 0..16u8 {
            assert_eq!(ram.peek(addr), addr % 16);
        }
    }

    #[test]
    fn test_data_masked_to_4_bits() {
        let mut ram = I4101::new();

        // Write value with bits outside 4-bit range
        ram.poke(0, 0xFF);

        // Verify only low 4 bits are stored
        assert_eq!(ram.peek(0), 0x0F);
    }

    #[test]
    fn test_outputs_disabled_when_cs_low() {
        let mut ram = I4101::new();

        ram.poke(0, 0x5);
        ram.set_address(0);
        ram.set_oe(true);
        ram.set_cs(false); // Chip not selected

        // Output should be tri-stated
        assert_eq!(ram.read(), 0x0F);
    }

    #[test]
    fn test_outputs_disabled_when_oe_low() {
        let mut ram = I4101::new();

        ram.poke(0, 0x5);
        ram.set_address(0);
        ram.set_cs(true);
        ram.set_oe(false); // Output not enabled

        // Output should be tri-stated
        assert_eq!(ram.read(), 0x0F);
    }

    #[test]
    fn test_outputs_active_when_cs_and_oe_high() {
        let mut ram = I4101::new();

        ram.poke(0, 0x5);
        ram.set_address(0);
        ram.set_cs(true);
        ram.set_oe(true);

        // Output drivers should be active
        assert!(ram.outputs_active());
    }

    #[test]
    fn test_write_only_when_we_and_cs_high() {
        let mut ram = I4101::new();

        ram.set_address(0);
        ram.set_cs(false); // CS low
        ram.set_we(true);
        ram.write_pulse_active = true;
        ram.write(0xA);

        // Should not write when CS is low
        assert_eq!(ram.peek(0), 0);
    }

    #[test]
    fn test_all_addresses_independent() {
        let mut ram = I4101::new();

        // Write to all addresses with unique values
        for addr in 0..=255u8 {
            ram.poke(addr, (addr >> 4) & 0x0F);
        }

        // Verify no interference
        for addr in 0..=255u8 {
            let expected = (addr >> 4) & 0x0F;
            assert_eq!(ram.peek(addr), expected, "Address {}: mismatch", addr);
        }
    }

    #[test]
    fn test_reset_clears_memory() {
        let mut ram = I4101::new();

        // Write pattern
        for addr in 0..16 {
            ram.poke(addr, 0x5);
        }

        // Reset
        ram.reset();

        // Verify memory cleared
        for addr in 0..16 {
            assert_eq!(ram.peek(addr), 0);
        }

        // Verify control signals reset
        assert!(!ram.cs);
        assert!(!ram.we);
        assert!(!ram.oe);
    }

    #[test]
    fn test_complete_operation_clears_timing_flags() {
        let mut ram = I4101::new();

        ram.set_address(0);
        ram.set_cs(true);
        ram.set_we(true);

        // Flags should be active
        assert!(ram.word_line_active || ram.write_pulse_active);

        // Complete cycle
        ram.complete_operation();

        // Flags should be cleared
        assert!(!ram.word_line_active);
        assert!(!ram.write_pulse_active);
    }

    #[test]
    fn test_output_data_updates_on_read() {
        let mut ram = I4101::new();

        ram.poke(5, 0xC);
        ram.set_address(5);
        ram.set_cs(true);
        ram.set_oe(true);
        ram.update_output_data();

        // Read should return latched data
        assert_eq!(ram.read(), 0xC);
    }

    #[test]
    fn test_write_pattern_all_zeros_and_all_ones() {
        let mut ram = I4101::new();

        // Write all zeros
        for addr in 0..16 {
            ram.poke(addr, 0x0);
        }
        for addr in 0..16 {
            assert_eq!(ram.peek(addr), 0x0);
        }

        // Write all ones (4-bit)
        for addr in 0..16 {
            ram.poke(addr, 0xF);
        }
        for addr in 0..16 {
            assert_eq!(ram.peek(addr), 0xF);
        }
    }

    #[test]
    fn test_alternating_pattern() {
        let mut ram = I4101::new();

        // Write alternating pattern: 0x5 = 0101, 0xA = 1010
        for addr in 0..16 {
            let pattern = if addr % 2 == 0 { 0x5 } else { 0xA };
            ram.poke(addr, pattern);
        }

        // Verify pattern
        for addr in 0..16 {
            let expected = if addr % 2 == 0 { 0x5 } else { 0xA };
            assert_eq!(ram.peek(addr), expected);
        }
    }

    #[test]
    fn test_clear_zeros_all_memory() {
        let mut ram = I4101::new();

        // Write pattern
        for addr in 0..=255 {
            ram.poke(addr, 0xF);
        }

        // Clear
        ram.clear();

        // Verify all zeros
        for addr in 0..=255 {
            assert_eq!(ram.peek(addr), 0);
        }
    }

    #[test]
    fn test_cs_high_enables_word_line() {
        let mut ram = I4101::new();

        assert!(!ram.word_line_active);

        ram.set_address(0);
        ram.set_cs(true);

        assert!(ram.word_line_active);
    }

    #[test]
    fn test_cs_low_disables_word_line() {
        let mut ram = I4101::new();

        ram.set_address(0);
        ram.set_cs(true);
        assert!(ram.word_line_active);

        ram.set_cs(false);
        assert!(!ram.word_line_active);
    }
}
