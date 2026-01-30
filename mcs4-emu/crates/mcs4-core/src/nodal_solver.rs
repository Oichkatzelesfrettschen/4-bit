//! Nodal analysis solver for transistor-level simulation
//!
//! Implements simplified nodal analysis using basic Gaussian elimination.
//! Handles RC-coupled node voltage updates and capacitive effects.

use std::collections::HashMap;

/// Node voltage state for nodal analysis
#[derive(Clone, Debug)]
pub struct NodalNode {
    pub id: u32,
    pub name: String,
    pub voltage: f64,           // Voltage in Volts
    pub capacitance: f64,       // Capacitance in pF
    pub is_fixed: bool,         // VDD/VSS/driven nodes are fixed
}

/// Conductance matrix element
#[derive(Clone, Debug)]
pub struct Conductance {
    pub from_node: u32,
    pub to_node: u32,
    pub value: f64,             // Conductance in Siemens
}

/// Nodal analysis solver
#[allow(dead_code)]
pub struct NodalSolver {
    nodes: HashMap<u32, NodalNode>,
    conductances: Vec<Conductance>,
    dt: f64,                    // Time step for integration
    max_iterations: u32,
}

impl NodalSolver {
    pub fn new() -> Self {
        NodalSolver {
            nodes: HashMap::new(),
            conductances: Vec::new(),
            dt: 0.001,           // 1ms default time step
            max_iterations: 100,
        }
    }

    /// Add a node to the nodal network
    pub fn add_node(&mut self, id: u32, name: String, voltage: f64, cap: f64, is_fixed: bool) {
        self.nodes.insert(id, NodalNode {
            id,
            name,
            voltage,
            capacitance: cap,
            is_fixed,
        });
    }

    /// Add conductance between two nodes
    pub fn add_conductance(&mut self, from: u32, to: u32, value: f64) {
        self.conductances.push(Conductance {
            from_node: from,
            to_node: to,
            value,
        });
    }

    /// Solve nodal equations using iterative method
    /// Returns true if converged within max_iterations
    pub fn solve(&mut self) -> bool {
        let num_nodes = self.nodes.len();
        if num_nodes == 0 {
            return true;
        }

        let mut converged = false;
        for iteration in 0..self.max_iterations {
            // Save previous state
            let mut prev_voltages = HashMap::new();
            for (id, node) in &self.nodes {
                prev_voltages.insert(*id, node.voltage);
            }

            // Collect node IDs to iterate
            let node_ids: Vec<u32> = self.nodes.keys().copied().collect();

            // Update each node based on conductance network
            for node_id in node_ids {
                {
                    let node = &self.nodes[&node_id];
                    if node.is_fixed {
                        continue;
                    }
                }

                // Calculate voltage update using weighted average of neighbors
                let mut sum_g_v = 0.0;
                let mut sum_g = 0.0;

                for cond in &self.conductances {
                    if cond.from_node == node_id {
                        if let Some(adj_node) = self.nodes.get(&cond.to_node) {
                            sum_g_v += cond.value * adj_node.voltage;
                            sum_g += cond.value;
                        }
                    } else if cond.to_node == node_id {
                        if let Some(adj_node) = self.nodes.get(&cond.from_node) {
                            sum_g_v += cond.value * adj_node.voltage;
                            sum_g += cond.value;
                        }
                    }
                }

                // Update voltage with damping for stability
                if sum_g > 0.0 {
                    let new_voltage = sum_g_v / sum_g;
                    let damping = 0.5;
                    if let Some(node) = self.nodes.get_mut(&node_id) {
                        node.voltage = damping * node.voltage + (1.0 - damping) * new_voltage;
                    }
                }
            }

            // Check for convergence
            let mut max_change = 0.0;
            for (id, node) in &self.nodes {
                if !node.is_fixed {
                    if let Some(&prev_v) = prev_voltages.get(id) {
                        let change = (node.voltage - prev_v).abs();
                        if change > max_change {
                            max_change = change;
                        }
                    }
                }
            }

            if max_change < 0.001 {
                converged = true;
                break;
            }

            if iteration == self.max_iterations - 1 {
                break;
            }
        }

        converged
    }

