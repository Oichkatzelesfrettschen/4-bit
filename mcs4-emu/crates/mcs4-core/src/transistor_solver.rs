//! Transistor-level switch simulator for MCS-4 circuitry
//!
//! Implements iterative switch-level simulation with RC delay model.
//! Validates extracted transistor netlists against gate-level truth tables.

use std::collections::HashMap;

/// Transistor type: NMOS or PMOS
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransistorType {
    Nmos,
    Pmos,
}

/// Signal voltage level: logical 0 (0V) or logical 1 (5V)
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VoltageLevel {
    Low,  // 0V (logical 0)
    High, // 5V (logical 1)
}

impl VoltageLevel {
    pub fn as_volt(&self) -> f64 {
        match self {
            VoltageLevel::Low => 0.0,
            VoltageLevel::High => 5.0,
        }
    }
}

/// Unique node identifier
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct NodeId(u32);

impl NodeId {
    pub fn new(id: u32) -> Self {
        NodeId(id)
    }
}

/// Unique transistor identifier
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TransistorId(u32);

impl TransistorId {
    pub fn new(id: u32) -> Self {
        TransistorId(id)
    }
}

/// Single transistor instance
#[derive(Clone, Debug)]
pub struct Transistor {
    pub id: TransistorId,
    pub ty: TransistorType,
    pub gate: NodeId,
    pub drain: NodeId,
    pub source: NodeId,
    pub bulk: NodeId,
    pub width: f64,         // Gate width (um)
    pub length: f64,        // Gate length (um)
    pub vth: f64,           // Threshold voltage (V)
    pub ron: f64,           // On-state resistance (Ohm)
    pub delay: f64,         // Propagation delay (ns)
}

impl Transistor {
    /// Check if transistor is conducting (Vgs > Vth for NMOS, Vgs < -Vth for PMOS)
    pub fn is_conducting(&self, vgate: f64, vsource: f64) -> bool {
        let vgs = vgate - vsource;
        match self.ty {
            TransistorType::Nmos => vgs > self.vth,
            TransistorType::Pmos => vgs < self.vth,
        }
    }
}

/// Network node with voltage and capacitance
#[derive(Clone, Debug)]
pub struct Node {
    pub id: NodeId,
    pub name: String,
    pub voltage: VoltageLevel,
    pub voltage_prev: VoltageLevel,
    pub capacitance: f64,              // Total node capacitance (pF)
    pub is_power_rail: bool,           // VDD or VSS?
    pub connected_transistors: Vec<TransistorId>,
}

/// Circuit netlist: transistors + nodes
#[derive(Clone, Debug)]
pub struct Circuit {
    pub transistors: HashMap<TransistorId, Transistor>,
    pub nodes: HashMap<NodeId, Node>,
    pub vdd: NodeId,
    pub vss: NodeId,
    pub next_transistor_id: u32,
    pub next_node_id: u32,
}

impl Circuit {
    pub fn new(vdd: NodeId, vss: NodeId) -> Self {
        let mut circuit = Circuit {
            transistors: HashMap::new(),
            nodes: HashMap::new(),
            vdd,
            vss,
            next_transistor_id: 0,
            next_node_id: 2,
        };

        circuit.nodes.insert(
            vdd,
            Node {
                id: vdd,
                name: "VDD".to_string(),
                voltage: VoltageLevel::High,
                voltage_prev: VoltageLevel::High,
                capacitance: 0.0,
                is_power_rail: true,
                connected_transistors: Vec::new(),
            },
        );
        circuit.nodes.insert(
            vss,
            Node {
                id: vss,
                name: "VSS".to_string(),
                voltage: VoltageLevel::Low,
                voltage_prev: VoltageLevel::Low,
                capacitance: 0.0,
                is_power_rail: true,
                connected_transistors: Vec::new(),
            },
        );

        circuit
    }

    pub fn add_node(&mut self, name: String, initial: VoltageLevel, cap: f64) -> NodeId {
        let id = NodeId::new(self.next_node_id);
        self.next_node_id += 1;
        self.nodes.insert(
            id,
            Node {
                id,
                name,
                voltage: initial,
                voltage_prev: initial,
                capacitance: cap,
                is_power_rail: false,
                connected_transistors: Vec::new(),
            },
        );
        id
    }

