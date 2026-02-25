#![allow(missing_docs)]
//! Integration validation tests for transistor-level simulation.
//!
//! These tests validate the DC operating point solver against
//! extracted netlists from real MCS-4 chips, and cross-validate
//! transistor-level results against gate-level truth tables.

use std::path::Path;

use mcs4_core::circuit::netlist_bridge::{self, BridgeConfig};
use mcs4_core::circuit::graph::{CircuitGraph, TransistorKind};
use mcs4_core::device::DeviceModel;
use mcs4_core::device::pmos_level1::PmosLevel1;
use mcs4_core::layout_netlist;
use mcs4_core::process::ProcessParams;
use mcs4_core::solver::{DcSolver, SolverBackend, SolverConfig};

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repo root")
}

/// Validate that the DC solver converges on the 4003 netlist (37 transistors).
#[test]
fn dc_op_4003_converges() {
    let path = repo_root().join("docs/evidence/netlists_v1/4003_netlist_v1.json");
    let netlist = layout_netlist::load_netlist_v1(&path).expect("load 4003");

    let config = BridgeConfig::default();
    let mut graph = netlist_bridge::netlist_v1_to_circuit(&netlist, &config);

    assert_eq!(graph.transistor_count(), 37,
        "4003 should have 37 transistors");

    let solver_config = SolverConfig::robust();
    let process = ProcessParams::default();
    let solver = DcSolver::new(solver_config, process);

    let result = solver.solve(&mut graph);

    assert!(result.converged,
        "DC operating point should converge on 4003, \
         iterations={}, delta={:.3e}",
        result.iterations, result.final_delta);
}

/// Verify that all 4003 node voltages are within the supply rails.
#[test]
fn dc_op_4003_voltages_in_range() {
    let path = repo_root().join("docs/evidence/netlists_v1/4003_netlist_v1.json");
    let netlist = layout_netlist::load_netlist_v1(&path).expect("load 4003");

    let config = BridgeConfig::default();
    let mut graph = netlist_bridge::netlist_v1_to_circuit(&netlist, &config);

    let solver_config = SolverConfig::robust();
    let process = ProcessParams::default();
    let solver = DcSolver::new(solver_config, process);
    let result = solver.solve(&mut graph);

    assert!(result.converged);

    let vdd = config.vdd; // -15V
    let vss = config.vss; // 0V
    let margin = 1.0; // Allow 1V margin outside rails

    for node in &graph.nodes {
        assert!(
            node.voltage >= vdd - margin && node.voltage <= vss + margin,
            "Node {:?} (id={}) voltage {:.2}V outside rails [{:.1}V, {:.1}V]",
            node.name, node.netlist_id, node.voltage, vdd - margin, vss + margin
        );
    }
}

/// Cross-validate: transistor-level inverter matches gate-level truth table.
#[test]
fn inverter_transistor_vs_gate_level() {
    let process = ProcessParams::default();
    let solver_config = SolverConfig::small_circuit();

    // Build transistor-level inverter
    let test_cases = [
        (0.0, true),     // Input at VSS -> output near VDD (logic high for pMOS)
        (-15.0, false),  // Input at VDD -> output near VSS (logic low for pMOS)
    ];

    for &(v_in, expect_output_near_vdd) in &test_cases {
        let mut g = CircuitGraph::new();

        let vdd = g.add_node(-1);
        let vss = g.add_node(-2);
        let input = g.add_node(10);
        let output = g.add_node(11);

        g.set_power_rail(vdd, -15.0, "VDD");
        g.set_power_rail(vss, 0.0, "VSS");
        g.vdd_idx = Some(vdd);
        g.vss_idx = Some(vss);

        g.nodes[input].voltage = v_in;
        g.nodes[input].is_fixed = true;
        g.nodes[output].voltage = -7.5;

        // Depletion load
        g.add_transistor(output, output, vdd, TransistorKind::Depletion, 10e-6, 20e-6);
        // Enhancement driver
        g.add_transistor(input, output, vss, TransistorKind::Enhancement, 10e-6, 10e-6);

        let solver = DcSolver::new(solver_config.clone(), process.clone());
        let result = solver.solve(&mut g);
        assert!(result.converged);

        let v_out = g.nodes[output.min(g.nodes.len() - 1)].voltage;
        let midpoint = -7.5;

        if expect_output_near_vdd {
            assert!(v_out < midpoint,
                "Vin={:.0}V: expected output near VDD(-15V), got {:.2}V", v_in, v_out);
        } else {
            assert!(v_out > midpoint,
                "Vin={:.0}V: expected output near VSS(0V), got {:.2}V", v_in, v_out);
        }
    }
}

