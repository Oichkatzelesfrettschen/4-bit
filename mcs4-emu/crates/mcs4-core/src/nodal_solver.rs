//! Nodal analysis solver for transistor-level simulation
//!
//! Implements Modified Nodal Analysis (MNA) using sparse-friendly matrix solving.
//! Handles resistors, capacitors, voltage sources, and current sources.
//! Provides DC operating point and transient simulation capabilities.

use std::collections::HashMap;

use faer::{prelude::*, Mat};

use crate::{
    circuit::graph::{CircuitGraph, TransistorKind},
    tcad::TCADBridge,
};

/// Types of circuit elements supported by the solver
#[derive(Clone, Debug)]
pub enum Element {
    /// Resistor with conductance G = 1/R
    Resistor { from: u32, to: u32, g: f64 },
    /// Capacitor with capacitance C
    Capacitor { from: u32, to: u32, c: f64 },
    /// Independent voltage source V
    VoltageSource { from: u32, to: u32, v: f64 },
    /// Independent current source I
    CurrentSource { from: u32, to: u32, i: f64 },
    /// TCAD-based pMOS transistor
    TCADTransistor {
        g: u32,
        s: u32,
        d: u32,
        b: u32,
        w: f64,
        l: f64,
        kind: TransistorKind,
    },
}

/// Nodal analysis solver using faer for matrix operations
pub struct NodalSolver {
    /// Mapping of node IDs to internal matrix indices
    node_to_idx: HashMap<u32, usize>,
    /// Number of nodes (excluding ground)
    num_nodes: usize,
    /// Circuit elements
    elements: Vec<Element>,
    /// Number of voltage sources (adds rows/cols to MNA matrix)
    num_vsources: usize,
    /// Simulation time step
    dt: f64,
    /// Previous node voltages (for transient analysis)
    prev_voltages: Vec<f64>,
    /// Current node voltages
    voltages: Vec<f64>,
    /// Ground node ID (default 0)
    gnd_id: u32,
    /// TCAD Bridge for physics-based simulation
    bridge: Option<TCADBridge>,
    /// Newton-Raphson convergence tolerance
    nr_tol: f64,
    /// Maximum Newton-Raphson iterations
    nr_max_iter: usize,
}

impl NodalSolver {
    pub fn new() -> Self {
        Self {
            node_to_idx: HashMap::new(),
            num_nodes: 0,
            elements: Vec::new(),
            num_vsources: 0,
            dt: 1e-6, // 1us default
            prev_voltages: Vec::new(),
            voltages: Vec::new(),
            gnd_id: 0,
            bridge: None,
            nr_tol: 1e-3,
            nr_max_iter: 500,
        }
    }

    /// Load circuit from a CircuitGraph
    pub fn load_circuit_graph(&mut self, graph: &CircuitGraph) {
        // Clear existing state
        self.node_to_idx.clear();
        self.num_nodes = 0;
        self.elements.clear();
        self.num_vsources = 0;

        // Determine ground node from graph if available, else default 0
        if let Some(vss_idx) = graph.vss_idx {
            self.set_gnd(vss_idx as u32);
        }

        // Add nodes
        for (i, node) in graph.nodes.iter().enumerate() {
            let id = i as u32;
            self.add_node(
                id,
                node.name.clone().unwrap_or_default(),
                node.voltage,
                node.capacitance,
                node.is_fixed,
            );

            // Fixed voltage sources (relative to ground)
            if node.is_fixed && id != self.gnd_id {
                self.add_voltage_source(id, self.gnd_id, node.voltage);
            }

            // Lumped capacitance to ground
            if node.capacitance > 0.0 && id != self.gnd_id {
                self.add_capacitor(id, self.gnd_id, node.capacitance);
            }
        }

        // Add transistors
        for t in &graph.transistors {
            self.add_tcad_transistor(
                t.gate as u32,
                t.a_node as u32,
                t.b_node as u32,
                t.bulk.unwrap_or(t.a_node) as u32, // Default bulk to source if unknown
                t.w,
                t.l,
                t.kind,
            );
        }
    }