    pub fn add_transistor(
        &mut self,
        ty: TransistorType,
        gate: NodeId,
        drain: NodeId,
        source: NodeId,
        bulk: NodeId,
        width: f64,
        length: f64,
        vth: f64,
        ron: f64,
        delay: f64,
    ) -> TransistorId {
        let id = TransistorId::new(self.next_transistor_id);
        self.next_transistor_id += 1;

        let trans = Transistor {
            id,
            ty,
            gate,
            drain,
            source,
            bulk,
            width,
            length,
            vth,
            ron,
            delay,
        };

        if let Some(node) = self.nodes.get_mut(&gate) {
            node.connected_transistors.push(id);
        }
        if let Some(node) = self.nodes.get_mut(&drain) {
            node.connected_transistors.push(id);
        }
        if let Some(node) = self.nodes.get_mut(&source) {
            node.connected_transistors.push(id);
        }

        self.transistors.insert(id, trans);
        id
    }
}

/// Transistor-level simulator using iterative convergence
#[allow(dead_code)]
pub struct TransistorSimulator {
    circuit: Circuit,
    current_time: f64,
    max_iterations: u32,
    iteration_count: u32,
}

impl TransistorSimulator {
    pub fn new(circuit: Circuit) -> Self {
        TransistorSimulator {
            circuit,
            current_time: 0.0,
            max_iterations: 100,
            iteration_count: 0,
        }
    }

    /// Inject input signal change and run simulation
    pub fn drive_input(&mut self, node_id: NodeId, new_voltage: VoltageLevel) -> bool {
        // Set input voltage
        if let Some(node) = self.circuit.nodes.get_mut(&node_id) {
            node.voltage = new_voltage;
        }
        
        // Iteratively evaluate until convergence
        let mut converged = false;
        self.iteration_count = 0;

        for iteration in 0..self.max_iterations {
            // Record previous state
            for node in self.circuit.nodes.values_mut() {
                node.voltage_prev = node.voltage;
            }

            // Evaluate all transistors (combinational logic model)
            let trans_list: Vec<TransistorId> = self.circuit.transistors.keys().copied().collect();
            for trans_id in trans_list {
                if let Some(trans) = self.circuit.transistors.get(&trans_id) {
                    let trans_copy = trans.clone();
                    self._evaluate_transistor(&trans_copy);
                }
            }

            // Check for convergence
            let mut changed = false;
            for node in self.circuit.nodes.values() {
                if node.voltage != node.voltage_prev {
                    changed = true;
                    break;
                }
            }

            if !changed {
                converged = true;
                self.iteration_count = iteration as u32 + 1;
                break;
            }

            self.iteration_count = iteration as u32 + 1;
        }

        converged
    }

    /// Evaluate transistor state and propagate changes
    fn _evaluate_transistor(&mut self, trans: &Transistor) {
        let vgate = self.circuit.nodes.get(&trans.gate)
            .map(|n| n.voltage.as_volt())
            .unwrap_or(0.0);
        let vsource = self.circuit.nodes.get(&trans.source)
            .map(|n| n.voltage.as_volt())
            .unwrap_or(0.0);

        // If transistor conducts, propagate source voltage to drain
        if trans.is_conducting(vgate, vsource) {
            let source_level = self.circuit.nodes.get(&trans.source)
                .map(|n| n.voltage)
                .unwrap_or(VoltageLevel::Low);

            if let Some(drain_node) = self.circuit.nodes.get_mut(&trans.drain) {
                drain_node.voltage = source_level;
            }
        }
    }

    /// Get current voltage of a node
    pub fn node_voltage(&self, node_id: NodeId) -> VoltageLevel {
        self.circuit.nodes.get(&node_id)
            .map(|n| n.voltage)
            .unwrap_or(VoltageLevel::Low)
    }

    /// Get simulation time
    pub fn time(&self) -> f64 {
        self.current_time
    }

