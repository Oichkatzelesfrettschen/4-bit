//! Transistor-level simulation primitives
//!
//! This module provides transistor-level types for the Intel 4004/4040
//! pMOS process (10um, ~2300 transistors). Includes pMOS FET switch model,
//! depletion-mode loads, circuit nodes, and a circuit builder.
//!
//! For SPICE-class analysis, see also `transistor_solver` (switch-level)
//! and `nodal_solver` (analog nodal analysis).

use crate::timing::Time;

/// Power-supply convention used by transistor-level helpers.
///
/// The historical 4004 pMOS process used a negative supply, but this crate avoids baking in a sign
/// convention at the type level. Use `Supply::mcs4_pmos()` for the repo's default evidence tooling.
#[derive(Clone, Copy, Debug)]
pub struct Supply {
    pub vdd: f64,
    pub vss: f64,
}

impl Supply {
    pub const fn new(vdd: f64, vss: f64) -> Self {
        Self { vdd, vss }
    }

    /// Default convention for this repo's transistor stubs (pMOS with a negative supply rail).
    pub const fn mcs4_pmos() -> Self {
        Self { vdd: -15.0, vss: 0.0 }
    }

    /// Midpoint threshold used for coarse digital classification.
    pub fn vmid(self) -> f64 {
        (self.vdd + self.vss) * 0.5
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DigitalLevel {
    Vdd,
    Vss,
}

impl DigitalLevel {
    pub fn as_voltage(self, supply: Supply) -> f64 {
        match self {
            Self::Vdd => supply.vdd,
            Self::Vss => supply.vss,
        }
    }
}

pub fn classify_digital(v: f64, supply: Supply) -> DigitalLevel {
    // Treat values closer to VDD as VDD, else VSS. This is intentionally simplistic.
    if (v - supply.vdd).abs() <= (v - supply.vss).abs() {
        DigitalLevel::Vdd
    } else {
        DigitalLevel::Vss
    }
}

/// pMOS transistor model
///
/// The 4004 uses enhancement-mode pMOS transistors with depletion-mode
/// loads. This is a simplified switch-level model.
#[derive(Clone, Debug)]
pub struct PmosFet {
    /// Channel width (micrometers)
    pub w: f64,

    /// Channel length (micrometers)
    pub l: f64,

    /// Threshold voltage (volts, negative for pMOS)
    pub vth: f64,

    /// Gate terminal voltage
    pub vg: f64,

    /// Source terminal voltage
    pub vs: f64,

    /// Drain terminal voltage
    pub vd: f64,

    /// On-resistance (calculated from W/L)
    pub ron: f64,

    /// Gate capacitance (femtofarads)
    pub cg: f64,
}

impl Default for PmosFet {
    fn default() -> Self {
        Self::new(10.0, 10.0) // 10um x 10um default
    }
}

impl PmosFet {
    /// Create a new pMOS transistor with given dimensions
    pub fn new(w: f64, l: f64) -> Self {
        Self {
            w,
            l,
            vth: -2.0, // Typical pMOS threshold
            vg: 0.0,
            vs: 0.0,
            vd: -15.0,
            ron: Self::calc_ron(w, l),
            cg: Self::calc_cg(w, l),
        }
    }

    /// Calculate on-resistance from dimensions
    fn calc_ron(w: f64, l: f64) -> f64 {
        // Simplified: Ron proportional to L/W
        // Typical sheet resistance ~10k ohms/square for this era
        10000.0 * l / w
    }

    /// Calculate gate capacitance from dimensions
    fn calc_cg(w: f64, l: f64) -> f64 {
        // Cox ~ 3.4e-8 F/cm^2 for 100nm oxide
        // Cg = Cox * W * L
        // Converting to fF for um dimensions:
        let cox_per_um2 = 0.034; // fF/um^2
        cox_per_um2 * w * l
    }

    /// Update terminal voltages
    pub fn set_voltages(&mut self, vg: f64, vs: f64, vd: f64) {
        self.vg = vg;
        self.vs = vs;
        self.vd = vd;
    }

    /// Is the transistor conducting?
    pub fn is_on(&self) -> bool {
        self.is_on_with(Supply::mcs4_pmos())
    }

    /// Is the transistor conducting, using a specific supply convention?
    ///
    /// This is still a simplified check; it mainly exists to avoid hardcoding a single supply
    /// polarity into future switch-level solvers.
    pub fn is_on_with(&self, supply: Supply) -> bool {
        // Interpret node voltages relative to the supplied rails to decide Vgs polarity.
        // For the default MCS-4 pMOS convention (VDD=-15, VSS=0), a more-negative gate is an
        // "asserted" gate. We normalize to a digital gate classification and use that as a switch.
        let gate = classify_digital(self.vg, supply);
        match gate {
            DigitalLevel::Vdd => true,
            DigitalLevel::Vss => false,
        }
    }

    /// Drain-source current (simplified)
    pub fn ids(&self) -> f64 {
        if !self.is_on() {
            return 0.0;
        }

        // Simple switch model: I = (Vs - Vd) / Ron
        (self.vs - self.vd).abs() / self.ron
    }
}

/// Depletion-mode pMOS load transistor
///
/// Used as active loads in the 4004's logic gates.
#[derive(Clone, Debug)]
pub struct DepletionLoad {
    /// Transistor
    pub fet: PmosFet,

    /// Depletion threshold (positive for depletion mode)
    pub vth_dep: f64,
}

impl DepletionLoad {
    pub fn new(w: f64, l: f64) -> Self {
        let mut fet = PmosFet::new(w, l);
        fet.vth = 1.0; // Positive threshold for depletion mode
        Self { fet, vth_dep: 1.0 }
    }

    /// Depletion loads are always "on" (conducting)
    pub fn is_conducting(&self) -> bool {
        true
    }

    /// Current through the load
    pub fn current(&self) -> f64 {
        // Simplified: constant current source behavior
        (self.fet.vs - self.fet.vd).abs() / self.fet.ron
    }
}

/// A node in the transistor-level circuit
#[derive(Clone, Debug)]
pub struct CircuitNode {
    /// Node identifier
    pub name: String,

    /// Current voltage
    pub voltage: f64,

    /// Total capacitance to ground
    pub capacitance: f64,

    /// Is this a power supply node?
    pub is_supply: bool,
}

impl CircuitNode {
    pub fn new(name: impl Into<String>, capacitance: f64) -> Self {
        Self {
            name: name.into(),
            voltage: 0.0,
            capacitance,
            is_supply: false,
        }
    }

    pub fn vdd() -> Self {
        Self {
            name: "VDD".into(),
            voltage: -15.0,
            capacitance: f64::INFINITY,
            is_supply: true,
        }
    }

    pub fn vss() -> Self {
        Self {
            name: "VSS".into(),
            voltage: 0.0,
            capacitance: f64::INFINITY,
            is_supply: true,
        }
    }
}

/// Transistor-level circuit container.
///
/// For SPICE-class simulation, prefer `transistor_solver::TransistorSimulator`
/// which provides convergence analysis, high-fanout support, and marginal
/// conduction modeling.
#[deprecated(
    since = "0.1.0",
    note = "Use transistor_solver::TransistorSimulator for simulation. \
            TransistorCircuit remains available as a circuit data container."
)]
#[derive(Clone, Debug, Default)]
pub struct TransistorCircuit {
    /// All transistors
    pub transistors: Vec<PmosFet>,