    /// Set the TCAD bridge for physics-based simulation
    pub fn set_bridge(&mut self, bridge: TCADBridge) {
        self.bridge = Some(bridge);
    }

    /// Set the simulation time step
    pub fn set_dt(&mut self, dt: f64) {
        self.dt = dt;
    }

    /// Set the ground node ID
    pub fn set_gnd(&mut self, id: u32) {
        self.gnd_id = id;
    }

    /// Get the ground node ID
    pub fn get_gnd(&self) -> u32 {
        self.gnd_id
    }

    /// Add a node to the network (explicit registration)
    pub fn add_node(&mut self, id: u32, _name: String, _v_init: f64, _cap: f64, _is_fixed: bool) {
        if id != self.gnd_id && !self.node_to_idx.contains_key(&id) {
            self.node_to_idx.insert(id, self.num_nodes);
            self.num_nodes += 1;
        }
    }

    /// Add a resistor between two nodes
    pub fn add_resistor(&mut self, n1: u32, n2: u32, r: f64) {
        let g = 1.0 / r;
        self.add_node_if_missing(n1);
        self.add_node_if_missing(n2);
        self.elements.push(Element::Resistor { from: n1, to: n2, g });
    }

    /// Legacy support: add conductance
    pub fn add_conductance(&mut self, from: u32, to: u32, g: f64) {
        self.add_node_if_missing(from);
        self.add_node_if_missing(to);
        self.elements.push(Element::Resistor { from, to, g });
    }

    /// Add a capacitor between two nodes
    pub fn add_capacitor(&mut self, n1: u32, n2: u32, c: f64) {
        self.add_node_if_missing(n1);
        self.add_node_if_missing(n2);
        self.elements.push(Element::Capacitor { from: n1, to: n2, c });
    }

    /// Add a voltage source
    pub fn add_voltage_source(&mut self, n_pos: u32, n_neg: u32, v: f64) {
        if n_pos == n_neg {
            return;
        }
        self.add_node_if_missing(n_pos);
        self.add_node_if_missing(n_neg);
        self.elements.push(Element::VoltageSource {
            from: n_pos,
            to: n_neg,
            v,
        });
        self.num_vsources += 1;
    }

    /// Add a current source
    pub fn add_current_source(&mut self, n_pos: u32, n_neg: u32, i: f64) {
        self.add_node_if_missing(n_pos);
        self.add_node_if_missing(n_neg);
        self.elements.push(Element::CurrentSource {
            from: n_pos,
            to: n_neg,
            i,
        });
    }

    /// Add a TCAD-based transistor
    pub fn add_tcad_transistor(&mut self, g: u32, s: u32, d: u32, b: u32, w: f64, l: f64, kind: TransistorKind) {
        self.add_node_if_missing(g);
        self.add_node_if_missing(s);
        self.add_node_if_missing(d);
        self.add_node_if_missing(b);
        self.elements.push(Element::TCADTransistor { g, s, d, b, w, l, kind });
    }

    fn add_node_if_missing(&mut self, id: u32) {
        if id != self.gnd_id && !self.node_to_idx.contains_key(&id) {
            self.node_to_idx.insert(id, self.num_nodes);
            self.num_nodes += 1;
        }
    }