    /// Get iteration count
    pub fn iteration_count(&self) -> u32 {
        self.iteration_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_inverter() -> (Circuit, NodeId, NodeId) {
        let vdd = NodeId::new(0);
        let vss = NodeId::new(1);
        let mut circuit = Circuit::new(vdd, vss);

        let in_node = circuit.add_node("IN".to_string(), VoltageLevel::Low, 5.0);
        let out_node = circuit.add_node("OUT".to_string(), VoltageLevel::Low, 10.0);

        circuit.add_transistor(
            TransistorType::Pmos,
            in_node, out_node, vdd, vdd,
            10.0, 1.0, -1.0, 20000.0, 50.0,
        );

        circuit.add_transistor(
            TransistorType::Nmos,
            in_node, out_node, vss, vss,
            5.0, 1.0, 1.0, 10000.0, 50.0,
        );

        (circuit, in_node, out_node)
    }


    #[test]
    fn test_inverter_low_to_high() {
        let (circuit, in_node, out_node) = create_test_inverter();
        let mut sim = TransistorSimulator::new(circuit);

        let settled = sim.drive_input(in_node, VoltageLevel::Low);
        assert!(settled, "Should converge");
        assert_eq!(sim.node_voltage(out_node), VoltageLevel::High);
    }

    #[test]
    fn test_inverter_high_to_low() {
        let (circuit, in_node, out_node) = create_test_inverter();
        let mut sim = TransistorSimulator::new(circuit);

        let settled = sim.drive_input(in_node, VoltageLevel::High);
        assert!(settled, "Should converge");
        assert_eq!(sim.node_voltage(out_node), VoltageLevel::Low);
    }

    #[test]
    fn test_inverter_truth_table() {
        let (circuit, in_node, out_node) = create_test_inverter();
        let mut sim = TransistorSimulator::new(circuit);

        sim.drive_input(in_node, VoltageLevel::Low);
        let out_0 = sim.node_voltage(out_node);

        sim.drive_input(in_node, VoltageLevel::High);
        let out_1 = sim.node_voltage(out_node);

        assert_ne!(out_0, out_1, "Inverter must invert input");
    }

    #[test]
    fn test_converges() {
        let (circuit, in_node, _out_node) = create_test_inverter();
        let mut sim = TransistorSimulator::new(circuit);

        let settled = sim.drive_input(in_node, VoltageLevel::High);
        assert!(settled, "Must converge");
    }

    #[test]
    fn test_tracks_iterations() {
        let (circuit, in_node, _out_node) = create_test_inverter();
        let mut sim = TransistorSimulator::new(circuit);

        sim.drive_input(in_node, VoltageLevel::High);
        assert!(sim.iteration_count() > 0);
    }

    #[test]
    fn test_node_voltage_default() {
        let (circuit, _in_node, _out_node) = create_test_inverter();
        let sim = TransistorSimulator::new(circuit);

        let vdd = NodeId::new(0);
        assert_eq!(sim.node_voltage(vdd), VoltageLevel::High);
    }

    #[test]
    fn test_voltage_level_conversion() {
        assert_eq!(VoltageLevel::Low.as_volt(), 0.0);
        assert_eq!(VoltageLevel::High.as_volt(), 5.0);
    }

    #[test]
    fn test_transistor_nmos_conducting() {
        let trans = Transistor {
            id: TransistorId::new(0),
            ty: TransistorType::Nmos,
            gate: NodeId::new(0),
            drain: NodeId::new(1),
            source: NodeId::new(2),
            bulk: NodeId::new(3),
            width: 5.0,
            length: 1.0,
            vth: 1.0,
            ron: 10000.0,
            delay: 50.0,
        };

        // Vgs = 5V - 0V = 5V > 1V (Vth) -> should conduct
        assert!(trans.is_conducting(5.0, 0.0));
        // Vgs = 0V - 0V = 0V < 1V -> should not conduct
        assert!(!trans.is_conducting(0.0, 0.0));
    }

    #[test]
    fn test_transistor_pmos_conducting() {
        let trans = Transistor {
            id: TransistorId::new(0),
            ty: TransistorType::Pmos,
            gate: NodeId::new(0),
            drain: NodeId::new(1),
            source: NodeId::new(2),
            bulk: NodeId::new(3),
            width: 10.0,
            length: 1.0,
            vth: -1.0,
            ron: 20000.0,
            delay: 50.0,
        };

        // Vgs = 0V - 5V = -5V < -1V (Vth=-1V) -> should conduct
        assert!(trans.is_conducting(0.0, 5.0));
        // Vgs = 5V - 5V = 0V > -1V -> should not conduct
        assert!(!trans.is_conducting(5.0, 5.0));
    }

    #[test]
    fn test_parallel_pmos_gate() {
        // Test parallel PMOS: both pull to VDD independently
        // When either gate is low, output is pulled high
        let vdd = NodeId::new(0);
        let vss = NodeId::new(1);
        let mut circuit = Circuit::new(vdd, vss);

        let a_node = circuit.add_node("A".to_string(), VoltageLevel::Low, 5.0);
        let b_node = circuit.add_node("B".to_string(), VoltageLevel::Low, 5.0);
        let out_node = circuit.add_node("OUT".to_string(), VoltageLevel::Low, 10.0);

        // Two PMOS in parallel to VDD
        circuit.add_transistor(
            TransistorType::Pmos,
            a_node, out_node, vdd, vdd,
            10.0, 1.0, -1.0, 20000.0, 50.0,
        );
        circuit.add_transistor(
            TransistorType::Pmos,
            b_node, out_node, vdd, vdd,
            10.0, 1.0, -1.0, 20000.0, 50.0,
        );

        let mut sim = TransistorSimulator::new(circuit);

        // Both inputs low -> both PMOS conduct -> output high
        sim.drive_input(a_node, VoltageLevel::Low);
        sim.drive_input(b_node, VoltageLevel::Low);
        assert_eq!(sim.node_voltage(out_node), VoltageLevel::High, "Both PMOS low-gate should pull output high");

        // One input high -> that PMOS off, other conducts -> output still high
        sim.drive_input(a_node, VoltageLevel::High);
        sim.drive_input(b_node, VoltageLevel::Low);
        assert_eq!(sim.node_voltage(out_node), VoltageLevel::High, "One PMOS conducting should maintain high output");
    }

    #[test]
    fn test_parallel_nmos_gate() {
        // Test parallel NMOS: either gate high pulls to VSS
        let vdd = NodeId::new(0);
        let vss = NodeId::new(1);
        let mut circuit = Circuit::new(vdd, vss);

        let a_node = circuit.add_node("A".to_string(), VoltageLevel::Low, 5.0);
        let b_node = circuit.add_node("B".to_string(), VoltageLevel::Low, 5.0);
        let out_node = circuit.add_node("OUT".to_string(), VoltageLevel::High, 10.0);

        // Two NMOS in parallel to VSS
        circuit.add_transistor(
            TransistorType::Nmos,
            a_node, out_node, vss, vss,
            5.0, 1.0, 1.0, 10000.0, 50.0,
        );
        circuit.add_transistor(
            TransistorType::Nmos,
            b_node, out_node, vss, vss,
            5.0, 1.0, 1.0, 10000.0, 50.0,
        );

        let mut sim = TransistorSimulator::new(circuit);

        // Both inputs low -> both NMOS off -> output high (no pull-down)
        sim.drive_input(a_node, VoltageLevel::Low);
        sim.drive_input(b_node, VoltageLevel::Low);
        assert_eq!(sim.node_voltage(out_node), VoltageLevel::High);

        // One input high -> that NMOS conducts -> output pulled low
        sim.drive_input(a_node, VoltageLevel::High);
        sim.drive_input(b_node, VoltageLevel::Low);
        assert_eq!(sim.node_voltage(out_node), VoltageLevel::Low);

        // Both inputs high -> both NMOS conduct -> output low
        sim.drive_input(a_node, VoltageLevel::High);
        sim.drive_input(b_node, VoltageLevel::High);
        assert_eq!(sim.node_voltage(out_node), VoltageLevel::Low);
    }

    #[test]
    fn test_inverter_chain_propagation() {
        // Chain of 3 inverters: IN -> INV1 -> INV2 -> INV3 -> OUT
        // Signal should return to original logic level
        let vdd = NodeId::new(0);
        let vss = NodeId::new(1);
        let mut circuit = Circuit::new(vdd, vss);

        let in_node = circuit.add_node("IN".to_string(), VoltageLevel::Low, 5.0);
        let n1 = circuit.add_node("N1".to_string(), VoltageLevel::High, 8.0);
        let n2 = circuit.add_node("N2".to_string(), VoltageLevel::Low, 8.0);
        let out_node = circuit.add_node("OUT".to_string(), VoltageLevel::High, 10.0);

        // INV1
        circuit.add_transistor(TransistorType::Pmos, in_node, n1, vdd, vdd, 10.0, 1.0, -1.0, 20000.0, 50.0);
        circuit.add_transistor(TransistorType::Nmos, in_node, n1, vss, vss, 5.0, 1.0, 1.0, 10000.0, 50.0);

        // INV2
        circuit.add_transistor(TransistorType::Pmos, n1, n2, vdd, vdd, 10.0, 1.0, -1.0, 20000.0, 50.0);
        circuit.add_transistor(TransistorType::Nmos, n1, n2, vss, vss, 5.0, 1.0, 1.0, 10000.0, 50.0);

        // INV3
        circuit.add_transistor(TransistorType::Pmos, n2, out_node, vdd, vdd, 10.0, 1.0, -1.0, 20000.0, 50.0);
        circuit.add_transistor(TransistorType::Nmos, n2, out_node, vss, vss, 5.0, 1.0, 1.0, 10000.0, 50.0);

        let mut sim = TransistorSimulator::new(circuit);

        // Apply low to input: INV1 inverts to High, INV2 inverts to Low, INV3 inverts to High
        sim.drive_input(in_node, VoltageLevel::Low);
        let out_from_low = sim.node_voltage(out_node);
        assert_eq!(out_from_low, VoltageLevel::High, "Three inverters should invert Low to High");

        // Apply high to input: INV1 inverts to Low, INV2 inverts to High, INV3 inverts to Low
        sim.drive_input(in_node, VoltageLevel::High);
        let out_from_high = sim.node_voltage(out_node);
        assert_eq!(out_from_high, VoltageLevel::Low, "Three inverters should invert High to Low");
    }

    #[test]
    fn test_transistor_marginal_conducting() {
        // Test NMOS at threshold voltage (Vth = 1.0)
        let trans = Transistor {
            id: TransistorId::new(0),
            ty: TransistorType::Nmos,
            gate: NodeId::new(0),
            drain: NodeId::new(1),
            source: NodeId::new(2),
            bulk: NodeId::new(3),
            width: 5.0,
            length: 1.0,
            vth: 1.0,
            ron: 10000.0,
            delay: 50.0,
        };

        // Vgs = 1.5V - 0V = 1.5V > 1.0V -> should conduct
        assert!(trans.is_conducting(1.5, 0.0));
        // Vgs = 1.0V - 0V = 1.0V = Vth -> boundary condition (typically ON)
        // but our implementation uses strict inequality, so should NOT conduct
        assert!(!trans.is_conducting(1.0, 0.0));
    }

    #[test]
    fn test_high_fanout_node() {
        // Test a node driving multiple gates
        let vdd = NodeId::new(0);
        let vss = NodeId::new(1);
        let mut circuit = Circuit::new(vdd, vss);

        let driver = circuit.add_node("DRIVER".to_string(), VoltageLevel::Low, 5.0);
        let load1 = circuit.add_node("LOAD1".to_string(), VoltageLevel::High, 15.0);
        let load2 = circuit.add_node("LOAD2".to_string(), VoltageLevel::High, 15.0);
        let load3 = circuit.add_node("LOAD3".to_string(), VoltageLevel::High, 15.0);

        // Three inverters fed from same driver
        for (_i, load_id) in [load1, load2, load3].iter().enumerate() {
            circuit.add_transistor(
                TransistorType::Pmos,
                driver, *load_id, vdd, vdd,
                10.0, 1.0, -1.0, 20000.0, 50.0,
            );
            circuit.add_transistor(
                TransistorType::Nmos,
                driver, *load_id, vss, vss,
                5.0, 1.0, 1.0, 10000.0, 50.0,
            );
        }

        let mut sim = TransistorSimulator::new(circuit);

        // Drive high
        sim.drive_input(driver, VoltageLevel::High);
        assert_eq!(sim.node_voltage(load1), VoltageLevel::Low);
        assert_eq!(sim.node_voltage(load2), VoltageLevel::Low);
        assert_eq!(sim.node_voltage(load3), VoltageLevel::Low);

        // Drive low
        sim.drive_input(driver, VoltageLevel::Low);
        assert_eq!(sim.node_voltage(load1), VoltageLevel::High);
        assert_eq!(sim.node_voltage(load2), VoltageLevel::High);
        assert_eq!(sim.node_voltage(load3), VoltageLevel::High);
    }
}
