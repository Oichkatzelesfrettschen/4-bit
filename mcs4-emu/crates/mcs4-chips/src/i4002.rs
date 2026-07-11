//! Intel 4002 RAM + Output
//!
//! The 4002 is a 320-bit RAM with a 4-bit output port.
//! Memory organization: 4 registers x 16 characters x 4 bits + 4 status characters x 4 bits

use mcs4_bus::prelude::*;
use mcs4_core::SimulationFidelity;

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

    /// Simulation fidelity level
    pub(crate) fidelity: SimulationFidelity,
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
            fidelity: SimulationFidelity::Behavioral,
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

    /// Get currently selected register (debug only)
    pub fn selected_register(&self) -> u8 {
        self.selected_register
    }

    /// Get currently selected character (debug only)
    pub fn selected_char(&self) -> u8 {
        self.selected_char
    }

    /// Get src_selected flag (debug only)
    pub fn src_selected(&self) -> bool {
        self.src_selected
    }

    /// Process a bus phase
    pub fn tick_bus(&mut self, phase: BusCycle, bus: &mut DataBus, ctrl: &ControlSignals) {
        self.phase = phase;

        // Each DATA RAM bank hangs off one CM-RAM line; this chip is selected
        // when its bank's line is asserted in the CM-RAM mask.
        let bank_selected = ctrl
            .selected_ram()
            .is_some_and(|lines| lines & (1 << self.bank_id) != 0);

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
                        // SRC selects exactly one chip at a time within the active bank; clear the
                        // prior selection until the character nibble is latched in X3.
                        self.src_selected = false;
                    } else {
                        self.src_pending = false;
                        // Clear any stale selection from a previous SRC targeting another chip.
                        self.src_selected = false;
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

// --- Solver Bridge ---

use mcs4_core::{
    bridge::{ChipSolverBridge, PinDirection, PinMapping},
    circuit::graph::CircuitGraph,
};

const SUBCIRCUIT_NAMES_4002: &[&str] = &["custom", "OUT1", "D0_PAD", "OUT2", "OUT3", "OUT0"];

impl I4002 {
    fn load_subcircuit_json(name: &str) -> Option<CircuitGraph> {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir.ancestors().nth(3)?;
        let path = repo_root
            .join("docs/evidence/subcircuits_v0/4002")
            .join(format!("4002_{}_subcircuit_v0.json", name));
        let netlist = mcs4_core::layout_netlist::load_netlist_v1(&path).ok()?;
        let config = mcs4_core::circuit::netlist_bridge::BridgeConfig::default();
        Some(mcs4_core::circuit::netlist_bridge::netlist_v1_to_circuit(
            &netlist, &config,
        ))
    }
}

impl ChipSolverBridge for I4002 {
    fn fidelity(&self) -> SimulationFidelity {
        self.fidelity
    }

    fn set_fidelity(&mut self, fidelity: SimulationFidelity) {
        self.fidelity = fidelity;
    }

    fn subcircuit_names(&self) -> Vec<&str> {
        SUBCIRCUIT_NAMES_4002.to_vec()
    }

    fn subcircuit(&self, name: &str) -> Option<CircuitGraph> {
        if SUBCIRCUIT_NAMES_4002.contains(&name) {
            Self::load_subcircuit_json(name)
        } else {
            None
        }
    }

    fn pin_map(&self) -> Vec<PinMapping> {
        vec![
            PinMapping {
                name: "CLK1".into(),
                node_id: 2259,
                direction: PinDirection::Input,
            },
            PinMapping {
                name: "CLK2".into(),
                node_id: 2619,
                direction: PinDirection::Input,
            },
            PinMapping {
                name: "CM".into(),
                node_id: 799,
                direction: PinDirection::Input,
            },
            PinMapping {
                name: "CS".into(),
                node_id: 789,
                direction: PinDirection::Input,
            },
            PinMapping {
                name: "D0_PAD".into(),
                node_id: 868,
                direction: PinDirection::Bidirectional,
            },
            PinMapping {
                name: "D1_PAD".into(),
                node_id: 872,
                direction: PinDirection::Bidirectional,
            },
            PinMapping {
                name: "D2_PAD".into(),
                node_id: 938,
                direction: PinDirection::Bidirectional,
            },
            PinMapping {
                name: "D3_PAD".into(),
                node_id: 896,
                direction: PinDirection::Bidirectional,
            },
            PinMapping {
                name: "OUT0".into(),
                node_id: 3115,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "OUT1".into(),
                node_id: 2855,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "OUT2".into(),
                node_id: 2605,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "OUT3".into(),
                node_id: 1685,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "SYNC".into(),
                node_id: 3261,
                direction: PinDirection::Input,
            },
            PinMapping {
                name: "RESET".into(),
                node_id: 3225,
                direction: PinDirection::Input,
            },
            PinMapping {
                name: "VDD".into(),
                node_id: 3251,
                direction: PinDirection::Input,
            },
            PinMapping {
                name: "VSS".into(),
                node_id: 73,
                direction: PinDirection::Input,
            },
        ]
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
        // Bank 1 hangs off CM-RAM1: assert that line in the mask.
        ctrl.select_ram(0b0010, 0);
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

    #[test]
    fn test_src_clears_other_chip_selection_in_bank() {
        let mut ram0 = I4002::new(0, 0);
        let mut ram1 = I4002::new(1, 0);

        let mut bus = DataBus::new();
        let mut ctrl = ControlSignals::mcs4();
        // Bank 0 hangs off CM-RAM0: assert that line in the mask.
        ctrl.select_ram(0b0001, 0);

        // SRC selects chip 0, reg 0, char 3.
        ctrl.io_op = Some(IoOp::Src);
        bus.write(0u8);
        ram0.tick_bus(BusCycle::X2, &mut bus, &ctrl);
        ram1.tick_bus(BusCycle::X2, &mut bus, &ctrl);

        bus.write(3);
        ram0.tick_bus(BusCycle::X3, &mut bus, &ctrl);
        ram1.tick_bus(BusCycle::X3, &mut bus, &ctrl);

        // WRM writes only to chip 0 (chip 1 must stay inactive).
        ctrl.io_op = Some(IoOp::RamMainWrite);
        bus.write(0xA);
        ram0.tick_bus(BusCycle::X2, &mut bus, &ctrl);
        ram1.tick_bus(BusCycle::X2, &mut bus, &ctrl);
        assert_eq!(ram0.read_direct(0, 3), 0xA);
        assert_eq!(ram1.read_direct(0, 3), 0x0);

        // SRC now targets chip 1, reg 2, char 4; this must clear chip 0 selection.
        ctrl.io_op = Some(IoOp::Src);
        bus.write(1u8 | (2u8 << 2));
        ram0.tick_bus(BusCycle::X2, &mut bus, &ctrl);
        ram1.tick_bus(BusCycle::X2, &mut bus, &ctrl);

        bus.write(4);
        ram0.tick_bus(BusCycle::X3, &mut bus, &ctrl);
        ram1.tick_bus(BusCycle::X3, &mut bus, &ctrl);

        // WRM should write only to chip 1, not to chip 0.
        ctrl.io_op = Some(IoOp::RamMainWrite);
        bus.write(0xB);
        ram0.tick_bus(BusCycle::X2, &mut bus, &ctrl);
        ram1.tick_bus(BusCycle::X2, &mut bus, &ctrl);
        assert_eq!(ram0.read_direct(0, 3), 0xA);
        assert_eq!(ram1.read_direct(2, 4), 0xB);
    }

    // --- Solver Bridge tests ---

    use mcs4_core::bridge::ChipSolverBridge;

    #[test]
    fn bridge_subcircuit_names_count() {
        let ram = I4002::new(0, 0);
        let names = ram.subcircuit_names();
        assert_eq!(names.len(), 6);
        assert!(names.contains(&"custom"));
        assert!(names.contains(&"OUT0"));
    }

    #[test]
    fn bridge_fidelity_default() {
        let ram = I4002::new(0, 0);
        assert_eq!(ram.fidelity(), mcs4_core::SimulationFidelity::Behavioral);
    }

    #[test]
    fn bridge_custom_subcircuit_loads() {
        let ram = I4002::new(0, 0);
        let g = ram.subcircuit("custom").expect("custom subcircuit");
        assert_eq!(g.transistor_count(), 42);
    }

    #[test]
    fn bridge_out0_subcircuit_loads() {
        let ram = I4002::new(0, 0);
        let g = ram.subcircuit("OUT0").expect("OUT0 subcircuit");
        assert_eq!(g.transistor_count(), 1);
    }

    #[test]
    fn bridge_pin_map_has_entries() {
        let ram = I4002::new(0, 0);
        let pins = ram.pin_map();
        assert!(pins.len() >= 10);
        assert!(pins.iter().any(|p| p.name == "CLK1"));
    }
}
