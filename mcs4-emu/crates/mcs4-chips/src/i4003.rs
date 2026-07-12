//! Intel 4003 10-bit Shift Register
//!
//! The 4003 is a 10-bit serial-in, parallel-out shift register used for
//! I/O expansion in MCS-4 and MCS-40 systems. CP shifts the register on a
//! rising edge. The physical E input is active low: E low exposes the
//! parallel outputs and E high drives those outputs to VSS. E does not gate
//! CP shifting or serial output. Multiple 4003s can be cascaded by connecting
//! serial_out of one chip to set_data_in of the next.
//!
//! Typical connection via 4002 output port:
//! - Port bit 0 -> 4003 serial data in
//! - Port bit 1 -> 4003 clock
//! - (a separate port bit drives each active-low 4003 E input when needed)
//!
//! The WMP instruction writes the accumulator to the 4002 output port,
//! which in turn drives the 4003.

use mcs4_bus::BusCycle;
use mcs4_core::SimulationFidelity;

/// Intel 4003: 10-bit shift register
#[derive(Clone, Debug)]
pub struct I4003 {
    /// Internal 10-bit register
    data: u16,

    /// Serial data input pin
    serial_in: bool,

    /// Clock pin state (prev)
    last_clock: bool,

    /// Physical E input level. E low exposes the parallel output pins.
    enable_pin: bool,

    /// Simulation fidelity level
    pub(crate) fidelity: SimulationFidelity,
}

impl Default for I4003 {
    fn default() -> Self {
        Self::new()
    }
}

impl I4003 {
    pub fn new() -> Self {
        Self {
            data: 0,
            serial_in: false,
            last_clock: false,
            // Standalone construction holds E low so existing convenience users
            // can observe the parallel outputs until they model a physical E pin.
            enable_pin: false,
            fidelity: SimulationFidelity::Behavioral,
        }
    }

    /// Shift in one serial bit (one clock pulse).
    pub fn shift_in(&mut self, bit: bool) {
        self.set_data_in(bit);
        // Ensure a clean rising edge.
        self.set_clock(false);
        self.set_clock(true);
    }

    /// Set serial data input
    pub fn set_data_in(&mut self, state: bool) {
        self.serial_in = state;
    }

    /// Drive the physical active-low E input.
    ///
    /// A low level exposes the parallel output pins. A high level drives the
    /// parallel output pins to VSS without stopping CP shifting or serial output.
    pub fn set_enable_pin(&mut self, level: bool) {
        self.enable_pin = level;
    }

    /// Return the physical active-low E input level.
    pub fn enable_pin(&self) -> bool {
        self.enable_pin
    }

    /// Report whether the physical E input exposes the parallel output pins.
    pub fn parallel_outputs_enabled(&self) -> bool {
        !self.enable_pin
    }

    /// Set parallel-output visibility with the historical API semantics.
    ///
    /// true exposes the parallel outputs. false drives them to VSS.
    /// Use set_enable_pin when modeling the physical active-low E input.
    pub fn set_enable(&mut self, enabled: bool) {
        self.set_enable_pin(!enabled);
    }

    /// Report whether the parallel output pins are exposed.
    pub fn is_enabled(&self) -> bool {
        self.parallel_outputs_enabled()
    }

    /// Drive CP. The register shifts on every rising edge.
    pub fn set_clock(&mut self, state: bool) {
        if !self.last_clock && state {
            // Rising edge: shift in
            self.data = ((self.data << 1) | (self.serial_in as u16)) & 0x3FF;
        }
        self.last_clock = state;
    }

    /// Drive from a 4002 output port nibble.
    ///
    /// Convention: bit 0 = serial data, bit 1 = CP.
    ///
    /// This helper does not drive E. Use set_enable_pin to model that
    /// separately wired active-low input.
    pub fn drive_from_port(&mut self, port_nibble: u8) {
        self.set_data_in(port_nibble & 0x01 != 0);
        self.set_clock(port_nibble & 0x02 != 0);
    }

    /// Get current parallel output
    pub fn parallel_out(&self) -> u16 {
        if self.parallel_outputs_enabled() {
            self.data
        } else {
            0
        }
    }

    /// Get individual output bit (0-9)
    pub fn output_bit(&self, bit: u8) -> bool {
        if bit < 10 {
            (self.parallel_out() >> bit) & 1 != 0
        } else {
            false
        }
    }

    /// Get serial data output (bit 9) for cascading
    pub fn serial_out(&self) -> bool {
        (self.data & 0x200) != 0
    }
}

