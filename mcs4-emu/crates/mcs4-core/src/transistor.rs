//! Transistor-level simulation primitives (STUB)
//!
//! This module provides placeholder types for future transistor-level
//! simulation. The Intel 4004 was implemented in 10um pMOS technology
//! with ~2300 transistors.
//!
//! TODO: Implement SPICE-like switch-level simulation

use crate::timing::Time;

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
        // pMOS is on when Vgs < Vth (both negative)
        let vgs = self.vg - self.vs;
        vgs < self.vth
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
        Self {
            fet,
            vth_dep: 1.0,
        }
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

/// Transistor-level circuit (placeholder for future implementation)
#[derive(Clone, Debug, Default)]
pub struct TransistorCircuit {
    /// All transistors
    pub transistors: Vec<PmosFet>,

    /// All nodes
    pub nodes: Vec<CircuitNode>,

    /// Simulation timestep (picoseconds)
    pub timestep: Time,
}

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

    /// Simulate one timestep (placeholder)
    pub fn step(&mut self) -> Time {
        // TODO: Implement nodal analysis or event-driven simulation
        // For now, this is just a placeholder
        self.timestep
    }
}

/// Builder for creating transistor-level circuits from schematics
pub struct CircuitBuilder {
    circuit: TransistorCircuit,
}

impl CircuitBuilder {
    pub fn new() -> Self {
        let mut circuit = TransistorCircuit::new();
        // Add power supply nodes
        circuit.add_node(CircuitNode::vdd());
        circuit.add_node(CircuitNode::vss());
        Self { circuit }
    }

    /// Add an inverter subcircuit
    pub fn inverter(&mut self, _input: &str, _output: &str) -> &mut Self {
        // TODO: Add transistors for inverter
        // Typical: 1 enhancement driver + 1 depletion load
        self
    }

    /// Add a NAND2 subcircuit
    pub fn nand2(&mut self, _a: &str, _b: &str, _output: &str) -> &mut Self {
        // TODO: Add transistors for 2-input NAND
        // Typical: 2 series enhancement drivers + 1 depletion load
        self
    }

    /// Build the final circuit
    pub fn build(self) -> TransistorCircuit {
        self.circuit
    }
}

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
    fn test_ron_scaling() {
        let narrow = PmosFet::new(5.0, 10.0);
        let wide = PmosFet::new(20.0, 10.0);

        // Wider transistor should have lower Ron
        assert!(wide.ron < narrow.ron);
    }

    #[test]
    fn test_circuit_builder() {
        let circuit = CircuitBuilder::new()
            .inverter("IN", "OUT")
            .build();

        // Should have at least VDD and VSS nodes
        assert!(circuit.nodes.len() >= 2);
    }
}
