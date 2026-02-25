//! DC operating point solver using Newton-Raphson iteration.
//!
//! Finds the steady-state voltages and currents in a circuit by
//! iteratively linearizing the nonlinear transistor models and solving
//! the resulting MNA system.
//!
//! Automatically selects between dense (nalgebra) and sparse (faer) backends
//! based on circuit size: circuits with fewer than [`SPARSE_THRESHOLD`] free
//! nodes use dense LU; larger circuits use sparse COO->CSC->LU.

use crate::circuit::graph::CircuitGraph;
use crate::circuit::TransistorKind;
use crate::device::pmos_level1::PmosLevel1;
use crate::device::depletion_load::DepletionLoadModel;
use crate::device::DeviceModel;
use crate::process::ProcessParams;

use super::convergence;
use super::matrix::MnaSystem;
use super::sparse_matrix::SparseMnaSystem;
use super::SolverConfig;

/// Free-node count threshold for switching from dense to sparse backend.
/// Circuits below this use dense nalgebra LU; circuits at or above use faer.
pub const SPARSE_THRESHOLD: usize = 100;

/// Solver backend selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SolverBackend {
    /// Dense nalgebra LU (optimal for small circuits).
    Dense,
    /// Sparse faer COO->CSC->LU (required for large circuits).
    Sparse,
    /// Automatically select based on circuit size vs [`SPARSE_THRESHOLD`].
    #[default]
    Auto,
}

/// Result of a DC operating point solve.
#[derive(Clone, Debug)]
pub struct DcOpResult {
    /// Node voltages (indexed by graph node index).
    pub voltages: Vec<f64>,
    /// Whether the solver converged.
    pub converged: bool,
    /// Number of Newton-Raphson iterations used.
    pub iterations: usize,
    /// Maximum voltage change in the final iteration.
    pub final_delta: f64,
}

/// Internal enum dispatch for dense/sparse MNA backends.
///
/// Avoids trait objects and keeps the Newton loop monomorphic per backend.
enum MnaBackend {
    Dense(MnaSystem),
    Sparse(SparseMnaSystem),
}

impl MnaBackend {
    fn node_to_row(&self) -> &[Option<usize>] {
        match self {
            Self::Dense(m) => &m.node_to_row,
            Self::Sparse(m) => &m.node_to_row,
        }
    }

    fn clear(&mut self) {
        match self {
            Self::Dense(m) => m.clear(),
            Self::Sparse(m) => m.clear(),
        }
    }

