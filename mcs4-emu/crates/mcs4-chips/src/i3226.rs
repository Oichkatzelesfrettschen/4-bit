//! Intel 3226 Bidirectional Bus Driver
//!
//! The 3226 is functionally similar to the 3216 but provides an
//! inverted output configuration. In the MCS-4 system, the 3226 is
//! used where complementary bus driving is needed.
//!
//! The key difference from the 3216: the 3226 inverts data during
//! transfer (active-low data representation on the output side).

use mcs4_bus::BusCycle;

/// Intel 3226: 4-bit Bidirectional Bus Driver (inverting)
///
/// Inverting driver with direction control and chip enable.
/// Port A connects to the CPU side, Port B to the peripheral side.
/// Data is inverted during transfer (complement of input).
#[derive(Clone, Debug)]
pub struct I3226 {
    /// Port A data (CPU side)
    port_a: u8,

    /// Port B data (peripheral side)
    port_b: u8,

    /// Direction: true = A->B (CPU drives peripherals), false = B->A
    dir_a_to_b: bool,

    /// Chip select (active-low in hardware; true = enabled here)
    enabled: bool,
}

impl I3226 {
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

    /// Enable the bus driver
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Drive port A (CPU side) with data
    pub fn drive_a(&mut self, data: u8) {
        self.port_a = data & 0x0F;
        if self.enabled && self.dir_a_to_b {
            // Invert: complement the 4-bit data
            self.port_b = (!self.port_a) & 0x0F;
        }
    }

    /// Drive port B (peripheral side) with data
    pub fn drive_b(&mut self, data: u8) {
        self.port_b = data & 0x0F;
        if self.enabled && !self.dir_a_to_b {
            // Invert: complement the 4-bit data
            self.port_a = (!self.port_b) & 0x0F;
        }
    }

    /// Read port A output
    pub fn read_a(&self) -> u8 {
        if self.enabled && !self.dir_a_to_b {
            (!self.port_b) & 0x0F
        } else {
            self.port_a & 0x0F
        }
    }

    /// Read port B output
    pub fn read_b(&self) -> u8 {
        if self.enabled && self.dir_a_to_b {
            (!self.port_a) & 0x0F
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

impl Default for I3226 {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Chip for I3226 {
    fn name(&self) -> &'static str {
        "3226"
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
    fn a_to_b_inverts() {
        let mut drv = I3226::new();
        drv.set_enabled(true);
        drv.set_direction(true); // A->B

        drv.drive_a(0xA); // 1010
                          // Inverted: 0101 = 0x5
        assert_eq!(drv.read_b(), 0x5);
    }

    #[test]
    fn b_to_a_inverts() {
        let mut drv = I3226::new();
        drv.set_enabled(true);
        drv.set_direction(false); // B->A

        drv.drive_b(0x3); // 0011
                          // Inverted: 1100 = 0xC
        assert_eq!(drv.read_a(), 0xC);
    }

    #[test]
    fn disabled_does_not_transfer() {
        let mut drv = I3226::new();
        drv.set_enabled(false);
        drv.set_direction(true);

        drv.drive_a(0xF);
        assert_eq!(drv.port_b, 0);
    }

    #[test]
    fn double_inversion_roundtrip() {
        let mut drv1 = I3226::new();
        let mut drv2 = I3226::new();
        drv1.set_enabled(true);
        drv2.set_enabled(true);
        drv1.set_direction(true); // A->B
        drv2.set_direction(true); // A->B

        // Drive 0x7 through two inversions: 0x7 -> ~0x7=0x8 -> ~0x8=0x7
        drv1.drive_a(0x7);
        let intermediate = drv1.read_b();
        assert_eq!(intermediate, 0x8);

        drv2.drive_a(intermediate);
        assert_eq!(drv2.read_b(), 0x7);
    }

    #[test]
    fn all_zeros_inverts_to_all_ones() {
        let mut drv = I3226::new();
        drv.set_enabled(true);
        drv.set_direction(true);

        drv.drive_a(0x0);
        assert_eq!(drv.read_b(), 0xF);
    }

    #[test]
    fn all_ones_inverts_to_all_zeros() {
        let mut drv = I3226::new();
        drv.set_enabled(true);
        drv.set_direction(true);

        drv.drive_a(0xF);
        assert_eq!(drv.read_b(), 0x0);
    }

    #[test]
    fn reset_clears_state() {
        use crate::Chip;
        let mut drv = I3226::new();
        drv.set_enabled(true);
        drv.drive_a(0xF);

        drv.reset();

        assert_eq!(drv.port_a, 0);
        assert_eq!(drv.port_b, 0);
        assert!(!drv.is_enabled());
    }

    #[test]
    fn chip_trait_name() {
        use crate::Chip;
        let drv = I3226::new();
        assert_eq!(drv.name(), "3226");
    }
}
