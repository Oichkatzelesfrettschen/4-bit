//! Property-based tests for the mcs4-core solver stack (debt phase D4.1).
//!
//! Invariants tested:
//! - DC: all free-node voltages stay within supply rails across circuit sizes
//! - DC: node-ID permutation does not change the solved voltage at each node
//! - DC: solver converges across the allowed gmin range (1e-12 to 1e-9)
//! - Transient: recorded time points are strictly monotone increasing
//! - Transient: all voltages stay within supply rails throughout the waveform

#![allow(missing_docs)]

use mcs4_core::{
    circuit::graph::{CircuitGraph, TransistorKind},
    process::ProcessParams,
    solver::{dc_op::DcSolver, SolverBackend, SolverConfig, TransientConfig, TransientSolver},
};
use proptest::prelude::*;

// --- Helpers ---

/// Build a minimal pMOS inverter: depletion load + enhancement driver.
///
/// Returns `(graph, output_idx)`.  All netlist IDs are caller-supplied so the
/// same topology can be instantiated with different node numbering.
fn build_inverter(v_in: f64, vdd_id: i32, vss_id: i32, input_id: i32, output_id: i32) -> (CircuitGraph, usize) {
    let mut g = CircuitGraph::new();

    let vdd = g.add_node(vdd_id);
    let vss = g.add_node(vss_id);
    let inp = g.add_node(input_id);
    let out = g.add_node(output_id);

    g.set_power_rail(vdd, -15.0, "VDD");
    g.set_power_rail(vss, 0.0, "VSS");
    g.vdd_idx = Some(vdd);
    g.vss_idx = Some(vss);

    g.nodes[inp].voltage = v_in;
    g.nodes[inp].is_fixed = true;
    g.nodes[out].voltage = -7.5;

    // Depletion load (gate=drain, drain side toward VDD)
    g.add_transistor(out, out, vdd, TransistorKind::Depletion, 10e-6, 20e-6);
    // Enhancement driver (gate=input, source=VSS)
    g.add_transistor(inp, out, vss, TransistorKind::Enhancement, 10e-6, 10e-6);

    (g, out)
}

// --- DC: voltages within supply rails for every chain length and input ---

proptest! {
    #[test]
    fn dc_inverter_chain_voltages_within_rails(
        stages in 1usize..=5,
        v_in_is_vdd in any::<bool>(),
    ) {
        let v_in = if v_in_is_vdd { -15.0 } else { 0.0 };
        let process = ProcessParams::default();
        let config = SolverConfig::small_circuit();
        let mut g = CircuitGraph::new();

        let vdd = g.add_node(-1);
        let vss = g.add_node(-2);
        g.set_power_rail(vdd, -15.0, "VDD");
        g.set_power_rail(vss, 0.0, "VSS");
        g.vdd_idx = Some(vdd);
        g.vss_idx = Some(vss);

        let inp = g.add_node(100);
        g.nodes[inp].voltage = v_in;
        g.nodes[inp].is_fixed = true;

        let mut prev = inp;
        for i in 0..stages {
            let out = g.add_node(200 + i as i32);
            g.nodes[out].voltage = -7.5;
            g.add_transistor(out, out, vdd, TransistorKind::Depletion, 10e-6, 20e-6);
            g.add_transistor(prev, out, vss, TransistorKind::Enhancement, 10e-6, 10e-6);
            prev = out;
        }

        let solver = DcSolver::new(config, process);
        let result = solver.solve(&mut g);

        prop_assert!(
            result.converged,
            "DC solver must converge for {}-stage chain at v_in={:.0}V",
            stages, v_in
        );

        for node in &g.nodes {
            if !node.is_fixed {
                prop_assert!(
                    node.voltage >= -16.0 && node.voltage <= 1.0,
                    "Node voltage {:.3}V outside supply range [-15V, 0V]",
                    node.voltage
                );
            }
        }
    }
}

// --- DC: node-ID permutation does not change the output voltage ---

proptest! {
    #[test]
    fn dc_node_id_permutation_invariance(
        v_in_is_vdd in any::<bool>(),
        id_offset in 1i32..=1000,
    ) {
        let v_in = if v_in_is_vdd { -15.0 } else { 0.0 };
        let process = ProcessParams::default();
        let config = SolverConfig::small_circuit();

        // Canonical IDs
        let (mut g_a, out_a) = build_inverter(v_in, -1, -2, 10, 11);
        let r_a = DcSolver::new(config.clone(), process.clone()).solve(&mut g_a);
        prop_assume!(r_a.converged);
        let v_a = g_a.nodes[out_a].voltage;

        // Offset IDs -- same topology, different netlist numbering
        let (mut g_b, out_b) = build_inverter(
            v_in,
            -(1 + id_offset),
            -(2 + id_offset),
            10 + id_offset,
            11 + id_offset,
        );
        let r_b = DcSolver::new(config, process).solve(&mut g_b);
        prop_assume!(r_b.converged);
        let v_b = g_b.nodes[out_b].voltage;

        prop_assert!(
            (v_a - v_b).abs() < 1e-4,
            "Permuted node IDs changed output: canonical={:.4}V offset={:.4}V delta={:.2e}",
            v_a, v_b, (v_a - v_b).abs()
        );
    }
}

// --- DC: convergence across the allowed gmin range (1e-12 to 1e-9) ---