    fn add_gmin(&mut self, gmin: f64) {
        match self {
            Self::Dense(m) => m.add_gmin(gmin),
            Self::Sparse(m) => m.add_gmin(gmin),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn stamp_transistor(
        &mut self,
        gate: usize, source: usize, drain: usize,
        ids_op: f64, gm: f64, gds: f64,
        vgs_op: f64, vds_op: f64,
        fixed_voltages: &[Option<f64>],
    ) {
        match self {
            Self::Dense(m) => m.stamp_transistor(
                gate, source, drain, ids_op, gm, gds, vgs_op, vds_op, fixed_voltages,
            ),
            Self::Sparse(m) => m.stamp_transistor(
                gate, source, drain, ids_op, gm, gds, vgs_op, vds_op, fixed_voltages,
            ),
        }
    }

    fn solve(&mut self) -> Option<Vec<f64>> {
        match self {
            Self::Dense(m) => m.solve().map(|v| v.as_slice().to_vec()),
            Self::Sparse(m) => m.solve(),
        }
    }
}

/// DC operating point solver.
pub struct DcSolver {
    config: SolverConfig,
    process: ProcessParams,
}

impl DcSolver {
    pub fn new(config: SolverConfig, process: ProcessParams) -> Self {
        Self { config, process }
    }

    /// Determine which backend to use for the given number of free nodes.
    fn select_backend(&self, num_free: usize) -> SolverBackend {
        match self.config.backend {
            SolverBackend::Auto => {
                if num_free >= SPARSE_THRESHOLD {
                    SolverBackend::Sparse
                } else {
                    SolverBackend::Dense
                }
            }
            explicit => explicit,
        }
    }

    /// Solve the DC operating point of a circuit.
    ///
    /// Modifies node voltages in the circuit graph in-place and returns
    /// the result summary. Automatically selects dense or sparse backend
    /// based on circuit size and configuration.
    pub fn solve(&self, graph: &mut CircuitGraph) -> DcOpResult {
        let num_nodes = graph.nodes.len();

        // Build list of free node indices
        let free_indices: Vec<usize> = graph.nodes.iter()
            .enumerate()
            .filter(|(_, n)| !n.is_fixed)
            .map(|(i, _)| i)
            .collect();
        let num_free = free_indices.len();

        if num_free == 0 {
            return DcOpResult {
                voltages: graph.nodes.iter().map(|n| n.voltage).collect(),
                converged: true,
                iterations: 0,
                final_delta: 0.0,
            };
        }

        // Build fixed voltage lookup
        let fixed_voltages: Vec<Option<f64>> = graph.nodes.iter()
            .map(|n| if n.is_fixed { Some(n.voltage) } else { None })
            .collect();

        // Create device models for each transistor
        let models: Vec<Box<dyn DeviceModel>> = graph.transistors.iter()
            .map(|t| -> Box<dyn DeviceModel> {
                match t.kind {
                    TransistorKind::Enhancement => {
                        Box::new(PmosLevel1::new(&self.process, t.w, t.l))
                    }
                    TransistorKind::Depletion => {
                        Box::new(DepletionLoadModel::new(&self.process, t.w, t.l))
                    }
                }
            })
            .collect();

        // Current voltage vector for free nodes
        let mut v_free: Vec<f64> = free_indices.iter()
            .map(|&i| graph.nodes[i].voltage)
            .collect();
        let mut v_prev = v_free.clone();

        // Select backend based on circuit size
        let backend_choice = self.select_backend(num_free);
        let mut mna = match backend_choice {
            SolverBackend::Sparse => {
                MnaBackend::Sparse(SparseMnaSystem::new(num_free, num_nodes, &free_indices))
            }
            SolverBackend::Dense | SolverBackend::Auto => {
                MnaBackend::Dense(MnaSystem::new(num_free, num_nodes, &free_indices))
            }
        };

        let mut converged = false;
        let mut iterations = 0;
        let mut final_delta = f64::MAX;

        // Source stepping: always use at least a 2-step ramp for pMOS
        // circuits to avoid initial guess issues with large supply voltages.
        let supply_steps = if self.config.source_stepping {
            self.config.source_steps
        } else {
            1
        };

        for step in 0..supply_steps {
            let scale = convergence::source_step_factor(step, supply_steps);

            // Scale fixed voltages for source stepping
            let scaled_fixed: Vec<Option<f64>> = fixed_voltages.iter()
                .map(|v| v.map(|val| val * scale))
                .collect();

            // Newton-Raphson loop
            for iter in 0..self.config.max_nr_iterations {
                mna.clear();

                // Add gmin shunts for convergence
                mna.add_gmin(self.config.gmin);

                // Build full voltage vector for model evaluation
                let node_to_row = mna.node_to_row();
                let mut v_all: Vec<f64> = vec![0.0; num_nodes];
                for (i, node) in graph.nodes.iter().enumerate() {
                    if node.is_fixed {
                        v_all[i] = scaled_fixed[i].unwrap_or(0.0);
                    } else if let Some(row) = node_to_row[i] {
                        v_all[i] = v_free[row];
                    }
                }

                // Stamp each transistor using linearized companion model
                for (ti, trans) in graph.transistors.iter().enumerate() {
                    let vg = v_all[trans.gate];
                    let va = v_all[trans.a_node];
                    let vb = v_all[trans.b_node];

                    // For pMOS: source terminal is at higher potential.
                    let (vsg, vsd) = PmosLevel1::orient_voltages(vg, va, vb);
                    let (source_idx, drain_idx) = if va >= vb {
                        (trans.a_node, trans.b_node)
                    } else {
                        (trans.b_node, trans.a_node)
                    };

                    let vgs = -vsg;
                    let vds = -vsd;

                    let ids = models[ti].ids(vgs, vds);
                    let gm = models[ti].gm(vgs, vds);
                    let gds_val = models[ti].gds(vgs, vds);

                    let gds_eff = gds_val.max(self.config.gmin);

                    mna.stamp_transistor(
                        trans.gate,
                        source_idx,
                        drain_idx,
                        ids, gm, gds_eff,
                        vgs, vds,
                        &scaled_fixed,
                    );
                }

                // Solve the linear system
                let sol = match mna.solve() {
                    Some(s) => s,
                    None => {
                        // Singular matrix: try increasing gmin
                        mna.add_gmin(self.config.gmin * 1e6);
                        match mna.solve() {
                            Some(s) => s,
                            None => break, // Give up
                        }
                    }
                };

                // Apply voltage limiting and damping
                v_prev.copy_from_slice(&v_free);
                for (j, v) in v_free.iter_mut().enumerate() {
                    let v_new = sol[j];
                    let v_limited = convergence::voltage_limit(v_new, *v, 2.0);
                    *v = *v + self.config.damping * (v_limited - *v);
                }

                // Check convergence
                final_delta = convergence::max_delta(&v_free, &v_prev);
                iterations = iter + 1;

                if convergence::check_convergence(&v_free, &v_prev, self.config.v_tol) {
                    converged = true;
                    break;
                }
            }

            if !converged && step < supply_steps - 1 {
                converged = false;
            }
        }

        // Write final voltages back to graph
        let node_to_row = mna.node_to_row();
        let voltages: Vec<f64> = (0..num_nodes).map(|i| {
            if let Some(v) = fixed_voltages[i] {
                v
            } else if let Some(row) = node_to_row[i] {
                v_free[row]
            } else {
                0.0
            }
        }).collect();

        for (i, &v) in voltages.iter().enumerate() {
            graph.nodes[i].voltage = v;
        }

        DcOpResult {
            voltages,
            converged,
            iterations,
            final_delta,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::graph::{CircuitGraph, TransistorKind};

    fn make_inverter_graph(v_input: f64) -> CircuitGraph {
        let mut g = CircuitGraph::new();

        let vdd = g.add_node(-1);
        let vss = g.add_node(-2);
        let input = g.add_node(10);
        let output = g.add_node(11);

        g.set_power_rail(vdd, -15.0, "VDD");
        g.set_power_rail(vss, 0.0, "VSS");
        g.vdd_idx = Some(vdd);
        g.vss_idx = Some(vss);

        // Set input as fixed (driven externally)
        g.nodes[input].voltage = v_input;
        g.nodes[input].is_fixed = true;
        g.nodes[input].name = Some("IN".to_string());

        // Initialize output to midpoint
        g.nodes[output].voltage = -7.5;
        g.nodes[output].name = Some("OUT".to_string());

        // Depletion load: gate tied to output (gate=output, a=output, b=VDD)
        g.add_transistor(output, output, vdd, TransistorKind::Depletion, 10e-6, 20e-6);
        // Enhancement driver: gate=input, a=output, b=VSS
        g.add_transistor(input, output, vss, TransistorKind::Enhancement, 10e-6, 10e-6);

        g
    }

    #[test]
    fn inverter_input_low_output_near_vdd() {
        // Input at VSS (0V) -> gate not driven -> enhancement OFF
        // Output should be pulled toward VDD (-15V) by depletion load
        let mut graph = make_inverter_graph(0.0);
        let config = SolverConfig::small_circuit();
        let process = ProcessParams::default();
        let solver = DcSolver::new(config, process);

        let result = solver.solve(&mut graph);

        assert!(result.converged,
            "Should converge, iterations={}, delta={:.3e}",
            result.iterations, result.final_delta);

        let v_out = graph.nodes[3].voltage; // output node index
        // Output should be close to VDD (-15V)
        assert!(v_out < -10.0,
            "Output should be near VDD when input=0V, got {:.2}V", v_out);
    }

    #[test]
    fn inverter_input_high_output_near_vss() {
        // Input at VDD (-15V) -> enhancement ON, pulls output toward VSS
        let mut graph = make_inverter_graph(-15.0);
        let config = SolverConfig::small_circuit();
        let process = ProcessParams::default();
        let solver = DcSolver::new(config, process);

        let result = solver.solve(&mut graph);

        assert!(result.converged,
            "Should converge, iterations={}, delta={:.3e}",
            result.iterations, result.final_delta);

        let v_out = graph.nodes[3].voltage;
        // Output should be near VSS (0V) but not exactly due to voltage divider
        assert!(v_out > -5.0,
            "Output should be near VSS when input=-15V, got {:.2}V", v_out);
    }

    #[test]
    fn inverter_inverts() {
        let process = ProcessParams::default();
        let config = SolverConfig::small_circuit();

        let mut g_low = make_inverter_graph(0.0);
        let mut g_high = make_inverter_graph(-15.0);

        let solver = DcSolver::new(config.clone(), process.clone());
        solver.solve(&mut g_low);

        let solver = DcSolver::new(config, process);
        solver.solve(&mut g_high);

        let v_out_low = g_low.nodes[3].voltage;
        let v_out_high = g_high.nodes[3].voltage;

        // Output when input is low should be more negative than when input is high
        assert!(v_out_low < v_out_high,
            "Inverter should invert: V_out(in=0)={:.2}V, V_out(in=-15)={:.2}V",
            v_out_low, v_out_high);
    }

    #[test]
    fn empty_circuit_converges() {
        let mut graph = CircuitGraph::new();
        let config = SolverConfig::default();
        let process = ProcessParams::default();
        let solver = DcSolver::new(config, process);

        let result = solver.solve(&mut graph);
        assert!(result.converged);
        assert_eq!(result.iterations, 0);
    }

    #[test]
    fn nand2_gate() {
        // 2-input NAND: depletion load + 2 series enhancement drivers
        let mut g = CircuitGraph::new();

        let vdd = g.add_node(-1);
        let vss = g.add_node(-2);
        let in_a = g.add_node(10);
        let in_b = g.add_node(11);
        let output = g.add_node(12);
        let mid = g.add_node(13); // between two series transistors

        g.set_power_rail(vdd, -15.0, "VDD");
        g.set_power_rail(vss, 0.0, "VSS");
        g.vdd_idx = Some(vdd);
        g.vss_idx = Some(vss);

        // Both inputs driven to VDD (-15V) -> both enhancement ON
        g.nodes[in_a].voltage = -15.0;
        g.nodes[in_a].is_fixed = true;
        g.nodes[in_b].voltage = -15.0;
        g.nodes[in_b].is_fixed = true;

        g.nodes[output].voltage = -7.5;
        g.nodes[mid].voltage = -7.5;

        // Depletion load: output to VDD
        g.add_transistor(output, output, vdd, TransistorKind::Depletion, 10e-6, 20e-6);
        // Enhancement A: output to mid
        g.add_transistor(in_a, output, mid, TransistorKind::Enhancement, 10e-6, 10e-6);
        // Enhancement B: mid to VSS
        g.add_transistor(in_b, mid, vss, TransistorKind::Enhancement, 10e-6, 10e-6);

        let config = SolverConfig::small_circuit();
        let process = ProcessParams::default();
        let solver = DcSolver::new(config, process);
        let result = solver.solve(&mut g);

        assert!(result.converged,
            "NAND2 should converge, iter={}, delta={:.3e}",
            result.iterations, result.final_delta);

        let v_out = g.nodes[output.min(g.nodes.len() - 1)].voltage;
        // Both inputs high -> both drivers ON -> output should be near VSS
        assert!(v_out > -5.0,
            "NAND2 output with both inputs=-15V should be near VSS, got {:.2}V", v_out);
    }

    #[test]
    fn nor2_gate() {
        // 2-input NOR: depletion load + 2 parallel enhancement drivers
        let mut g = CircuitGraph::new();

        let vdd = g.add_node(-1);
        let vss = g.add_node(-2);
        let in_a = g.add_node(10);
        let in_b = g.add_node(11);
        let output = g.add_node(12);

        g.set_power_rail(vdd, -15.0, "VDD");
        g.set_power_rail(vss, 0.0, "VSS");
        g.vdd_idx = Some(vdd);
        g.vss_idx = Some(vss);

        // Both inputs at VSS (0V) -> both enhancement OFF
        g.nodes[in_a].voltage = 0.0;
        g.nodes[in_a].is_fixed = true;
        g.nodes[in_b].voltage = 0.0;
        g.nodes[in_b].is_fixed = true;

        g.nodes[output].voltage = -7.5;

        // Depletion load
        g.add_transistor(output, output, vdd, TransistorKind::Depletion, 10e-6, 20e-6);
        // Enhancement A: output to VSS (parallel)
        g.add_transistor(in_a, output, vss, TransistorKind::Enhancement, 10e-6, 10e-6);
        // Enhancement B: output to VSS (parallel)
        g.add_transistor(in_b, output, vss, TransistorKind::Enhancement, 10e-6, 10e-6);

        let config = SolverConfig::small_circuit();
        let process = ProcessParams::default();
        let solver = DcSolver::new(config, process);
        let result = solver.solve(&mut g);

        assert!(result.converged);

        let v_out = g.nodes[4].voltage; // output node
        // Both inputs low -> both drivers OFF -> output pulled to VDD
        assert!(v_out < -10.0,
            "NOR2 output with both inputs=0V should be near VDD, got {:.2}V", v_out);
    }

    #[test]
    fn dc_transfer_sweep() {
        // Sweep input voltage and verify monotonic output for inverter
        let process = ProcessParams::default();
        let config = SolverConfig::small_circuit();

        let mut prev_out = f64::NEG_INFINITY;
        let mut monotonic = true;
        let steps = 11;

        for i in 0..steps {
            let v_in = -15.0 * (i as f64) / ((steps - 1) as f64);
            let mut graph = make_inverter_graph(v_in);
            let solver = DcSolver::new(config.clone(), process.clone());
            let result = solver.solve(&mut graph);

            assert!(result.converged,
                "Should converge at Vin={:.1}V", v_in);

            let v_out = graph.nodes[3].voltage;

            if v_out < prev_out - 0.01 {
                monotonic = false;
            }
            prev_out = v_out;
        }

        assert!(monotonic,
            "Inverter DC transfer should be monotonically increasing (Vout vs Vin going more negative)");
    }

    #[test]
    fn robust_config_converges_on_inverter() {
        let mut graph = make_inverter_graph(-7.5);
        let config = SolverConfig::robust();
        let process = ProcessParams::default();
        let solver = DcSolver::new(config, process);
        let result = solver.solve(&mut graph);
        assert!(result.converged);
    }

    #[test]
    fn backend_auto_selects_dense_for_small() {
        let process = ProcessParams::default();
        let config = SolverConfig { backend: SolverBackend::Auto, ..SolverConfig::small_circuit() };
        let solver = DcSolver::new(config, process);

        // Inverter has 1 free node -- well below SPARSE_THRESHOLD
        assert_eq!(solver.select_backend(1), SolverBackend::Dense);
        assert_eq!(solver.select_backend(99), SolverBackend::Dense);
        assert_eq!(solver.select_backend(100), SolverBackend::Sparse);
        assert_eq!(solver.select_backend(1000), SolverBackend::Sparse);
    }

    #[test]
    fn force_sparse_on_small_circuit_matches_dense() {
        let process = ProcessParams::default();

        // Solve with dense (default)
        let mut graph_dense = make_inverter_graph(0.0);
        let config_dense = SolverConfig::small_circuit();
        let solver_dense = DcSolver::new(config_dense, process.clone());
        let result_dense = solver_dense.solve(&mut graph_dense);

        // Solve with forced sparse
        let mut graph_sparse = make_inverter_graph(0.0);
        let config_sparse = SolverConfig {
            backend: SolverBackend::Sparse,
            ..SolverConfig::small_circuit()
        };
        let solver_sparse = DcSolver::new(config_sparse, process);
        let result_sparse = solver_sparse.solve(&mut graph_sparse);

        assert!(result_dense.converged);
        assert!(result_sparse.converged);

        // Voltages should match within tolerance
        for (i, (&vd, &vs)) in result_dense.voltages.iter()
            .zip(result_sparse.voltages.iter())
            .enumerate()
        {
            assert!(
                (vd - vs).abs() < 1e-4,
                "Node {}: dense={:.6}V, sparse={:.6}V", i, vd, vs
            );
        }
    }

    #[test]
    fn force_sparse_nand2_matches_dense() {
        let process = ProcessParams::default();

        // Build NAND2 circuits for both backends
        let make_nand2 = || {
            let mut g = CircuitGraph::new();
            let vdd = g.add_node(-1);
            let vss = g.add_node(-2);
            let in_a = g.add_node(10);
            let in_b = g.add_node(11);
            let output = g.add_node(12);
            let mid = g.add_node(13);

            g.set_power_rail(vdd, -15.0, "VDD");
            g.set_power_rail(vss, 0.0, "VSS");
            g.vdd_idx = Some(vdd);
            g.vss_idx = Some(vss);

            g.nodes[in_a].voltage = -15.0;
            g.nodes[in_a].is_fixed = true;
            g.nodes[in_b].voltage = -15.0;
            g.nodes[in_b].is_fixed = true;
            g.nodes[output].voltage = -7.5;
            g.nodes[mid].voltage = -7.5;

            g.add_transistor(output, output, vdd, TransistorKind::Depletion, 10e-6, 20e-6);
            g.add_transistor(in_a, output, mid, TransistorKind::Enhancement, 10e-6, 10e-6);
            g.add_transistor(in_b, mid, vss, TransistorKind::Enhancement, 10e-6, 10e-6);
            g
        };

        let mut g_dense = make_nand2();
        let mut g_sparse = make_nand2();

        let solver_dense = DcSolver::new(SolverConfig::small_circuit(), process.clone());
        let solver_sparse = DcSolver::new(
            SolverConfig { backend: SolverBackend::Sparse, ..SolverConfig::small_circuit() },
            process,
        );

        let r_dense = solver_dense.solve(&mut g_dense);
        let r_sparse = solver_sparse.solve(&mut g_sparse);

        assert!(r_dense.converged);
        assert!(r_sparse.converged);

        for (i, (&vd, &vs)) in r_dense.voltages.iter()
            .zip(r_sparse.voltages.iter())
            .enumerate()
        {
            assert!(
                (vd - vs).abs() < 1e-4,
                "Node {}: dense={:.6}V, sparse={:.6}V", i, vd, vs
            );
        }
    }

    #[test]
    fn force_dense_explicit() {
        let mut graph = make_inverter_graph(0.0);
        let config = SolverConfig {
            backend: SolverBackend::Dense,
            ..SolverConfig::small_circuit()
        };
        let process = ProcessParams::default();
        let solver = DcSolver::new(config, process);
        let result = solver.solve(&mut graph);
        assert!(result.converged);
    }
}
