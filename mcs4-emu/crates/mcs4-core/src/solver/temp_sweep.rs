//! Temperature sweep analysis for process parameter sensitivity over the 4004 operating range.
//!
//! # Why
//!
//! The Intel 4004 is specified from 0 deg C to 70 deg C (273 K to 343 K). Over this range,
//! carrier mobility and threshold voltage shift significantly (mobility -30%, Vth ~140 mV),
//! which can affect logic margins. This module provides a sweep runner that executes a DC
//! operating-point solve at each temperature step and aggregates the results.
//!
//! # How
//!
//! 1. Create a [`TemperatureSweepConfig`] specifying the T range and step size.
//! 2. Call [`run_temperature_sweep`] with your circuit graph, base process params, and config.
//! 3. Inspect [`TempSweepResult`] for per-temperature voltages and the worst-case margin.

use crate::{
    circuit::CircuitGraph,
    process::ProcessParams,
    solver::{
        dc_op::{DcOpResult, DcSolver},
        SolverConfig,
    },
};

/// Configuration for a temperature sweep analysis.
///
/// Temperatures are in Kelvin. The sweep runs from `t_start` to `t_stop`
/// (inclusive), stepping by `t_step`.
#[derive(Clone, Debug)]
pub struct TemperatureSweepConfig {
    /// Start temperature (K). Default: 273.15 (0 deg C).
    pub t_start: f64,
    /// Stop temperature (K). Default: 343.15 (70 deg C).
    pub t_stop: f64,
    /// Step size (K). Default: 10 K.
    pub t_step: f64,
}

impl Default for TemperatureSweepConfig {
    /// Default sweep covers the 4004 operating range: 0 to 70 deg C, step 10 K.
    fn default() -> Self {
        Self {
            t_start: 273.15,
            t_stop: 343.15,
            t_step: 10.0,
        }
    }
}

/// One data point from a temperature sweep: temperature + DC operating point.
#[derive(Clone, Debug)]
pub struct TempSweepPoint {
    /// Temperature at which the solve was run (K).
    pub temp_k: f64,
    /// DC operating point result at this temperature.
    pub dc: DcOpResult,
}

/// Result of a complete temperature sweep.
#[derive(Clone, Debug)]
pub struct TempSweepResult {
    /// Per-temperature operating points, in order of increasing temperature.
    pub points: Vec<TempSweepPoint>,
    /// Index of the node with the largest voltage variation across the sweep,
    /// or `None` if fewer than 2 points or fewer than 2 nodes.
    pub max_variation_node: Option<usize>,
    /// Peak-to-peak voltage variation (V) at `max_variation_node`.
    pub max_variation_v: f64,
}

impl TempSweepResult {
    /// Number of temperature points collected.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// True if no temperature points were collected.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Voltages at node `node_idx` across all temperature points.
    ///
    /// Returns an empty vec if `node_idx` is out of range for any point.
    pub fn node_voltages(&self, node_idx: usize) -> Vec<f64> {
        self.points
            .iter()
            .filter_map(|p| p.dc.voltages.get(node_idx).copied())
            .collect()
    }
}

/// Run a DC operating-point solve at each temperature in the sweep range.
///
/// For each temperature step:
/// 1. Create a temperature-scaled copy of `base_params` via [`ProcessParams::at_temperature`].
/// 2. Run a DC solve using the default [`SolverConfig`].
/// 3. Record the result in [`TempSweepResult`].
///
/// After the sweep, identifies the circuit node with the largest peak-to-peak
/// voltage variation across temperature.
///
/// # Arguments
///
/// * `graph` - Circuit graph to solve (borrowed mutably; voltages are updated in place
///   for each solve but restored between steps).
/// * `base_params` - Process parameters at the nominal temperature (300 K).
/// * `config` - Sweep configuration (temperature range and step).
pub fn run_temperature_sweep(
    graph: &mut CircuitGraph,
    base_params: &ProcessParams,
    config: &TemperatureSweepConfig,
) -> TempSweepResult {
    let mut points: Vec<TempSweepPoint> = Vec::new();

    // Save initial node voltages to reset between solves (each solve mutates graph).
    let initial_voltages: Vec<f64> = graph.nodes.iter().map(|n| n.voltage).collect();

    let mut t = config.t_start;
    while t <= config.t_stop + 1e-9 {
        // Reset node voltages to initial state before each solve.
        for (node, &v) in graph.nodes.iter_mut().zip(initial_voltages.iter()) {
            node.voltage = v;
        }

        let params_t = base_params.at_temperature(t);
        let solver_config = SolverConfig::default();
        let dc_result = DcSolver::new(solver_config, params_t).solve(graph);

        points.push(TempSweepPoint {
            temp_k: t,
            dc: dc_result,
        });

        t += config.t_step;
    }

    // Find node with maximum peak-to-peak variation.
    let (max_variation_node, max_variation_v) = compute_max_variation(&points);

    TempSweepResult {
        points,
        max_variation_node,
        max_variation_v,
    }
}