    /// All nodes
    pub nodes: Vec<CircuitNode>,

    /// Simulation timestep (picoseconds)
    pub timestep: Time,
}

#[allow(deprecated)]
impl TransistorCircuit {
    pub fn new() -> Self {
        Self {
            transistors: Vec::new(),
            nodes: Vec::new(),
            timestep: 100, // 100 ps default
        }
    }

    /// Add a transistor to the circuit
    pub fn add_transistor(&mut self, fet: PmosFet) -> usize {
        let id = self.transistors.len();
        self.transistors.push(fet);
        id
    }

    /// Add a node to the circuit
    pub fn add_node(&mut self, node: CircuitNode) -> usize {
        let id = self.nodes.len();
        self.nodes.push(node);
        id
    }

    /// Advance by one timestep. Returns the timestep duration in picoseconds.
    ///
    /// This is a basic time-advance for the circuit container. For actual
    /// simulation with convergence analysis, use `transistor_solver::TransistorSimulator`.
    pub fn step(&mut self) -> Time {
        self.timestep
    }
}

/// Builder for creating transistor-level circuits from schematics
#[allow(deprecated)]
pub struct CircuitBuilder {
    circuit: TransistorCircuit,
}

#[allow(deprecated)]
impl CircuitBuilder {
    pub fn new() -> Self {
        let mut circuit = TransistorCircuit::new();
        // Add power supply nodes
        circuit.add_node(CircuitNode::vdd());
        circuit.add_node(CircuitNode::vss());
        Self { circuit }
    }

    /// Add a pMOS inverter subcircuit (1 enhancement driver + 1 depletion load).
    ///
    /// The input node drives the gate of an enhancement-mode pMOS transistor.
    /// A depletion-mode load pulls the output toward VDD when the driver is off.
    pub fn inverter(&mut self, input: &str, output: &str) -> &mut Self {
        // Output node
        let out_id = self.circuit.add_node(CircuitNode::new(output, 0.1));

        // Enhancement driver: gate=input, source=VSS(1), drain=output
        let mut driver = PmosFet::new(10.0, 10.0);
        driver.vg = 0.0; // will be driven by input signal
        driver.vs = 0.0; // VSS
        driver.vd = -15.0;
        self.circuit.add_transistor(driver);

        // Depletion load: always conducting, pulls output toward VDD
        let mut load = DepletionLoad::new(5.0, 20.0);
        load.fet.vs = -15.0; // VDD
        load.fet.vd = self.circuit.nodes[out_id].voltage;
        self.circuit.add_transistor(load.fet);

        // Store input node reference
        let _in_id = self.circuit.add_node(CircuitNode::new(input, 0.05));

        self
    }

