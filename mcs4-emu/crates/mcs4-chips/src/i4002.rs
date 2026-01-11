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
    pub chip_id: u8,

    /// Bank ID (0-3, selected by CM-RAM lines)
    pub bank_id: u8,

    /// Latched register select from SRC command
    selected_register: u8,

    /// Latched character address from SRC command
    selected_char: u8,

    /// Was this chip selected by the most recent SRC?
    src_selected: bool,

    /// SRC in progress for this chip (chip/reg latched, waiting for char nibble).
    src_pending: bool,

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
            src_selected: false,
            src_pending: false,
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

    /// Process a bus phase
    pub fn tick_bus(&mut self, phase: BusCycle, bus: &mut DataBus, ctrl: &ControlSignals) {
        self.phase = phase;

        // Check if we're selected by CM-RAM bank
        let bank_selected = ctrl.selected_ram() == Some(self.bank_id);

        match phase {
            BusCycle::A1 | BusCycle::A2 | BusCycle::A3 | BusCycle::M1 | BusCycle::M2 => {
                // Address and memory phases - RAM doesn't respond
            }
            BusCycle::X1 => {
                // Phase-accurate model: selection is evaluated in the transfer phase (X2/X3).
                self.selected = false;
            }
            BusCycle::X2 => {
                self.selected = bank_selected && self.src_selected;

                // SRC latches chip+register in X2 (one nibble).
                if bank_selected && ctrl.io_op == Some(IoOp::Src) {
                    let chip_reg = bus.read() & 0x0F;
                    let chip = chip_reg & 0x03;
                    let reg = (chip_reg >> 2) & 0x03;

                    if chip == self.chip_id {
                        self.selected_register = reg;
                        self.src_pending = true;
                    } else {
                        self.src_pending = false;
                    }
                }

                // Write operations during X2
                if self.selected && ctrl.is_io_write() && ctrl.io_op != Some(IoOp::Src) {
                    match ctrl.io_op {
                        Some(IoOp::RamMainWrite) => {
                            let value = bus.read() & 0x0F;
                            self.ram[self.selected_register as usize][self.selected_char as usize] = value;
                        }
                        Some(IoOp::RamPortWrite) => {
                            self.output = bus.read() & 0x0F;
                        }
                        Some(IoOp::RamStatusWrite(idx)) => {
                            self.status[(idx & 3) as usize] = bus.read() & 0x0F;
                        }
                        _ => {}
                    }
                } else if ctrl.io_op == Some(IoOp::Src) {
                    // SRC uses the bus in X2, but the RAM selection is per-chip nibble, not `self.selected`.
                    // Keep `self.selected` as "SRC previously selected" for subsequent operations.
                } else {
                    self.selected = false;
                }
            }
            BusCycle::X3 => {
                self.selected = bank_selected && self.src_selected;

                // SRC latches character in X3 (second nibble).
                if bank_selected && ctrl.io_op == Some(IoOp::Src) && self.src_pending {
                    self.selected_char = bus.read() & 0x0F;
                    self.src_selected = true;
                    self.src_pending = false;
                }

                // Read operations during X3
                if self.selected && ctrl.is_io_read() && ctrl.io_op != Some(IoOp::Src) {
                    match ctrl.io_op {
                        Some(IoOp::RamMainRead) => {
                            let value = self.ram[self.selected_register as usize][self.selected_char as usize];
                            bus.write(value);
                        }
                        Some(IoOp::RamStatusRead(idx)) => {
                            bus.write(self.status[(idx & 3) as usize]);
                        }
                        _ => {}
                    }
                } else if ctrl.io_op == Some(IoOp::Src) {
                    // SRC uses the bus in X3 to latch the character nibble.
                } else {
                    self.selected = false;
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
        self.src_selected = false;
        self.src_pending = false;
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

        // Simulate SRC bus transfer:
        // - X2: chip+register nibble (chip in bits 0-1, reg in bits 2-3)
        // - X3: character nibble
        let mut bus = DataBus::new();
        let mut ctrl = ControlSignals::mcs4();
        ctrl.select_ram(1, 0);
        ctrl.io_op = Some(IoOp::Src);

        let chip_reg = 2u8 | (1u8 << 2);
        bus.write(chip_reg);
        ram.tick_bus(BusCycle::X2, &mut bus, &ctrl);

        bus.write(8);
        ram.tick_bus(BusCycle::X3, &mut bus, &ctrl);

        // Now WRM/RDM use that address
        ram.wrm(0x7);
        assert_eq!(ram.rdm(), 0x7);
        assert_eq!(ram.read_direct(1, 8), 0x7);
    }
}