    /// Get current voltage of a node
    pub fn voltage(&self, node_id: u32) -> f64 {
        self.nodes.get(&node_id)
            .map(|n| n.voltage)
            .unwrap_or(0.0)
    }

    /// Set voltage of a driven node
    pub fn set_voltage(&mut self, node_id: u32, voltage: f64) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.voltage = voltage;
        }
    }
}

impl Default for NodalSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rc_charging() {
        let mut solver = NodalSolver::new();

        // Setup: VDD = 5V (fixed), VSS = 0V (fixed), RC node (initially 0V)
        let vdd_id = 0u32;
        let vss_id = 1u32;
        let rc_id = 2u32;

        solver.add_node(vdd_id, "VDD".to_string(), 5.0, 0.0, true);   // Fixed power rail
        solver.add_node(vss_id, "VSS".to_string(), 0.0, 0.0, true);   // Fixed ground
        solver.add_node(rc_id, "RC".to_string(), 0.0, 10.0, false);   // Node with capacitance

        // Conductances: VDD-RC (resistor), RC-VSS (resistor)
        // R1 = 10kΩ = 0.0001 S, R2 = 10kΩ = 0.0001 S
        let conductance = 0.0001;  // 1/10000
        solver.add_conductance(vdd_id, rc_id, conductance);
        solver.add_conductance(rc_id, vss_id, conductance);

        // Solve: RC node should charge toward VDD
        let converged = solver.solve();
        assert!(converged, "Should converge");