/// Cross-validate: transistor-level NAND2 matches gate-level truth table.
#[test]
fn nand2_transistor_vs_gate_level() {
    let process = ProcessParams::default();
    let solver_config = SolverConfig::small_circuit();

    // NAND truth table (pMOS logic, inverted):
    // A=0, B=0 -> OUT = VDD (high)
    // A=VDD, B=0 -> OUT = VDD (high)
    // A=0, B=VDD -> OUT = VDD (high)
    // A=VDD, B=VDD -> OUT = VSS (low)
    let test_cases: [(f64, f64, bool); 4] = [
        (0.0, 0.0, true),       // Both OFF -> output high
        (-15.0, 0.0, true),     // A on, B off -> output high
        (0.0, -15.0, true),     // A off, B on -> output high
        (-15.0, -15.0, false),  // Both ON -> output low
    ];

    for &(va, vb, expect_high) in &test_cases {
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

        g.nodes[in_a].voltage = va;
        g.nodes[in_a].is_fixed = true;
        g.nodes[in_b].voltage = vb;
        g.nodes[in_b].is_fixed = true;
        g.nodes[output].voltage = -7.5;
        g.nodes[mid].voltage = -7.5;

        // Depletion load
        g.add_transistor(output, output, vdd, TransistorKind::Depletion, 10e-6, 20e-6);
        // Series enhancement: A drives output to mid
        g.add_transistor(in_a, output, mid, TransistorKind::Enhancement, 10e-6, 10e-6);
        // Series enhancement: B drives mid to VSS
        g.add_transistor(in_b, mid, vss, TransistorKind::Enhancement, 10e-6, 10e-6);

        let solver = DcSolver::new(solver_config.clone(), process.clone());
        let result = solver.solve(&mut g);
        assert!(result.converged,
            "NAND2 should converge at A={:.0}, B={:.0}", va, vb);

        let v_out = g.nodes[4].voltage; // output index
        let midpoint = -7.5;

        if expect_high {
            assert!(v_out < midpoint,
                "NAND2 A={:.0}, B={:.0}: expected high (near VDD), got {:.2}V", va, vb, v_out);
        } else {
            assert!(v_out > midpoint,
                "NAND2 A={:.0}, B={:.0}: expected low (near VSS), got {:.2}V", va, vb, v_out);
        }
    }
}

/// Verify transistor model I-V characteristics match expected pMOS behavior.
#[test]
fn pmos_iv_characteristics() {
    let process = ProcessParams::default();
    let model = PmosLevel1::new(&process, 10e-6, 10e-6);

    // Cutoff: Vgs = 0 (gate at source level)
    // With subthreshold model, deeply-off device has negligible but non-zero current
    let ids_cutoff = model.ids(0.0, -15.0);
    assert!(ids_cutoff.abs() < 1e-12,
        "Should be negligible in deep cutoff at Vgs=0, got {:.3e}", ids_cutoff);

    // Strong inversion: Vgs = -15V
    let ids_strong = model.ids(-15.0, -15.0);
    assert!(ids_strong > 0.0,
        "Should conduct strongly at Vgs=-15V");

    // Current should be in microamp range for 10um/10um transistor
    assert!(ids_strong > 1e-6 && ids_strong < 1e-2,
        "Ids = {:.3e} A, expected uA to mA range", ids_strong);
}

/// Verify propagation delay estimate from transistor solver is
/// within reasonable range of gate-level analytical estimate.
#[test]
fn propagation_delay_order_of_magnitude() {
    let process = ProcessParams::default();

    // Gate-level estimate: delay ~ R_on * C_load
    // Ron ~ 10k ohm (from transistor.rs convention)
    // Cload ~ 50fF (typical node)
    let gate_delay_estimate = 10_000.0 * 50e-15; // 500 ps = 0.5 ns

    // Transistor-level estimate: use beta and capacitance
    let model = PmosLevel1::new(&process, 10e-6, 10e-6);
    let ids_sat = model.ids(-15.0, -15.0);
    let c_load = 50e-15; // 50 fF
    let v_swing = 15.0; // full supply swing

    // Propagation delay ~ C * V / (2 * Ids)
    let transistor_delay = c_load * v_swing / (2.0 * ids_sat);

    // Both estimates should be in the nanosecond range
    assert!(gate_delay_estimate > 1e-12 && gate_delay_estimate < 1e-6,
        "Gate estimate = {:.3e} s", gate_delay_estimate);
    assert!(transistor_delay > 1e-12 && transistor_delay < 1e-6,
        "Transistor estimate = {:.3e} s", transistor_delay);

    // They should agree within 2 orders of magnitude
    let ratio = transistor_delay / gate_delay_estimate;
    assert!(ratio > 0.01 && ratio < 100.0,
        "Delay ratio = {:.2}, estimates too far apart", ratio);
}

