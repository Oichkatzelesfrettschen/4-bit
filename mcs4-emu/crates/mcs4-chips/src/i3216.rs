//! Intel 3216 Bidirectional Bus Driver
//!
//! The 3216 is a 4-bit bidirectional bus driver used in the MCS-4 system
//! to buffer and isolate the CPU data bus from peripheral buses. It provides
//! direction control (A->B or B->A) and an enable/disable function.
//!
//! In the MCS-4 system, the 3216 is used on the data bus side, providing
//! non-inverting bidirectional buffering with active-low chip select.

use mcs4_bus::BusCycle;

/// Intel 3216: 4-bit Bidirectional Bus Driver
///
/// Non-inverting driver with direction control and chip enable.
/// Port A connects to the CPU side, Port B to the peripheral side.
#[derive(Clone, Debug)]
pub struct I3216 {
    /// Port A data (CPU side)
    port_a: u8,

    /// Port B data (peripheral side)
    port_b: u8,

    /// Direction: true = A->B (CPU drives peripherals), false = B->A
    dir_a_to_b: bool,

    /// Chip select (active-low in hardware; true = enabled here)
    enabled: bool,
}

impl I3216 {
    pub fn new() -> Self {
        Self {
            port_a: 0,
            port_b: 0,
            dir_a_to_b: true,
            enabled: false,
        }
    }

    /// Set direction: true = A->B (CPU out), false = B->A (CPU in)
    pub fn set_direction(&mut self, a_to_b: bool) {
        self.dir_a_to_b = a_to_b;
    }

    /// Enable the bus driver (active-low CS in hardware)
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Drive port A (CPU side) with data
    pub fn drive_a(&mut self, data: u8) {
        self.port_a = data & 0x0F;
        if self.enabled && self.dir_a_to_b {
            self.port_b = self.port_a;
        }
    }

    /// Drive port B (peripheral side) with data
    pub fn drive_b(&mut self, data: u8) {
        self.port_b = data & 0x0F;
        if self.enabled && !self.dir_a_to_b {
            self.port_a = self.port_b;
        }
    }

    /// Read port A output
    pub fn read_a(&self) -> u8 {
        if self.enabled && !self.dir_a_to_b {
            self.port_b & 0x0F
        } else {
            self.port_a & 0x0F
        }
    }

    /// Read port B output
    pub fn read_b(&self) -> u8 {
        if self.enabled && self.dir_a_to_b {
            self.port_a & 0x0F
        } else {
            self.port_b & 0x0F
        }
    }

    /// Whether the driver is currently enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Current direction (true = A->B)
    pub fn direction_a_to_b(&self) -> bool {
        self.dir_a_to_b
    }
}

impl Default for I3216 {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Chip for I3216 {
    fn name(&self) -> &'static str {
        "3216"
    }

    fn reset(&mut self) {
        self.port_a = 0;
        self.port_b = 0;
        self.dir_a_to_b = true;
        self.enabled = false;
    }

    fn tick(&mut self, _phase: BusCycle) {
        // Direction and enable are set by system driver based on bus phase
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_to_b_transfer() {
        let mut drv = I3216::new();
        drv.set_enabled(true);
        drv.set_direction(true); // A->B

        drv.drive_a(0xA);
        assert_eq!(drv.read_b(), 0xA);
    }

    #[test]
    fn b_to_a_transfer() {
        let mut drv = I3216::new();
        drv.set_enabled(true);
        drv.set_direction(false); // B->A

        drv.drive_b(0x5);
        assert_eq!(drv.read_a(), 0x5);
    }

    #[test]
    fn disabled_does_not_transfer() {
        let mut drv = I3216::new();
        drv.set_enabled(false);
        drv.set_direction(true); // A->B

        drv.drive_a(0xF);
        // Port B should not be updated when disabled
        assert_eq!(drv.port_b, 0);
    }

    #[test]
    fn data_masked_to_4_bits() {
        let mut drv = I3216::new();
        drv.set_enabled(true);
        drv.set_direction(true);

        drv.drive_a(0xFF);
        assert_eq!(drv.read_b(), 0x0F);
    }

    #[test]
    fn direction_change() {
        let mut drv = I3216::new();
        drv.set_enabled(true);

        // A->B
        drv.set_direction(true);
        drv.drive_a(0x3);
        assert_eq!(drv.read_b(), 0x3);

        // Switch to B->A
        drv.set_direction(false);
        drv.drive_b(0xC);
        assert_eq!(drv.read_a(), 0xC);
    }

    #[test]
    fn reset_clears_state() {
        use crate::Chip;
        let mut drv = I3216::new();
        drv.set_enabled(true);
        drv.drive_a(0xF);

        drv.reset();

        assert_eq!(drv.read_a(), 0);
        assert_eq!(drv.read_b(), 0);
        assert!(!drv.is_enabled());
    }

    #[test]
    fn chip_trait_name() {
        use crate::Chip;
        let drv = I3216::new();
        assert_eq!(drv.name(), "3216");
    }

    #[test]
    fn enable_disable_toggle() {
        let mut drv = I3216::new();
        assert!(!drv.is_enabled());

        drv.set_enabled(true);
        assert!(drv.is_enabled());

        drv.set_enabled(false);
        assert!(!drv.is_enabled());
    }
}