fn compute_max_variation(points: &[TempSweepPoint]) -> (Option<usize>, f64) {
    if points.len() < 2 {
        return (None, 0.0);
    }
    let num_nodes = points[0].dc.voltages.len();
    if num_nodes == 0 {
        return (None, 0.0);
    }

    let mut best_node = None;
    let mut best_variation = 0.0_f64;

    for node_idx in 0..num_nodes {
        let voltages: Vec<f64> = points
            .iter()
            .filter_map(|p| p.dc.voltages.get(node_idx).copied())
            .collect();
        if voltages.is_empty() {
            continue;
        }
        let v_min = voltages.iter().cloned().fold(f64::INFINITY, f64::min);
        let v_max = voltages.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let variation = v_max - v_min;
        if variation > best_variation {
            best_variation = variation;
            best_node = Some(node_idx);
        }
    }

    (best_node, best_variation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::CircuitGraph;

    // Build a minimal graph: two fixed-voltage nodes (no free nodes), so DC solve
    // returns immediately without failing. Used to validate sweep mechanics.
    fn fixed_graph() -> CircuitGraph {
        let mut g = CircuitGraph::new();
        let vss = g.add_node(0);
        let vdd = g.add_node(1);
        g.set_power_rail(vss, 0.0, "VSS");
        g.set_power_rail(vdd, -15.0, "VDD");
        g
    }

    #[test]
    fn mu0_temperature_scaling() {
        // WHY: verify (300/T)^1.5 power law for mobility reduction with temperature.
        let base = ProcessParams::default();
        let hot = base.at_temperature(350.0);
        let cold = base.at_temperature(250.0);

        // mu_0 should decrease as T increases (lattice scattering)
        assert!(
            cold.mu_0 > base.mu_0,
            "Cold mu_0 ({:.2}) should exceed nominal ({:.2})",
            cold.mu_0,
            base.mu_0
        );
        assert!(
            hot.mu_0 < base.mu_0,
            "Hot mu_0 ({:.2}) should be below nominal ({:.2})",
            hot.mu_0,
            base.mu_0
        );

        // At 350K vs 250K the ratio is (300/350)^1.5 / (300/250)^1.5
        let expected_hot = base.mu_0 * (300.0_f64 / 350.0).powf(1.5);
        let expected_cold = base.mu_0 * (300.0_f64 / 250.0).powf(1.5);
        assert!(
            (hot.mu_0 - expected_hot).abs() < 0.01,
            "mu_0 at 350K: got {:.4}, expected {:.4}",
            hot.mu_0,
            expected_hot
        );
        assert!(
            (cold.mu_0 - expected_cold).abs() < 0.01,
            "mu_0 at 250K: got {:.4}, expected {:.4}",
            cold.mu_0,
            expected_cold
        );
    }

    #[test]
    fn vth_temperature_shift() {
        // WHY: verify ~-2 mV/K Vth shift (pMOS enhancement: magnitude decreases with T).
        let base = ProcessParams::default();
        let hot = base.at_temperature(350.0); // +50 K from 300 K
        let cold = base.at_temperature(250.0); // -50 K from 300 K

        // Vth_enhancement is negative; at higher T the magnitude should decrease
        // (less negative), meaning vth_enhancement_hot > vth_enhancement_base
        assert!(
            hot.vth_enhancement > base.vth_enhancement,
            "Vth at 350K ({:.4}) should be less negative than at 300K ({:.4})",
            hot.vth_enhancement,
            base.vth_enhancement
        );
        assert!(
            cold.vth_enhancement < base.vth_enhancement,
            "Vth at 250K ({:.4}) should be more negative than at 300K ({:.4})",
            cold.vth_enhancement,
            base.vth_enhancement
        );

        // Shift magnitude: +2 mV/K * 50 K = 100 mV = 0.1 V
        // At 350K (hot): Vth shifts +0.1 V (less negative)
        // At 250K (cold): Vth shifts -0.1 V (more negative)
        let expected_hot_shift = 0.002 * 50.0; // +0.1 V
        let expected_cold_shift = -0.002 * 50.0; // -0.1 V
        let hot_shift = hot.vth_enhancement - base.vth_enhancement;
        let cold_shift = cold.vth_enhancement - base.vth_enhancement;
        assert!(
            (hot_shift - expected_hot_shift).abs() < 1e-6,
            "Hot Vth shift: got {:.6}, expected {:.6}",
            hot_shift,
            expected_hot_shift
        );
        assert!(
            (cold_shift - expected_cold_shift).abs() < 1e-6,
            "Cold Vth shift: got {:.6}, expected {:.6}",
            cold_shift,
            expected_cold_shift
        );
    }

    #[test]
    fn at_temperature_preserves_other_params() {
        // WHY: non-temperature params (geometry, oxide, supply) must not change.
        let base = ProcessParams::default();
        let scaled = base.at_temperature(350.0);

        assert!((scaled.vss - base.vss).abs() < 1e-15, "vss changed");
        assert!((scaled.vdd - base.vdd).abs() < 1e-15, "vdd changed");
        assert!((scaled.t_ox - base.t_ox).abs() < 1e-20, "t_ox changed");
        assert!((scaled.theta - base.theta).abs() < 1e-15, "theta changed");
        assert!((scaled.n_sub - base.n_sub).abs() < 1.0, "n_sub changed");
        assert!((scaled.n_sd - base.n_sd).abs() < 1.0, "n_sd changed");
        assert!((scaled.l_min - base.l_min).abs() < 1e-15, "l_min changed");
        assert!((scaled.x_j - base.x_j).abs() < 1e-15, "x_j changed");
        assert!((scaled.lambda - base.lambda).abs() < 1e-15, "lambda changed");
        assert!((scaled.eta_dibl - base.eta_dibl).abs() < 1e-15, "eta_dibl changed");
        assert!((scaled.temp - 350.0).abs() < 1e-10, "temp not updated");
    }

    #[test]
    fn sweep_default_range() {
        // WHY: verify the sweep runs without panicking over the 4004 operating range.
        let base = ProcessParams::default();
        let config = TemperatureSweepConfig::default();
        let mut graph = fixed_graph();

        let result = run_temperature_sweep(&mut graph, &base, &config);

        // 273.15 to 343.15 in steps of 10 K -> 8 steps (273, 283, 293, 303, 313, 323, 333, 343)
        assert!(result.len() >= 7, "Expected at least 7 points, got {}", result.len());
        assert!(!result.is_empty());

        // All temperatures should be within the configured range (with floating point slack)
        for pt in &result.points {
            assert!(pt.temp_k >= config.t_start - 1e-9);
            assert!(pt.temp_k <= config.t_stop + 1.0);
        }
    }

    #[test]
    fn sweep_inverter_margin() {
        // WHY: verify that for a fixed-node graph (trivial inverter represented as two
        // fixed rails), the sweep runs and records stable voltages across temperature.
        // A true inverter would require transistors; this validates sweep mechanics.
        let base = ProcessParams::default();
        let config = TemperatureSweepConfig {
            t_start: 273.15,
            t_stop: 343.15,
            t_step: 20.0, // coarser step for speed
        };
        let mut graph = fixed_graph();

        let result = run_temperature_sweep(&mut graph, &base, &config);

        // Fixed nodes: voltages should remain constant across temperature
        let vss_voltages = result.node_voltages(0);
        let vdd_voltages = result.node_voltages(1);
        assert!(!vss_voltages.is_empty());
        assert!(!vdd_voltages.is_empty());

        for v in &vss_voltages {
            assert!(v.abs() < 1e-9, "VSS node should stay at 0V, got {}", v);
        }
        for v in &vdd_voltages {
            assert!((*v - (-15.0)).abs() < 1e-9, "VDD node should stay at -15V, got {}", v);
        }
    }
}