    /// Solve the DC operating point or one transient step.
    ///
    /// For systems with non-linear elements (TCAD transistors), this performs
    /// Newton-Raphson iterations.
    /// Solve the DC operating point or one transient step.
    pub fn solve(&mut self) -> bool {
        // Ensure vectors are correctly sized
        if self.voltages.len() != self.num_nodes {
            self.voltages = vec![0.0; self.num_nodes];
        }

        // Capture current voltages as previous for this step
        self.prev_voltages = self.voltages.clone();

        let dim = self.num_nodes + self.num_vsources;
        if dim == 0 {
            return true;
        }

        let mut converged = false;
        let has_nonlinear = self
            .elements
            .iter()
            .any(|e| matches!(e, Element::TCADTransistor { .. }));
        let mut damp = if has_nonlinear { 0.5 } else { 1.0 };
        let mut last_max_delta = f64::MAX;

        for _iter in 0..self.nr_max_iter {
            let mut a = Mat::<f64>::zeros(dim, dim);
            let mut b = Mat::<f64>::zeros(dim, 1);
            let mut vsource_idx = 0;

            // Gmin stabilization: add a small conductance to ground on every node.
            // WHY: Prevents singular matrices when nodes are floating (no DC path
            // to ground). 1e-9 S = 1 GOhm shunt, chosen as a trade-off:
            //   too small (< 1e-12) -> near-singular matrix, poor convergence
            //   too large (> 1e-6)  -> shifts resistive dividers by > 1mV
            // At 1e-9 S the voltage shift on a typical voltage divider is ~1e-5 V;
            // tests use 1e-4 V tolerance to account for this systematic offset.
            let gmin = 1e-9;
            for i in 0..self.num_nodes {
                a[(i, i)] += gmin;
            }

            for el in &self.elements {
                match *el {
                    Element::Resistor { from, to, g } => {
                        self.stamp_conductance(&mut a, from, to, g);
                    }
                    Element::Capacitor { from, to, c } => {
                        let g_cap = c / self.dt;
                        self.stamp_conductance(&mut a, from, to, g_cap);
                        let v_prev = self.get_prev_v_diff(from, to);
                        self.stamp_current_source(&mut b, to, from, g_cap * v_prev);
                    }
                    Element::VoltageSource { from, to, v } => {
                        let idx = self.num_nodes + vsource_idx;
                        self.stamp_voltage_source(&mut a, &mut b, from, to, v, idx);
                        vsource_idx += 1;
                    }
                    Element::CurrentSource { from, to, i } => {
                        self.stamp_current_source(&mut b, from, to, i);
                    }
                    Element::TCADTransistor {
                        g,
                        s,
                        d,
                        b: body,
                        w,
                        l: _,
                        kind,
                    } => {
                        if let Some(ref bridge) = self.bridge {
                            let v_g = self.voltage(g);
                            let v_s = self.voltage(s);
                            let v_d = self.voltage(d);
                            let v_b = self.voltage(body);
                            let res = bridge.solve_device(v_g, v_s, v_d, v_b, w, kind);
                            let i_rhs = res.ids - res.gm * v_g - res.gds * v_d;
                            self.stamp_transconductance(&mut a, d, s, g, res.gm);
                            self.stamp_conductance(&mut a, d, s, res.gds);
                            self.stamp_current_source(&mut b, s, d, i_rhs);
                            let g_cap = res.cg / self.dt;
                            self.stamp_conductance(&mut a, g, s, g_cap);
                            let v_gs_prev = self.get_prev_v_diff(g, s);
                            self.stamp_current_source(&mut b, s, g, g_cap * v_gs_prev);
                        }
                    }
                }
            }

            let lu = a.partial_piv_lu();
            let x = lu.solve(&b);

            // Check for NaN results (divergence)
            for i in 0..dim {
                if x[(i, 0)].is_nan() {
                    return false;
                }
            }
            let mut max_delta_raw: f64 = 0.0;
            for i in 0..self.num_nodes {
                let v_new_raw = x[(i, 0)];
                let v_old = self.voltages[i];
                let delta_raw = v_new_raw - v_old;
                max_delta_raw = max_delta_raw.max(delta_raw.abs());

                // Voltage-limiting for nonlinear convergence stability
                let limit = if has_nonlinear { 1.0 } else { f64::MAX };
                let delta_limited = if delta_raw.abs() > limit {
                    delta_raw.signum() * limit
                } else {
                    delta_raw
                };

                let delta = delta_limited * damp;
                self.voltages[i] = v_old + delta;
            }

            if max_delta_raw < self.nr_tol {
                converged = true;
                break;
            }

            // Update damping factor
            if max_delta_raw > last_max_delta {
                damp *= 0.5; // Back off
            } else {
                damp = (damp * 1.1).min(1.0); // Accelerate
            }
            last_max_delta = max_delta_raw;
        }

        converged
    }

