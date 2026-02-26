//! Intel 4008 Address Latch
//!
//! The 4008 latches the 12-bit address from the multiplexed 4-bit data bus
//! during the A1, A2, and A3 phases of the MCS-4 bus cycle. It also decodes
//! CM-ROM lines to produce active-low chip select outputs for up to 16 ROM
//! chips.
//!
//! In the MCS-4 system, the 4008 sits between the 4004 CPU and the ROM/RAM
//! chips, providing a stable address output while the data bus is used for
//! other purposes during M1/M2/X1/X2/X3 phases.

use mcs4_bus::prelude::*;

/// Intel 4008: 12-bit Address Latch with CM-ROM decode
///
/// Latches the multiplexed address from the 4-bit data bus across three
/// bus phases (A1, A2, A3) and decodes CM-ROM chip select lines.
#[derive(Clone, Debug)]
pub struct I4008 {
    /// Latched 12-bit address (assembled from A1/A2/A3 nibbles)
    address: u16,

    /// Address nibble latches (for phase-by-phase assembly)
    a1_nibble: u8,
    a2_nibble: u8,
    a3_nibble: u8,

    /// CM-ROM decoded chip select (0-15, or None if no ROM selected)
    rom_select: Option<u8>,

    /// Address valid flag (true after A3 completes)
    address_valid: bool,
}

impl I4008 {
    pub fn new() -> Self {
        Self {
            address: 0,
            a1_nibble: 0,
            a2_nibble: 0,
            a3_nibble: 0,
            rom_select: None,
            address_valid: false,
        }
    }

    /// Get the latched 12-bit address
    pub fn address(&self) -> u16 {
        self.address
    }

    /// Whether the latched address is valid (all three phases completed)
    pub fn address_valid(&self) -> bool {
        self.address_valid
    }

    /// Get the decoded ROM chip select (0-15), or None if no ROM selected
    pub fn rom_select(&self) -> Option<u8> {
        self.rom_select
    }

    /// Active-low chip select output for a specific ROM chip.
    /// Returns true (active) if this ROM chip is selected.
    pub fn cs_rom(&self, chip_id: u8) -> bool {
        self.rom_select == Some(chip_id)
    }

    /// Process bus phases to latch address and decode chip selects.
    pub fn tick_bus(&mut self, phase: BusCycle, bus: &DataBus, ctrl: &ControlSignals) {
        match phase {
            BusCycle::A1 => {
                // Latch low address nibble (bits 0-3)
                self.a1_nibble = bus.read() & 0x0F;
                self.address_valid = false;
            }
            BusCycle::A2 => {
                // Latch middle address nibble (bits 4-7)
                self.a2_nibble = bus.read() & 0x0F;
            }
            BusCycle::A3 => {
                // Latch high address nibble (bits 8-11)
                self.a3_nibble = bus.read() & 0x0F;

                // Assemble full 12-bit address
                self.address =
                    (self.a1_nibble as u16) | ((self.a2_nibble as u16) << 4) | ((self.a3_nibble as u16) << 8);
                self.address_valid = true;

                // Decode CM-ROM chip select from control signals
                self.rom_select = ctrl.selected_rom();
            }
            _ => {
                // Address latch holds during M1/M2/X1/X2/X3
            }
        }
    }
}

impl Default for I4008 {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Chip for I4008 {
    fn name(&self) -> &'static str {
        "4008"
    }

    fn reset(&mut self) {
        self.address = 0;
        self.a1_nibble = 0;
        self.a2_nibble = 0;
        self.a3_nibble = 0;
        self.rom_select = None;
        self.address_valid = false;
    }

    fn tick(&mut self, _phase: BusCycle) {
        // Bus-connected operation handled by tick_bus()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctrl_with_rom(rom_id: u8) -> ControlSignals {
        let mut ctrl = ControlSignals::mcs4();
        ctrl.select_rom(rom_id, 0);
        ctrl
    }

    #[test]
    fn address_latching_a1_a2_a3() {
        let mut latch = I4008::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs4();

        // A1: low nibble = 0x5
        bus.write(0x5);
        latch.tick_bus(BusCycle::A1, &bus, &ctrl);
        assert!(!latch.address_valid());

        // A2: mid nibble = 0xA
        bus.write(0xA);
        latch.tick_bus(BusCycle::A2, &bus, &ctrl);

        // A3: high nibble = 0x3
        bus.write(0x3);
        latch.tick_bus(BusCycle::A3, &bus, &ctrl);

        assert!(latch.address_valid());
        assert_eq!(latch.address(), 0x3A5);
    }