impl super::Chip for I4003 {
    fn name(&self) -> &'static str {
        "4003"
    }
    fn reset(&mut self) {
        self.data = 0;
        self.last_clock = false;
        // E is an external input and retains its driven level.
    }
    fn tick(&mut self, _phase: BusCycle) {
        // Behavioral model: clock is driven by I/O instructions (WMP/WRR)
    }
}

// --- Solver Bridge ---

use mcs4_core::{
    bridge::{ChipSolverBridge, PinDirection, PinMapping},
    circuit::graph::CircuitGraph,
};

/// Subcircuit names from `docs/evidence/subcircuits_v0/4003/metrics.json`.
const SUBCIRCUIT_NAMES: &[&str] = &["custom", "OUT", "CLOCK", "EN", "DATA"];

impl I4003 {
    /// Load a subcircuit from the evidence JSON files.
    fn load_subcircuit_json(name: &str) -> Option<CircuitGraph> {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir.ancestors().nth(3)?;
        let path = repo_root
            .join("docs/evidence/subcircuits_v0/4003")
            .join(format!("4003_{}_subcircuit_v0.json", name));

        let netlist = mcs4_core::layout_netlist::load_netlist_v1(&path).ok()?;
        let config = mcs4_core::circuit::netlist_bridge::BridgeConfig::default();
        Some(mcs4_core::circuit::netlist_bridge::netlist_v1_to_circuit(
            &netlist, &config,
        ))
    }
}

impl ChipSolverBridge for I4003 {
    fn fidelity(&self) -> SimulationFidelity {
        self.fidelity
    }

    fn set_fidelity(&mut self, fidelity: SimulationFidelity) {
        self.fidelity = fidelity;
    }

    fn subcircuit_names(&self) -> Vec<&str> {
        SUBCIRCUIT_NAMES.to_vec()
    }

    fn subcircuit(&self, name: &str) -> Option<CircuitGraph> {
        if SUBCIRCUIT_NAMES.contains(&name) {
            Self::load_subcircuit_json(name)
        } else {
            None
        }
    }