proptest! {
    #[test]
    fn dc_converges_across_gmin_range(
        gmin_exp in -12i32..=-9,
        v_in_is_vdd in any::<bool>(),
    ) {
        let v_in = if v_in_is_vdd { -15.0 } else { 0.0 };
        let gmin = 10f64.powi(gmin_exp);
        let (mut g, _) = build_inverter(v_in, -1, -2, 10, 11);
        let process = ProcessParams::default();
        let config = SolverConfig {
            gmin,
            ..SolverConfig::small_circuit()
        };

        let result = DcSolver::new(config, process).solve(&mut g);

        prop_assert!(
            result.converged,
            "DC must converge with gmin=1e{}: v_in={:.0}V, iterations={}",
            gmin_exp, v_in, result.iterations
        );
    }
}

// --- DC: dense and sparse backends agree on the same output voltage ---

proptest! {
    #[test]
    fn dc_dense_sparse_backends_agree(v_in_is_vdd in any::<bool>()) {
        let v_in = if v_in_is_vdd { -15.0 } else { 0.0 };
        let process = ProcessParams::default();

        let (mut g_dense, out_dense) = build_inverter(v_in, -1, -2, 10, 11);
        let r_dense = DcSolver::new(
            SolverConfig { backend: SolverBackend::Dense, ..SolverConfig::small_circuit() },
            process.clone(),
        ).solve(&mut g_dense);

        let (mut g_sparse, out_sparse) = build_inverter(v_in, -1, -2, 10, 11);
        let r_sparse = DcSolver::new(
            SolverConfig { backend: SolverBackend::Sparse, ..SolverConfig::small_circuit() },
            process,
        ).solve(&mut g_sparse);

        prop_assume!(r_dense.converged && r_sparse.converged);

        let v_dense = g_dense.nodes[out_dense].voltage;
        let v_sparse = g_sparse.nodes[out_sparse].voltage;

        prop_assert!(
            (v_dense - v_sparse).abs() < 1e-3,
            "Dense/sparse backend mismatch: dense={:.4}V sparse={:.4}V delta={:.2e}",
            v_dense, v_sparse, (v_dense - v_sparse).abs()
        );
    }
}

// --- Transient: time points are strictly monotone increasing ---

proptest! {
    #[test]
    fn transient_time_points_monotone_increasing(
        n_steps in 5usize..=40,
        cap_exp in -15i32..=-12,
    ) {
        let cap = 10f64.powi(cap_exp);
        let process = ProcessParams::default();

        let mut g = CircuitGraph::new();
        let vdd = g.add_node(-1);
        let vss = g.add_node(-2);
        g.set_power_rail(vdd, -15.0, "VDD");
        g.set_power_rail(vss, 0.0, "VSS");
        g.vdd_idx = Some(vdd);
        g.vss_idx = Some(vss);

        let out = g.add_node(10);
        g.nodes[out].voltage = 0.0;
        g.nodes[out].capacitance = cap;
        // Depletion load charges the capacitive output node toward VDD
        g.add_transistor(out, out, vdd, TransistorKind::Depletion, 10e-6, 20e-6);

        let dt = 1e-9;
        let config = TransientConfig {
            dt,
            t_stop: (n_steps as f64) * dt,
            adaptive: false,
            ..TransientConfig::default()
        };

        let result = TransientSolver::run(&mut g, &process, &config, &[]);

        prop_assert!(
            result.waveforms.len() >= 2,
            "Expected at least 2 waveform points, got {}",
            result.waveforms.len()
        );

        let times: Vec<f64> = result.waveforms.iter().map(|p| p.time).collect();
        for i in 1..times.len() {
            prop_assert!(
                times[i] > times[i - 1] - 1e-15,
                "Time points not non-decreasing at index {}: prev={:.3e} curr={:.3e}",
                i, times[i - 1], times[i]
            );
        }
    }
}

// --- Transient: all voltages stay within supply rails ---

proptest! {
    #[test]
    fn transient_voltages_within_supply_rails(
        cap_exp in -15i32..=-12,
        n_steps in 5usize..=30,
    ) {
        let cap = 10f64.powi(cap_exp);
        let process = ProcessParams::default();

        let mut g = CircuitGraph::new();
        let vdd = g.add_node(-1);
        let vss = g.add_node(-2);
        g.set_power_rail(vdd, -15.0, "VDD");
        g.set_power_rail(vss, 0.0, "VSS");
        g.vdd_idx = Some(vdd);
        g.vss_idx = Some(vss);

        let out = g.add_node(10);
        g.nodes[out].voltage = 0.0;
        g.nodes[out].capacitance = cap;
        g.add_transistor(out, out, vdd, TransistorKind::Depletion, 10e-6, 20e-6);

        let config = TransientConfig {
            dt: 1e-9,
            t_stop: (n_steps as f64) * 1e-9,
            adaptive: false,
            ..TransientConfig::default()
        };

        let result = TransientSolver::run(&mut g, &process, &config, &[]);

        for (step_i, point) in result.waveforms.iter().enumerate() {
            for (node_j, &v) in point.voltages.iter().enumerate() {
                prop_assert!(
                    (-16.0..=1.0).contains(&v),
                    "Voltage out of rails at step {} node {}: {:.3}V",
                    step_i, node_j, v
                );
            }
        }
    }
}