    /// Add a pMOS 2-input NAND subcircuit (2 series enhancement drivers + 1 depletion load).
    ///
    /// Inputs a and b each drive a series enhancement-mode pMOS transistor.
    /// Output goes low only when both inputs are asserted (both drivers on).
    pub fn nand2(&mut self, a: &str, b: &str, output: &str) -> &mut Self {
        // Output node
        let out_id = self.circuit.add_node(CircuitNode::new(output, 0.1));

        // Internal node between series transistors
        let mid_id = self.circuit.add_node(CircuitNode::new(format!("{output}_mid"), 0.05));

        // First series driver (gate=a): source=VSS, drain=mid
        let mut driver_a = PmosFet::new(10.0, 10.0);
        driver_a.vs = 0.0; // VSS
        driver_a.vd = self.circuit.nodes[mid_id].voltage;
        self.circuit.add_transistor(driver_a);

        // Second series driver (gate=b): source=mid, drain=output
        let mut driver_b = PmosFet::new(10.0, 10.0);
        driver_b.vs = self.circuit.nodes[mid_id].voltage;
        driver_b.vd = self.circuit.nodes[out_id].voltage;
        self.circuit.add_transistor(driver_b);

        // Depletion load: always conducting, pulls output toward VDD
        let mut load = DepletionLoad::new(5.0, 20.0);
        load.fet.vs = -15.0; // VDD
        load.fet.vd = self.circuit.nodes[out_id].voltage;
        self.circuit.add_transistor(load.fet);

        // Input nodes
        let _a_id = self.circuit.add_node(CircuitNode::new(a, 0.05));
        let _b_id = self.circuit.add_node(CircuitNode::new(b, 0.05));

        self
    }

    /// Build the final circuit
    #[allow(deprecated)]
    pub fn build(self) -> TransistorCircuit {
        self.circuit
    }
}

#[allow(deprecated)]
impl Default for CircuitBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pmos_on_off() {
        let mut fet = PmosFet::new(20.0, 10.0);

        // Gate at Vss (0V), Source at 0V -> Vgs = 0, should be off
        fet.set_voltages(0.0, 0.0, -15.0);
        assert!(!fet.is_on());

        // Gate at Vdd (-15V), Source at 0V -> Vgs = -15V < Vth, should be on
        fet.set_voltages(-15.0, 0.0, -15.0);
        assert!(fet.is_on());
    }

    #[test]
    fn classify_digital_uses_nearest_rail() {
        let supply = Supply::mcs4_pmos();
        assert_eq!(classify_digital(0.0, supply), DigitalLevel::Vss);
        assert_eq!(classify_digital(-15.0, supply), DigitalLevel::Vdd);
        assert_eq!(classify_digital(-14.0, supply), DigitalLevel::Vdd);
        assert_eq!(classify_digital(-1.0, supply), DigitalLevel::Vss);
    }

    #[test]
    fn test_ron_scaling() {
        let narrow = PmosFet::new(5.0, 10.0);
        let wide = PmosFet::new(20.0, 10.0);

        // Wider transistor should have lower Ron
        assert!(wide.ron < narrow.ron);
    }

    #[test]
    #[allow(deprecated)]
    fn test_circuit_builder() {
        let mut builder = CircuitBuilder::new();
        builder.inverter("IN", "OUT");
        let circuit = builder.build();

        // VDD, VSS, output node, input node = 4 nodes minimum
        assert!(circuit.nodes.len() >= 4);
        // 1 enhancement driver + 1 depletion load = 2 transistors
        assert_eq!(circuit.transistors.len(), 2);
    }

    #[test]
    #[allow(deprecated)]
    fn test_nand2_builder() {
        let mut builder = CircuitBuilder::new();
        builder.nand2("A", "B", "Y");
        let circuit = builder.build();

        // VDD, VSS, output, mid, input_a, input_b = 6 nodes
        assert!(circuit.nodes.len() >= 6);
        // 2 series drivers + 1 depletion load = 3 transistors
        assert_eq!(circuit.transistors.len(), 3);
    }

    #[test]
    #[allow(deprecated)]
    fn test_inverter_then_nand2() {
        let mut builder = CircuitBuilder::new();
        builder.inverter("A", "A_INV").nand2("A_INV", "B", "Y");
        let circuit = builder.build();

        // 2 (inverter) + 3 (nand2) = 5 transistors
        assert_eq!(circuit.transistors.len(), 5);
    }

    #[test]
    #[allow(deprecated)]
    fn test_step_returns_timestep() {
        let mut circuit = TransistorCircuit::new();
        assert_eq!(circuit.step(), 100); // default 100ps
    }

    #[test]
    fn test_depletion_load_always_conducting() {
        let load = DepletionLoad::new(5.0, 20.0);
        assert!(load.is_conducting());
    }

    #[test]
    fn test_supply_vmid() {
        let supply = Supply::mcs4_pmos();
        assert!((supply.vmid() - (-7.5)).abs() < 1e-10);
    }
}