    fn pin_map(&self) -> Vec<PinMapping> {
        vec![
            PinMapping {
                name: "CLOCK".into(),
                node_id: 239,
                direction: PinDirection::Input,
            },
            PinMapping {
                name: "DATA".into(),
                node_id: 235,
                direction: PinDirection::Input,
            },
            PinMapping {
                name: "EN".into(),
                node_id: 279,
                direction: PinDirection::Input,
            },
            PinMapping {
                name: "OUT".into(),
                node_id: 352,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "Q0".into(),
                node_id: 237,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "Q1".into(),
                node_id: 151,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "Q2".into(),
                node_id: 78,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "Q3".into(),
                node_id: 370,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "Q4".into(),
                node_id: 389,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "Q5".into(),
                node_id: 110,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "Q6".into(),
                node_id: 7,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "Q7".into(),
                node_id: 386,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "Q8".into(),
                node_id: 385,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "Q9".into(),
                node_id: 390,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "VDD".into(),
                node_id: 359,
                direction: PinDirection::Input,
            },
            PinMapping {
                name: "VSS".into(),
                node_id: 152,
                direction: PinDirection::Input,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_in_builds_parallel_word() {
        let mut sr = I4003::new();
        // 10-bit pattern 1010101010 (msb first)
        for bit in [true, false, true, false, true, false, true, false, true, false] {
            sr.shift_in(bit);
        }
        assert_eq!(sr.parallel_out(), 0x2AA);
    }

    #[test]
    fn serial_cascade_overflow() {
        let mut sr = I4003::new();
        // Fill with all ones (10 bits)
        for _ in 0..10 {
            sr.shift_in(true);
        }
        assert_eq!(sr.parallel_out(), 0x3FF);
        assert!(sr.serial_out());

        // Shift in one more zero -- oldest bit (was bit 9) is lost
        sr.shift_in(false);
        assert_eq!(sr.parallel_out(), 0x3FE);
        assert!(sr.serial_out()); // bit 9 is still 1

        // Shift in another zero to push the high bit out
        sr.shift_in(false);
        assert_eq!(sr.parallel_out(), 0x3FC);
    }

    #[test]
    fn all_zeros() {
        let mut sr = I4003::new();
        for _ in 0..10 {
            sr.shift_in(false);
        }
        assert_eq!(sr.parallel_out(), 0x000);
        assert!(!sr.serial_out());
    }

    #[test]
    fn all_ones() {
        let mut sr = I4003::new();
        for _ in 0..10 {
            sr.shift_in(true);
        }
        assert_eq!(sr.parallel_out(), 0x3FF);
        assert!(sr.serial_out());
    }

    #[test]
    fn reset_clears_data() {
        use crate::Chip;
        let mut sr = I4003::new();
        for _ in 0..5 {
            sr.shift_in(true);
        }
        assert_ne!(sr.parallel_out(), 0);
        sr.reset();
        assert_eq!(sr.parallel_out(), 0);
        assert!(!sr.serial_out());
    }

    #[test]
    fn clock_edge_sensitivity() {
        let mut sr = I4003::new();
        sr.set_data_in(true);

        // No shift on high-to-high (no edge)
        sr.set_clock(true);
        sr.set_clock(true);
        // Only one rising edge above, so only 1 bit shifted
        assert_eq!(sr.parallel_out(), 0x001);

        // No shift on falling edge
        sr.set_clock(false);
        assert_eq!(sr.parallel_out(), 0x001);

        // Shift on next rising edge
        sr.set_data_in(false);
        sr.set_clock(true);
        assert_eq!(sr.parallel_out(), 0x002); // previous 1 shifted left, new 0 at bit 0
    }

    #[test]
    fn data_masks_to_10_bits() {
        let mut sr = I4003::new();
        // Shift in 12 ones -- only 10 bits should be retained
        for _ in 0..12 {
            sr.shift_in(true);
        }
        assert_eq!(sr.parallel_out(), 0x3FF);
    }

    // --- B.1.1: Cascade integration tests ---

    #[test]
    fn cascade_two_chips_20_bits() {
        let mut sr0 = I4003::new();
        let mut sr1 = I4003::new();

        // Shift a 20-bit pattern through two cascaded 4003s.
        // Pattern: 0xABCDE (20 bits) = 1010_1011_1100_1101_1110 (msb first)
        let pattern: u32 = 0xABCDE;
        for i in (0..20).rev() {
            let bit = (pattern >> i) & 1 != 0;

            // Feed serial data to first chip
            sr0.set_data_in(bit);

            // Before clocking, connect cascade: sr0.serial_out -> sr1.data_in
            sr1.set_data_in(sr0.serial_out());

            // Clock both simultaneously
            sr0.set_clock(false);
            sr1.set_clock(false);
            sr0.set_clock(true);
            sr1.set_clock(true);
        }

        // sr0 has the lower 10 bits, sr1 has the upper 10 bits
        // Pattern 0xABCDE = 0b1010_1011_1100_1101_1110
        // Lower 10: 0b00_1101_1110 = 0x0DE
        // Upper 10: 0b10_1010_1111 = 0x2AF
        assert_eq!(sr0.parallel_out(), 0x0DE);
        assert_eq!(sr1.parallel_out(), 0x2AF);
    }

    #[test]
    fn cascade_three_chips_30_bits() {
        let mut sr0 = I4003::new();
        let mut sr1 = I4003::new();
        let mut sr2 = I4003::new();

        // Shift a 30-bit word through 3 cascaded chips.
        // Pattern: all 0s for first 10, alternating for next 10, all 1s for last 10
        let mut bits: Vec<bool> = vec![false; 10]; // goes into sr2
        bits.extend((0..10).map(|i| i % 2 == 0)); // goes into sr1
        bits.extend(vec![true; 10]); // stays in sr0

        for &bit in &bits {
            sr1.set_data_in(sr0.serial_out());
            sr2.set_data_in(sr1.serial_out());
            sr0.set_data_in(bit);

            sr0.set_clock(false);
            sr1.set_clock(false);
            sr2.set_clock(false);
            sr0.set_clock(true);
            sr1.set_clock(true);
            sr2.set_clock(true);
        }

        // sr0: last 10 bits shifted in = all 1s = 0x3FF
        assert_eq!(sr0.parallel_out(), 0x3FF);
        // sr1: middle 10 bits = alternating (first in = MSB) = 0b1010101010 = 0x2AA
        assert_eq!(sr1.parallel_out(), 0x2AA);
        // sr2: first 10 bits = all 0s
        assert_eq!(sr2.parallel_out(), 0x000);
    }

    // --- B.1.2: Port-driven operation ---

    #[test]
    fn drive_from_port_nibble() {
        let mut sr = I4003::new();

        // Bit 0 = data, bit 1 = clock
        // Write data=1, clock=0 (setup)
        sr.drive_from_port(0b0001); // data=1, clock=0
        assert_eq!(sr.parallel_out(), 0); // no edge yet

        // Write data=1, clock=1 (rising edge)
        sr.drive_from_port(0b0011); // data=1, clock=1
        assert_eq!(sr.parallel_out(), 1); // shifted in a 1

        // Write data=0, clock=0 (setup next bit)
        sr.drive_from_port(0b0000); // data=0, clock=0

        // Write data=0, clock=1 (rising edge)
        sr.drive_from_port(0b0010); // data=0, clock=1
        assert_eq!(sr.parallel_out(), 0b10); // shifted in a 0, previous 1 moved up
    }

    #[test]
    fn port_driven_full_byte() {
        let mut sr = I4003::new();

        // Shift in 8 bits via port: 0xA5 = 10100101 (msb first)
        let byte: u8 = 0xA5;
        for i in (0..8).rev() {
            let bit = (byte >> i) & 1;
            // Setup: data on bit 0, clock low
            sr.drive_from_port(bit);
            // Clock: data on bit 0, clock high
            sr.drive_from_port(bit | 0x02);
            // Release clock
            sr.drive_from_port(bit);
        }

        // Lower 8 bits of parallel out should be 0xA5
        assert_eq!(sr.parallel_out() & 0xFF, 0xA5);
    }

    // --- B.1.3: Enable pin tests ---

    #[test]
    fn enable_high_masks_parallel_outputs_without_stopping_shift() {
        let mut sr = I4003::new();
        sr.set_enable_pin(true);
        assert!(sr.enable_pin());
        assert!(!sr.parallel_outputs_enabled());

        for _ in 0..10 {
            sr.shift_in(true);
        }

        // E high masks only the parallel output pins.
        assert_eq!(sr.parallel_out(), 0);
        assert!(!sr.output_bit(9));
        assert!(sr.serial_out());

        sr.set_enable_pin(false);
        assert!(!sr.enable_pin());
        assert!(sr.parallel_outputs_enabled());
        assert_eq!(sr.parallel_out(), 0x3FF);
        assert!(sr.output_bit(9));
        assert!(sr.serial_out());
    }

    #[test]
    fn compatibility_enable_wrapper_controls_parallel_output_visibility() {
        let mut sr = I4003::new();
        sr.shift_in(true);

        sr.set_enable(false);
        assert!(!sr.is_enabled());
        assert_eq!(sr.parallel_out(), 0);

        sr.set_enable(true);
        assert!(sr.is_enabled());
        assert_eq!(sr.parallel_out(), 1);
    }

    // --- B.1.4: Output bit accessor ---

    #[test]
    fn output_bit_accessor() {
        let mut sr = I4003::new();
        // Shift in pattern: bit 0 = 1, bit 1 = 0 (shift in 0 then 1)
        sr.shift_in(true); // bit 0 = 1
        sr.shift_in(false); // bit 0 = 0, bit 1 = 1

        assert!(!sr.output_bit(0));
        assert!(sr.output_bit(1));
        assert!(!sr.output_bit(2));
        assert!(!sr.output_bit(10)); // out of range
    }

    // --- B.1.5: Data sampled at rising edge, not before ---

    #[test]
    fn data_sampled_at_rising_edge() {
        let mut sr = I4003::new();

        // Set data to 1 while clock is low
        sr.set_data_in(true);
        sr.set_clock(false);

        // Change data to 0 just before rising edge
        sr.set_data_in(false);
        sr.set_clock(true);

        // The shifted-in bit should be 0 (value at rising edge)
        assert_eq!(sr.parallel_out(), 0);

        // Now: data=1 at rising edge
        sr.set_clock(false);
        sr.set_data_in(true);
        sr.set_clock(true);
        assert_eq!(sr.parallel_out(), 1);
    }

    #[test]
    fn chip_trait_name() {
        use crate::Chip;
        let sr = I4003::new();
        assert_eq!(sr.name(), "4003");
    }

    // --- Solver Bridge tests ---

    use mcs4_core::bridge::ChipSolverBridge;

    #[test]
    fn bridge_subcircuit_names_count() {
        let sr = I4003::new();
        let names = sr.subcircuit_names();
        assert_eq!(names.len(), 5);
        assert!(names.contains(&"CLOCK"));
        assert!(names.contains(&"DATA"));
    }

    #[test]
    fn bridge_fidelity_default() {
        let sr = I4003::new();
        assert_eq!(sr.fidelity(), mcs4_core::SimulationFidelity::Behavioral);
    }

    #[test]
    fn bridge_clock_subcircuit_has_2_transistors() {
        let sr = I4003::new();
        let g = sr.subcircuit("CLOCK").expect("CLOCK subcircuit");
        assert_eq!(g.transistor_count(), 2);
    }

    #[test]
    fn bridge_custom_subcircuit_has_9_transistors() {
        let sr = I4003::new();
        let g = sr.subcircuit("custom").expect("custom subcircuit");
        assert_eq!(g.transistor_count(), 9);
    }

    #[test]
    fn bridge_pin_map_has_entries() {
        let sr = I4003::new();
        let pins = sr.pin_map();
        assert!(pins.len() >= 10);
        assert!(pins.iter().any(|p| p.name == "CLOCK"));
    }
}