    /// Solve using source stepping for better convergence on complex non-linear circuits.
    pub fn solve_robust(&mut self) -> bool {
        let original_elements = self.elements.clone();
        let steps = 50;

        for step in 1..=steps {
            let scale = step as f64 / steps as f64;

            // Temporarily scale all voltage sources
            for el in &mut self.elements {
                if let Element::VoltageSource { ref mut v, .. } = *el {
                    *v *= scale;
                }
            }

            if !self.solve() {
                self.elements = original_elements;
                return false;
            }

            // Restore original values for next step scaling
            self.elements = original_elements.clone();
        }

        true
    }

    fn stamp_conductance(&self, a: &mut Mat<f64>, n1: u32, n2: u32, g: f64) {
        let i1 = self.node_to_idx.get(&n1).copied();
        let i2 = self.node_to_idx.get(&n2).copied();

        if let Some(idx1) = i1 {
            a[(idx1, idx1)] += g;
        }
        if let Some(idx2) = i2 {
            a[(idx2, idx2)] += g;
        }
        if let (Some(idx1), Some(idx2)) = (i1, i2) {
            a[(idx1, idx2)] -= g;
            a[(idx2, idx1)] -= g;
        }
    }

    fn stamp_transconductance(&self, a: &mut Mat<f64>, n_d: u32, n_s: u32, n_g: u32, gm: f64) {
        let i_d = self.node_to_idx.get(&n_d).copied();
        let i_s = self.node_to_idx.get(&n_s).copied();
        let i_g = self.node_to_idx.get(&n_g).copied();

        if let (Some(idx_d), Some(idx_g)) = (i_d, i_g) {
            a[(idx_d, idx_g)] += gm;
        }
        if let (Some(idx_s), Some(idx_g)) = (i_s, i_g) {
            a[(idx_s, idx_g)] -= gm;
        }
    }

    fn stamp_current_source(&self, b: &mut Mat<f64>, n_pos: u32, n_neg: u32, i: f64) {
        if let Some(&idx) = self.node_to_idx.get(&n_pos) {
            b[(idx, 0)] -= i;
        }
        if let Some(&idx) = self.node_to_idx.get(&n_neg) {
            b[(idx, 0)] += i;
        }
    }

    fn stamp_voltage_source(&self, a: &mut Mat<f64>, b: &mut Mat<f64>, n_pos: u32, n_neg: u32, v: f64, vs_idx: usize) {
        let i_pos = self.node_to_idx.get(&n_pos).copied();
        let i_neg = self.node_to_idx.get(&n_neg).copied();

        if let Some(idx) = i_pos {
            a[(vs_idx, idx)] = 1.0;
            a[(idx, vs_idx)] = 1.0;
        }
        if let Some(idx) = i_neg {
            a[(vs_idx, idx)] = -1.0;
            a[(idx, vs_idx)] = -1.0;
        }
        b[(vs_idx, 0)] = v;
    }

    fn get_prev_v_diff(&self, n1: u32, n2: u32) -> f64 {
        if self.prev_voltages.is_empty() {
            return 0.0;
        }
        let v1 = self
            .node_to_idx
            .get(&n1)
            .map(|&idx| self.prev_voltages[idx])
            .unwrap_or(0.0);
        let v2 = self
            .node_to_idx
            .get(&n2)
            .map(|&idx| self.prev_voltages[idx])
            .unwrap_or(0.0);
        v1 - v2
    }

    /// Get current voltage of a node
    pub fn voltage(&self, node_id: u32) -> f64 {
        if node_id == self.gnd_id {
            return 0.0;
        }
        self.node_to_idx
            .get(&node_id)
            .map(|&idx| self.voltages[idx])
            .unwrap_or(0.0)
    }

    /// Set voltage of a driven node (for initialization)
    pub fn set_voltage(&mut self, node_id: u32, voltage: f64) {
        if let Some(&idx) = self.node_to_idx.get(&node_id) {
            if self.voltages.is_empty() {
                self.voltages = vec![0.0; self.num_nodes];
            }
            self.voltages[idx] = voltage;
        }
    }

