//! Intel 4001 ROM + I/O
//!
//! The 4001 is a 256x8-bit ROM with a 4-bit I/O port.
//! Up to 16 4001 chips can be addressed in an MCS-4 system.

use mcs4_bus::prelude::*;

/// Intel 4001: 256x8 ROM with 4-bit I/O port
#[derive(Clone, Debug)]
pub struct I4001 {
    /// ROM contents (256 bytes)
    rom: [u8; 256],

    /// I/O port output latch (directly controlled by CPU)
    io_output: u8,

    /// I/O port input (directly readable by CPU)
    io_input: u8,

    /// Chip select ID (0-15), set at construction
    pub chip_id: u8,

    /// Latched address from A1/A2/A3 phases
    address: u8,

    /// Is this chip selected for current transaction?
    selected: bool,

    /// Current phase tracking
    phase: BusCycle,
}

impl I4001 {
    /// Create a new 4001 ROM with specified chip ID (0-15)
    pub fn new(chip_id: u8) -> Self {
        Self {
            rom: [0; 256],
            io_output: 0,
            io_input: 0,
            chip_id: chip_id & 0x0F,
            address: 0,
            selected: false,
            phase: BusCycle::A1,
        }
    }

    /// Load ROM contents from a byte slice
    pub fn load(&mut self, data: &[u8]) {
        let len = data.len().min(256);
        self.rom[..len].copy_from_slice(&data[..len]);
    }

    /// Load ROM contents from a byte slice at an offset
    pub fn load_at(&mut self, offset: usize, data: &[u8]) {
        let end = (offset + data.len()).min(256);
        let len = end.saturating_sub(offset);
        if len > 0 {
            self.rom[offset..end].copy_from_slice(&data[..len]);
        }
    }

    /// Read ROM at address (direct access for debugging)
    pub fn read_direct(&self, addr: u8) -> u8 {
        self.rom[addr as usize]
    }

    /// Write ROM at address (for programming/testing)
    pub fn write_direct(&mut self, addr: u8, value: u8) {
        self.rom[addr as usize] = value;
    }

    /// Get I/O port output value
    pub fn io_output(&self) -> u8 {
        self.io_output & 0x0F
    }

    /// Set I/O port input value (from external source)
    pub fn set_io_input(&mut self, value: u8) {
        self.io_input = value & 0x0F;
    }

    /// Get I/O port input value
    pub fn io_input(&self) -> u8 {
        self.io_input & 0x0F
    }

    /// Get chip ID
    pub fn chip_id(&self) -> u8 {
        self.chip_id
    }

    /// Check if chip is currently selected
    pub fn is_selected(&self) -> bool {
        self.selected
    }

    /// Process a bus phase
    pub fn tick_bus(&mut self, phase: BusCycle, bus: &mut DataBus, ctrl: &ControlSignals) {
        self.phase = phase;

        match phase {
            BusCycle::A1 => {
                // Capture address bits 0-3
                self.address = (self.address & 0xF0) | (bus.read() & 0x0F);
                self.selected = false;
            }
            BusCycle::A2 => {
                // Capture address bits 4-7
                self.address = (self.address & 0x0F) | ((bus.read() & 0x0F) << 4);
            }
            BusCycle::A3 => {
                // Check if we're selected by CM-ROM
                let rom_select = ctrl.cm_rom();
                self.selected = rom_select == self.chip_id;
            }
            BusCycle::M1 => {
                // Output OPA (lower nibble of instruction) if selected
                if self.selected {
                    let data = self.rom[self.address as usize];
                    bus.write(data & 0x0F);
                }
            }
            BusCycle::M2 => {
                // Output OPR (upper nibble of instruction) if selected
                if self.selected {
                    let data = self.rom[self.address as usize];
                    bus.write((data >> 4) & 0x0F);
                }
            }
            BusCycle::X1 => {
                // I/O operations happen during X phases
            }
            BusCycle::X2 => {
                // WRR: Write ROM port (from accumulator via bus)
                if self.selected && ctrl.is_io_write() {
                    self.io_output = bus.read() & 0x0F;
                }
            }
            BusCycle::X3 => {
                // RDR: Read ROM port (to accumulator via bus)
                if self.selected && ctrl.is_io_read() {
                    bus.write(self.io_input);
                }
            }
        }
    }
}

impl Default for I4001 {
    fn default() -> Self {
        Self::new(0)
    }
}

impl super::Chip for I4001 {
    fn name(&self) -> &'static str {
        "4001"
    }

    fn reset(&mut self) {
        self.io_output = 0;
        self.address = 0;
        self.selected = false;
        self.phase = BusCycle::A1;
    }

    fn tick(&mut self, phase: BusCycle) {
        // Simplified tick without bus access
        self.phase = phase;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_rom() {
        let mut rom = I4001::new(0);
        rom.load(&[0x10, 0x20, 0x30, 0x40]);

        assert_eq!(rom.read_direct(0), 0x10);
        assert_eq!(rom.read_direct(1), 0x20);
        assert_eq!(rom.read_direct(2), 0x30);
        assert_eq!(rom.read_direct(3), 0x40);
        assert_eq!(rom.read_direct(4), 0x00);
    }

    #[test]
    fn test_io_port() {
        let mut rom = I4001::new(5);

        rom.set_io_input(0xA);
        assert_eq!(rom.io_input(), 0xA);

        // Test masking to 4 bits
        rom.set_io_input(0xFF);
        assert_eq!(rom.io_input(), 0x0F);
    }

    #[test]
    fn test_chip_id() {
        let rom = I4001::new(7);
        assert_eq!(rom.chip_id(), 7);

        // Test masking to 4 bits
        let rom2 = I4001::new(0x1F);
        assert_eq!(rom2.chip_id(), 0x0F);
    }
}
