//! Intel 4289 Standard Memory Interface
//!
//! The 4289 bridges the MCS-4 bus to standard memory (e.g. 2101 RAM, 1702 EPROM).
//! Organization:
//! - Latches 12-bit address from CPU (A1, A2, A3)
//! - Interfaces with CM-ROM/CM-RAM lines
//! - Provides 8-bit data path to standard memory

use mcs4_bus::prelude::*;

/// Intel 4289: Standard Memory Interface
#[derive(Clone, Debug)]
pub struct I4289 {
    /// Latched 12-bit address
    address: u16,

    /// Current bus phase
    phase: BusCycle,

    /// Chip selects
    cs_rom: bool,
    cs_ram: bool,
}

impl Default for I4289 {
    fn default() -> Self {
        Self {
            address: 0,
            phase: BusCycle::A1,
            cs_rom: false,
            cs_ram: false,
        }
    }
}

impl I4289 {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get current 12-bit address for external memory
    pub fn address(&self) -> u16 {
        self.address
    }

    /// Process bus phases
    pub fn tick_bus(&mut self, phase: BusCycle, bus: &mut DataBus, ctrl: &ControlSignals) {
        self.phase = phase;

        match phase {
            BusCycle::A1 => {
                // Latch address bits 0-3
                self.address = (self.address & 0xFFF0) | (bus.read() as u16);
            }
            BusCycle::A2 => {
                // Latch address bits 4-7
                self.address = (self.address & 0xFF0F) | ((bus.read() as u16) << 4);
            }
            BusCycle::A3 => {
                // Latch address bits 8-11
                self.address = (self.address & 0xF0FF) | ((bus.read() as u16) << 8);

                // Check if we are selected by CM-ROM lines
                self.cs_rom = ctrl.selected_rom().is_some();
                self.cs_ram = ctrl.selected_ram().is_some();
            }
            _ => {}
        }
    }
}

impl super::Chip for I4289 {
    fn name(&self) -> &'static str {
        "4289"
    }
    fn reset(&mut self) {
        self.address = 0;
        self.cs_rom = false;
        self.cs_ram = false;
    }
    fn tick(&mut self, _phase: BusCycle) {}
}