/// Validate the 4001 netlist can be parsed (larger chip: 256 transistors expected).
#[test]
fn parse_4001_netlist() {
    let path = repo_root().join("docs/evidence/netlists_v1/4001_netlist_v1.json");
    let netlist = layout_netlist::load_netlist_v1(&path).expect("load 4001");
    assert_eq!(netlist.chip, "4001");

    let config = BridgeConfig::default();
    let graph = netlist_bridge::netlist_v1_to_circuit(&netlist, &config);

    assert!(graph.transistor_count() > 0,
        "4001 should have transistors, got {}", graph.transistor_count());

    // All node references valid
    for trans in &graph.transistors {
        assert!(trans.gate < graph.nodes.len());
        assert!(trans.a_node < graph.nodes.len());
        assert!(trans.b_node < graph.nodes.len());
    }
}

/// Validate the 4004 netlist can be parsed (2300 transistors).
#[test]
fn parse_4004_netlist() {
    let path = repo_root().join("docs/evidence/netlists_v1/4004_netlist_v1.json");
    let netlist = layout_netlist::load_netlist_v1(&path).expect("load 4004");
    assert_eq!(netlist.chip, "4004");

    let config = BridgeConfig::default();
    let graph = netlist_bridge::netlist_v1_to_circuit(&netlist, &config);

    assert!(graph.transistor_count() > 100,
        "4004 should have many transistors, got {}", graph.transistor_count());
}

// ============================================================
// Sparse vs Dense equivalence tests
// ============================================================

/// Helper: load a chip netlist and build a circuit graph.
fn load_chip_graph(chip_name: &str) -> CircuitGraph {
    let filename = format!("{}_netlist_v1.json", chip_name);
    let path = repo_root().join("docs/evidence/netlists_v1").join(filename);
    let netlist = layout_netlist::load_netlist_v1(&path)
        .unwrap_or_else(|e| panic!("load {} netlist: {}", chip_name, e));
    let config = BridgeConfig::default();
    netlist_bridge::netlist_v1_to_circuit(&netlist, &config)
}

/// Sparse solver on 4003 produces same results as dense within tolerance.
#[test]
fn sparse_4003_matches_dense_4003() {
    let process = ProcessParams::default();

    // Dense solve
    let mut graph_dense = load_chip_graph("4003");
    let config_dense = SolverConfig {
        backend: SolverBackend::Dense,
        ..SolverConfig::robust()
    };
    let result_dense = DcSolver::new(config_dense, process.clone()).solve(&mut graph_dense);
    assert!(result_dense.converged, "Dense 4003 should converge");

    // Sparse solve
    let mut graph_sparse = load_chip_graph("4003");
    let config_sparse = SolverConfig {
        backend: SolverBackend::Sparse,
        ..SolverConfig::robust()
    };
    let result_sparse = DcSolver::new(config_sparse, process).solve(&mut graph_sparse);
    assert!(result_sparse.converged, "Sparse 4003 should converge");

    // Compare voltages
    let tol = 1e-4; // 0.1 mV tolerance
    for (i, (&vd, &vs)) in result_dense.voltages.iter()
        .zip(result_sparse.voltages.iter())
        .enumerate()
    {
        assert!(
            (vd - vs).abs() < tol,
            "4003 node {}: dense={:.6}V, sparse={:.6}V, diff={:.2e}V",
            i, vd, vs, (vd - vs).abs()
        );
    }
}

/// Sparse solve on 4003 completes in reasonable time.
#[test]
fn sparse_4003_performance() {
    let process = ProcessParams::default();
    let mut graph = load_chip_graph("4003");
    let config = SolverConfig {
        backend: SolverBackend::Sparse,
        ..SolverConfig::robust()
    };

    let start = std::time::Instant::now();
    let result = DcSolver::new(config, process).solve(&mut graph);
    let elapsed = start.elapsed();

    assert!(result.converged, "Sparse 4003 should converge");
    assert!(
        elapsed.as_secs_f64() < 5.0,
        "Sparse 4003 took {:.2}s, expected < 5s",
        elapsed.as_secs_f64()
    );
}

// ============================================================
// Chip scaling tests (4002, 4004, 4001)
// ============================================================

/// 4002 DC operating point converges with sparse solver.
#[test]
fn dc_op_4002_converges() {
    let mut graph = load_chip_graph("4002");
    let num_transistors = graph.transistor_count();
    let num_free = graph.free_node_count();

    eprintln!("4002: {} transistors, {} free nodes", num_transistors, num_free);

    let config = SolverConfig::robust();
    let process = ProcessParams::default();
    let result = DcSolver::new(config, process).solve(&mut graph);

    assert!(
        result.converged,
        "4002 DC should converge, iterations={}, delta={:.3e}",
        result.iterations, result.final_delta
    );
}