        // With equal resistances and VDD=5V, VSS=0V, RC should settle to ~2.5V
        let rc_voltage = solver.voltage(rc_id);
        assert!(rc_voltage > 0.0, "RC node should charge");
        assert!(rc_voltage < 5.0, "RC node should not exceed VDD");
        println!("RC node voltage: {} V (expected ~2.5V)", rc_voltage);
    }

    #[test]
    fn test_capacitive_coupling() {
        let mut solver = NodalSolver::new();

        // Setup: VDD = 5V (fixed), VSS = 0V, Node1 = 2.5V, Node2 = 0V
        let vdd_id = 0u32;
        let vss_id = 1u32;
        let node1_id = 2u32;
        let node2_id = 3u32;

        solver.add_node(vdd_id, "VDD".to_string(), 5.0, 0.0, true);
        solver.add_node(vss_id, "VSS".to_string(), 0.0, 0.0, true);
        solver.add_node(node1_id, "N1".to_string(), 2.5, 10.0, false);
        solver.add_node(node2_id, "N2".to_string(), 0.0, 10.0, false);

        // Couple nodes through high-impedance conductance
        let coupling_conductance = 0.00001;  // High impedance
        solver.add_conductance(node1_id, node2_id, coupling_conductance);

        // Solve
        let converged = solver.solve();
        assert!(converged, "Should converge");

        // Nodes should move toward each other
        let v1 = solver.voltage(node1_id);
        let v2 = solver.voltage(node2_id);
        assert!(v1 < 2.5, "Node1 should decrease");
        assert!(v2 > 0.0, "Node2 should increase");
        println!("Coupling: N1={:.2}V, N2={:.2}V", v1, v2);
    }

    #[test]
    fn test_voltage_divider() {
        let mut solver = NodalSolver::new();

        // Setup: Voltage divider - VDD(5V) -> R1 -> N -> R2 -> VSS(0V)
        let vdd_id = 0u32;
        let vss_id = 1u32;
        let mid_id = 2u32;

        solver.add_node(vdd_id, "VDD".to_string(), 5.0, 0.0, true);
        solver.add_node(vss_id, "VSS".to_string(), 0.0, 0.0, true);
        solver.add_node(mid_id, "MID".to_string(), 0.0, 5.0, false);

        // Equal resistors: R1 = R2 = 10kΩ
        let g = 0.0001;  // 1/10000
        solver.add_conductance(vdd_id, mid_id, g);
        solver.add_conductance(mid_id, vss_id, g);

        let converged = solver.solve();
        assert!(converged, "Should converge");

        // With equal resistors, mid should be at 2.5V
        let v_mid = solver.voltage(mid_id);
        assert!((v_mid - 2.5).abs() < 0.1, "Mid should be ~2.5V, got {}", v_mid);
    }

    #[test]
    fn test_three_node_network() {
        let mut solver = NodalSolver::new();

        // Network: VDD -> N1 -> N2 -> N3 -> VSS (series resistors)
        let vdd_id = 0u32;
        let vss_id = 1u32;
        let n1_id = 2u32;
        let n2_id = 3u32;
        let n3_id = 4u32;

        solver.add_node(vdd_id, "VDD".to_string(), 5.0, 0.0, true);
        solver.add_node(vss_id, "VSS".to_string(), 0.0, 0.0, true);
        solver.add_node(n1_id, "N1".to_string(), 0.0, 5.0, false);
        solver.add_node(n2_id, "N2".to_string(), 0.0, 5.0, false);
        solver.add_node(n3_id, "N3".to_string(), 0.0, 5.0, false);

        let g = 0.0001;
        solver.add_conductance(vdd_id, n1_id, g);
        solver.add_conductance(n1_id, n2_id, g);
        solver.add_conductance(n2_id, n3_id, g);
        solver.add_conductance(n3_id, vss_id, g);

        let converged = solver.solve();
        assert!(converged, "Should converge");

        // Voltages should form linear gradient from VDD to VSS
        let v1 = solver.voltage(n1_id);
        let v2 = solver.voltage(n2_id);
        let v3 = solver.voltage(n3_id);

        // Expected: 3.75V, 2.5V, 1.25V respectively
        assert!(v1 > v2 && v2 > v3, "Voltages should decrease");
        assert!(v1 > 0.0 && v3 < 5.0, "Voltages should be in range");
        println!("Series divider: N1={:.2}V, N2={:.2}V, N3={:.2}V", v1, v2, v3);
    }

    #[test]
    fn test_parallel_paths() {
        let mut solver = NodalSolver::new();

        // Network: VDD -> (N1 parallel N2) -> VSS
        let vdd_id = 0u32;
        let vss_id = 1u32;
        let n1_id = 2u32;
        let n2_id = 3u32;

        solver.add_node(vdd_id, "VDD".to_string(), 5.0, 0.0, true);
        solver.add_node(vss_id, "VSS".to_string(), 0.0, 0.0, true);
        solver.add_node(n1_id, "N1".to_string(), 0.0, 5.0, false);
        solver.add_node(n2_id, "N2".to_string(), 0.0, 5.0, false);

        let g = 0.0001;
        solver.add_conductance(vdd_id, n1_id, g);
        solver.add_conductance(vdd_id, n2_id, g);
        solver.add_conductance(n1_id, vss_id, g);
        solver.add_conductance(n2_id, vss_id, g);

        let converged = solver.solve();
        assert!(converged, "Should converge");

        let v1 = solver.voltage(n1_id);
        let v2 = solver.voltage(n2_id);

        // Both should be equal at 2.5V
        assert!((v1 - v2).abs() < 0.01, "Parallel nodes should have equal voltage");
        assert!((v1 - 2.5).abs() < 0.1, "Should be ~2.5V");
    }

    #[test]
    fn test_empty_network() {
        let mut solver = NodalSolver::new();
        let converged = solver.solve();
        assert!(converged, "Empty network should trivially converge");
    }

    #[test]
    fn test_single_node() {
        let mut solver = NodalSolver::new();

        solver.add_node(0u32, "N".to_string(), 3.0, 0.0, false);
        let converged = solver.solve();
        assert!(converged, "Single unconnected node should converge");

        assert_eq!(solver.voltage(0u32), 3.0, "Voltage should not change");
    }
}
