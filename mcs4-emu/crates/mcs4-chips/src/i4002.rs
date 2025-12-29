//! Intel 4002 RAM + Output
//!
//! The 4002 is a 320-bit RAM with a 4-bit output port.
//! Memory organization: 4 registers x 16 characters x 4 bits + 4 status characters x 4 bits

use mcs4_bus::prelude::*;

/// Intel 4002: 320-bit RAM with 4-bit output port
#[derive(Clone, Debug)]
pub struct I4002 {
    /// RAM: 4 registers x 16 nibbles (main memory)
    ram: [[u8; 16]; 4],

    /// Status registers: 4 nibbles (one per register)
    status: [u8; 4],

    /// Output port latch
    output: u8,

    /// Chip ID within bank (0-3)
    chip_id: u8,

    /// Bank ID (0-3, selected by CM-RAM lines)
    bank_id: u8,

    /// Latched register select from SRC command
    selected_register: u8,

    /// Latched character address from SRC command
    selected_char: u8,

    /// Is this chip selected for current transaction?
    selected: bool,

    /// Current phase tracking
    phase: BusCycle,
}

impl I4002 {
    /// Create a new 4002 RAM with specified chip ID (0-3) and bank (0-3)
    pub fn new(chip_id: u8, bank_id: u8) -> Self {
        Self {
            ram: [[0; 16]; 4],
            status: [0; 4],
            output: 0,
            chip_id: chip_id & 0x03,
            bank_id: bank_id & 0x03,
            selected_register: 0,
            selected_char: 0,
            selected: false,
            phase: BusCycle::A1,
        }
    }

    /// Create a 4002 with just chip ID (bank 0)
    pub fn with_chip_id(chip_id: u8) -> Self {
        Self::new(chip_id, 0)
    }

    /// Read RAM character (direct access for debugging)
    pub fn read_direct(&self, reg: u8, char_idx: u8) -> u8 {
        self.ram[(reg & 3) as usize][(char_idx & 0x0F) as usize] & 0x0F
    }

    /// Write RAM character (direct access for debugging/initialization)
    pub fn write_direct(&mut self, reg: u8, char_idx: u8, value: u8) {
        self.ram[(reg & 3) as usize][(char_idx & 0x0F) as usize] = value & 0x0F;
    }

    /// Read status character (direct access)
    pub fn read_status(&self, reg: u8) -> u8 {
        self.status[(reg & 3) as usize] & 0x0F
    }

    /// Write status character (direct access)
    pub fn write_status(&mut self, reg: u8, value: u8) {
        self.status[(reg & 3) as usize] = value & 0x0F;
    }

    /// Get output port value
    pub fn output(&self) -> u8 {
        self.output & 0x0F
    }

    /// Get chip ID
    pub fn chip_id(&self) -> u8 {
        self.chip_id
    }

    /// Get bank ID
    pub fn bank_id(&self) -> u8 {
        self.bank_id
    }

    /// Check if chip is currently selected
    pub fn is_selected(&self) -> bool {
        self.selected
    }

    /// Set the SRC address (called by system when CPU executes SRC)
    pub fn set_src_address(&mut self, chip: u8, reg: u8, char_addr: u8) {
        if (chip & 0x03) == self.chip_id {
            self.selected_register = reg & 0x03;
            self.selected_char = char_addr & 0x0F;
        }
    }

    /// Process a bus phase
    pub fn tick_bus(&mut self, phase: BusCycle, bus: &mut DataBus, ctrl: &ControlSignals) {
        self.phase = phase;

        // Check if we're selected by CM-RAM bank
        let ram_bank = ctrl.cm_ram();
        let bank_selected = ram_bank == self.bank_id;

        match phase {
            BusCycle::A1 | BusCycle::A2 | BusCycle::A3 | BusCycle::M1 | BusCycle::M2 => {
                // Address and memory phases - RAM doesn't respond
            }
            BusCycle::X1 => {
                // Check selection - in real hardware this would be from SRC
                self.selected = bank_selected;
            }
            BusCycle::X2 => {
                // Write operations during X2
                if self.selected {
                    if ctrl.is_io_write() {
                        // This could be WRM, WMP, or WRx
                        // For WRM: write to RAM
                        // For WMP: write to output port
                        // For WRx: write to status register x
                        // The exact operation is determined by the instruction decoder
                        // For now, we store to RAM as the default
                        let value = bus.read() & 0x0F;
                        self.ram[self.selected_register as usize][self.selected_char as usize] = value;
                    }
                }
            }
            BusCycle::X3 => {
                // Read operations during X3
                if self.selected && ctrl.is_io_read() {
                    // RDM: read from RAM
                    let value = self.ram[self.selected_register as usize][self.selected_char as usize];
                    bus.write(value);
                }
            }
        }
    }

    /// Write to RAM main memory (WRM instruction)
    pub fn wrm(&mut self, value: u8) {
        self.ram[self.selected_register as usize][self.selected_char as usize] = value & 0x0F;
    }

    /// Read from RAM main memory (RDM instruction)
    pub fn rdm(&self) -> u8 {
        self.ram[self.selected_register as usize][self.selected_char as usize] & 0x0F
    }

    /// Write to output port (WMP instruction)
    pub fn wmp(&mut self, value: u8) {
        self.output = value & 0x0F;
    }

    /// Write to status character (WR0-WR3 instructions)
    pub fn wrx(&mut self, status_idx: u8, value: u8) {
        self.status[(status_idx & 3) as usize] = value & 0x0F;
    }

    /// Read from status character (RD0-RD3 instructions)
    pub fn rdx(&self, status_idx: u8) -> u8 {
        self.status[(status_idx & 3) as usize] & 0x0F
    }
}

impl Default for I4002 {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

impl super::Chip for I4002 {
    fn name(&self) -> &'static str {
        "4002"
    }

    fn reset(&mut self) {
        self.ram = [[0; 16]; 4];
        self.status = [0; 4];
        self.output = 0;
        self.selected_register = 0;
        self.selected_char = 0;
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
    fn test_ram_read_write() {
        let mut ram = I4002::new(0, 0);

        // Direct write/read
        ram.write_direct(0, 5, 0xA);
        assert_eq!(ram.read_direct(0, 5), 0xA);

        // Test masking
        ram.write_direct(1, 7, 0xFF);
        assert_eq!(ram.read_direct(1, 7), 0x0F);
    }

    #[test]
    fn test_status_registers() {
        let mut ram = I4002::new(0, 0);

        ram.write_status(2, 0xC);
        assert_eq!(ram.read_status(2), 0xC);

        // Test via instruction-level API
        ram.wrx(3, 0x5);
        assert_eq!(ram.rdx(3), 0x5);
    }

    #[test]
    fn test_output_port() {
        let mut ram = I4002::new(0, 0);

        ram.wmp(0xB);
        assert_eq!(ram.output(), 0xB);

        // Test masking
        ram.wmp(0xFF);
        assert_eq!(ram.output(), 0x0F);
    }

    #[test]
    fn test_addressing() {
        let mut ram = I4002::new(2, 1);

        assert_eq!(ram.chip_id(), 2);
        assert_eq!(ram.bank_id(), 1);

        // Set SRC address for this chip
        ram.set_src_address(2, 1, 8);

        // Now WRM/RDM use that address
        ram.wrm(0x7);
        assert_eq!(ram.rdm(), 0x7);
        assert_eq!(ram.read_direct(1, 8), 0x7);
    }
}