/// 4002 node voltages are within supply rails.
#[test]
fn dc_op_4002_voltages_in_range() {
    let mut graph = load_chip_graph("4002");
    let config = SolverConfig::robust();
    let process = ProcessParams::default();
    let result = DcSolver::new(config, process).solve(&mut graph);

    assert!(result.converged);

    let vdd = -15.0;
    let vss = 0.0;
    let margin = 1.0;

    for node in &graph.nodes {
        assert!(
            node.voltage >= vdd - margin && node.voltage <= vss + margin,
            "4002 node {:?} (id={}) voltage {:.2}V outside [{:.1}, {:.1}]V",
            node.name, node.netlist_id, node.voltage, vdd - margin, vss + margin
        );
    }
}

/// 4002 convergence iteration count is reasonable.
#[test]
fn dc_op_4002_iteration_count() {
    let mut graph = load_chip_graph("4002");
    let config = SolverConfig::robust();
    let process = ProcessParams::default();
    let result = DcSolver::new(config, process).solve(&mut graph);

    assert!(result.converged);
    assert!(
        result.iterations <= 200 * 20, // max_nr_iterations * source_steps
        "4002 took {} iterations, expected reasonable count", result.iterations
    );
}

/// 4004 DC operating point converges with sparse solver.
#[test]
fn dc_op_4004_converges() {
    let mut graph = load_chip_graph("4004");
    let num_transistors = graph.transistor_count();
    let num_free = graph.free_node_count();

    eprintln!("4004: {} transistors, {} free nodes", num_transistors, num_free);

    let config = SolverConfig::robust();
    let process = ProcessParams::default();
    let result = DcSolver::new(config, process).solve(&mut graph);

    assert!(
        result.converged,
        "4004 DC should converge, iterations={}, delta={:.3e}",
        result.iterations, result.final_delta
    );
}

/// 4004 node voltages are within supply rails.
#[test]
fn dc_op_4004_voltages_in_range() {
    let mut graph = load_chip_graph("4004");
    let config = SolverConfig::robust();
    let process = ProcessParams::default();
    let result = DcSolver::new(config, process).solve(&mut graph);

    assert!(result.converged);

    let vdd = -15.0;
    let vss = 0.0;
    let margin = 1.0;

    for node in &graph.nodes {
        assert!(
            node.voltage >= vdd - margin && node.voltage <= vss + margin,
            "4004 node {:?} (id={}) voltage {:.2}V outside [{:.1}, {:.1}]V",
            node.name, node.netlist_id, node.voltage, vdd - margin, vss + margin
        );
    }
}

/// 4004 DC solve completes in reasonable time.
#[test]
fn dc_op_4004_performance() {
    let mut graph = load_chip_graph("4004");
    let config = SolverConfig::robust();
    let process = ProcessParams::default();

    let start = std::time::Instant::now();
    let result = DcSolver::new(config, process).solve(&mut graph);
    let elapsed = start.elapsed();

    assert!(result.converged, "4004 should converge");
    assert!(
        elapsed.as_secs_f64() < 60.0,
        "4004 DC solve took {:.1}s, expected < 60s",
        elapsed.as_secs_f64()
    );
}

/// 4001 DC operating point converges (largest chip, stretch goal).
#[test]
fn dc_op_4001_converges() {
    let mut graph = load_chip_graph("4001");
    let num_transistors = graph.transistor_count();
    let num_free = graph.free_node_count();

    eprintln!("4001: {} transistors, {} free nodes", num_transistors, num_free);

    let config = SolverConfig::robust();
    let process = ProcessParams::default();
    let result = DcSolver::new(config, process).solve(&mut graph);

    assert!(
        result.converged,
        "4001 DC should converge, iterations={}, delta={:.3e}",
        result.iterations, result.final_delta
    );
}

/// 4001 node voltages are within supply rails.
#[test]
fn dc_op_4001_voltages_in_range() {
    let mut graph = load_chip_graph("4001");
    let config = SolverConfig::robust();
    let process = ProcessParams::default();
    let result = DcSolver::new(config, process).solve(&mut graph);

    assert!(result.converged);

    let vdd = -15.0;
    let vss = 0.0;
    let margin = 1.0;

    for node in &graph.nodes {
        assert!(
            node.voltage >= vdd - margin && node.voltage <= vss + margin,
            "4001 node {:?} (id={}) voltage {:.2}V outside [{:.1}, {:.1}]V",
            node.name, node.netlist_id, node.voltage, vdd - margin, vss + margin
        );
    }
}