    #[test]
    fn address_nibbles_independent() {
        let mut latch = I4008::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs4();

        bus.write(0xF);
        latch.tick_bus(BusCycle::A1, &bus, &ctrl);

        bus.write(0x0);
        latch.tick_bus(BusCycle::A2, &bus, &ctrl);

        bus.write(0x7);
        latch.tick_bus(BusCycle::A3, &bus, &ctrl);

        assert_eq!(latch.address(), 0x70F);
    }

    #[test]
    fn address_holds_during_execution_phases() {
        let mut latch = I4008::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs4();

        // Set up address
        bus.write(0xA);
        latch.tick_bus(BusCycle::A1, &bus, &ctrl);
        bus.write(0xB);
        latch.tick_bus(BusCycle::A2, &bus, &ctrl);
        bus.write(0xC);
        latch.tick_bus(BusCycle::A3, &bus, &ctrl);

        let addr = latch.address();
        assert_eq!(addr, 0xCBA);

        // Bus changes during M1/M2/X1/X2/X3 should not affect latched address
        bus.write(0x0);
        latch.tick_bus(BusCycle::M1, &bus, &ctrl);
        assert_eq!(latch.address(), addr);

        latch.tick_bus(BusCycle::M2, &bus, &ctrl);
        assert_eq!(latch.address(), addr);

        latch.tick_bus(BusCycle::X1, &bus, &ctrl);
        assert_eq!(latch.address(), addr);
    }

    #[test]
    fn rom_select_decoded_from_control() {
        let mut latch = I4008::new();
        let mut bus = DataBus::new();
        let ctrl = make_ctrl_with_rom(5);

        bus.write(0);
        latch.tick_bus(BusCycle::A1, &bus, &ctrl);
        latch.tick_bus(BusCycle::A2, &bus, &ctrl);
        latch.tick_bus(BusCycle::A3, &bus, &ctrl);

        assert_eq!(latch.rom_select(), Some(5));
        assert!(latch.cs_rom(5));
        assert!(!latch.cs_rom(0));
        assert!(!latch.cs_rom(15));
    }

    #[test]
    fn no_rom_select_without_cm() {
        let mut latch = I4008::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs4(); // no ROM selected

        bus.write(0);
        latch.tick_bus(BusCycle::A1, &bus, &ctrl);
        latch.tick_bus(BusCycle::A2, &bus, &ctrl);
        latch.tick_bus(BusCycle::A3, &bus, &ctrl);

        assert_eq!(latch.rom_select(), None);
        assert!(!latch.cs_rom(0));
    }

    #[test]
    fn reset_clears_all_state() {
        use crate::Chip;
        let mut latch = I4008::new();
        let mut bus = DataBus::new();
        let ctrl = make_ctrl_with_rom(3);

        bus.write(0xF);
        latch.tick_bus(BusCycle::A1, &bus, &ctrl);
        latch.tick_bus(BusCycle::A2, &bus, &ctrl);
        latch.tick_bus(BusCycle::A3, &bus, &ctrl);

        assert_ne!(latch.address(), 0);
        assert!(latch.address_valid());

        latch.reset();

        assert_eq!(latch.address(), 0);
        assert!(!latch.address_valid());
        assert_eq!(latch.rom_select(), None);
    }

    #[test]
    fn chip_trait_name() {
        use crate::Chip;
        let latch = I4008::new();
        assert_eq!(latch.name(), "4008");
    }

    #[test]
    fn a1_invalidates_previous_address() {
        let mut latch = I4008::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs4();

        // Complete first address
        bus.write(0x1);
        latch.tick_bus(BusCycle::A1, &bus, &ctrl);
        bus.write(0x2);
        latch.tick_bus(BusCycle::A2, &bus, &ctrl);
        bus.write(0x3);
        latch.tick_bus(BusCycle::A3, &bus, &ctrl);
        assert!(latch.address_valid());

        // Start new address cycle -- A1 should invalidate
        bus.write(0x4);
        latch.tick_bus(BusCycle::A1, &bus, &ctrl);
        assert!(!latch.address_valid());
    }

    #[test]
    fn all_zeros_address() {
        let mut latch = I4008::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs4();

        bus.write(0x0);
        latch.tick_bus(BusCycle::A1, &bus, &ctrl);
        latch.tick_bus(BusCycle::A2, &bus, &ctrl);
        latch.tick_bus(BusCycle::A3, &bus, &ctrl);

        assert_eq!(latch.address(), 0x000);
    }

    #[test]
    fn max_address() {
        let mut latch = I4008::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs4();

        bus.write(0xF);
        latch.tick_bus(BusCycle::A1, &bus, &ctrl);
        latch.tick_bus(BusCycle::A2, &bus, &ctrl);
        latch.tick_bus(BusCycle::A3, &bus, &ctrl);

        assert_eq!(latch.address(), 0xFFF);
    }
}