    /// Update the value of an existing voltage source
    pub fn update_voltage_source(&mut self, n_pos: u32, n_neg: u32, new_v: f64) {
        for el in &mut self.elements {
            if let Element::VoltageSource { from, to, ref mut v } = *el {
                if from == n_pos && to == n_neg {
                    *v = new_v;
                    return;
                }
            }
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
    fn test_voltage_divider() {
        let mut solver = NodalSolver::new();
        solver.set_gnd(0);

        // VDD(1) -> R1(10k) -> MID(2) -> R2(10k) -> GND(0)
        // Voltage Source 5V between 1 and 0
        solver.add_voltage_source(1, 0, 5.0);
        solver.add_resistor(1, 2, 10000.0);
        solver.add_resistor(2, 0, 10000.0);

        assert!(solver.solve());

        assert!((solver.voltage(1) - 5.0).abs() < 1e-6);
        // Tolerance 1e-4: gmin stabilization (1e-9 S) introduces small leakage
        // that shifts the divider output by ~G_min / (2*G + G_min) * V
        assert!((solver.voltage(2) - 2.5).abs() < 1e-4);
        assert_eq!(solver.voltage(0), 0.0);
    }

    #[test]
    fn test_rc_transient() {
        let mut solver = NodalSolver::new();
        solver.set_gnd(0);
        solver.set_dt(1e-4); // 0.1ms

        // VDD(1) -> R(1k) -> CAP(2) -> GND(0)
        solver.add_voltage_source(1, 0, 5.0);
        solver.add_resistor(1, 2, 1000.0);
        solver.add_capacitor(2, 0, 1e-6); // 1uF

        // RC time constant = 1k * 1uF = 1ms

        // Initial solve (t=0)
        solver.solve();
        let v_0 = solver.voltage(2);

        // Run 10 steps (1ms total)
        for _ in 0..9 {
            solver.solve();
        }

        let v_1ms = solver.voltage(2);
        assert!(v_1ms > v_0);

        // After 1ms (1 time constant), V should be ~63% of 5V = 3.16V
        let expected = 5.0 * (1.0 - (-1.0f64).exp());
        assert!(
            (v_1ms - expected).abs() < 0.1,
            "Expected ~{:.2}V, got {:.2}V",
            expected,
            v_1ms
        );
    }

    #[test]
    fn test_tcad_inverter() {
        use crate::process::ProcessParams;
        let params = ProcessParams::default();
        let bridge = TCADBridge::new(&params);

        let mut solver = NodalSolver::new();
        solver.set_bridge(bridge);
        solver.set_gnd(0); // VSS/Body

        // VDD = -15V (Node 1)
        solver.add_voltage_source(1, 0, -15.0);

        // Input = -15V (Node 2)
        solver.add_voltage_source(2, 0, -15.0);

        // pMOS Transistor: G=2, S=0, D=3, B=0
        // W=10um, L=10um
        solver.add_tcad_transistor(2, 0, 3, 0, 10e-6, 10e-6, TransistorKind::Enhancement);

        // Load resistor to VDD
        solver.add_resistor(3, 1, 100000.0); // 100k

        // Solve DC OP
        assert!(solver.solve());

        // With Vg = -15V (Logic Low), pMOS is ON.
        // Output (Node 3) should be near Source (0V).
        let v_out_on = solver.voltage(3);
        assert!(v_out_on > -5.0, "Output should be near 0V, got {}", v_out_on);

        // Change input to 0V (Logic High)
        let mut solver_off = NodalSolver::new();
        solver_off.set_bridge(TCADBridge::new(&params));
        solver_off.set_gnd(0);
        solver_off.add_voltage_source(1, 0, -15.0);
        solver_off.add_voltage_source(2, 0, 0.0); // Input at 0V
        solver_off.add_tcad_transistor(2, 0, 3, 0, 10e-6, 10e-6, TransistorKind::Enhancement);
        solver_off.add_resistor(3, 1, 100000.0);

        assert!(solver_off.solve());

        // With Vg = 0V (Logic High), pMOS is OFF.
        // Output (Node 3) should be pulled to VDD (-15V).
        let v_out_off = solver_off.voltage(3);
        assert!(v_out_off < -14.0, "Output should be near -15V, got {}", v_out_off);
    }
}
