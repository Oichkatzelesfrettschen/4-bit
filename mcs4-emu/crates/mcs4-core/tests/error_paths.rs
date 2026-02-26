//! Error path tests for solvers and simulation primitives (D.2.4)
//!
//! Validates graceful behavior when accessing nonexistent nodes,
//! operating on empty circuits, and hitting boundary conditions.

use mcs4_core::{
    nodal_solver::NodalSolver,
    transistor_solver::{Circuit, NodeId, TransistorSimulator, VoltageLevel},
};

// --- NodalSolver error paths ---

#[test]
fn nodal_solver_missing_node_returns_zero() {
    let solver = NodalSolver::new();
    // Requesting voltage of a node that was never added should return 0.0
    assert_eq!(solver.voltage(999), 0.0);
    assert_eq!(solver.voltage(0), 0.0);
    assert_eq!(solver.voltage(u32::MAX), 0.0);
}

#[test]
fn nodal_solver_set_voltage_on_missing_node_is_noop() {
    let mut solver = NodalSolver::new();
    // Setting voltage on nonexistent node should not panic
    solver.set_voltage(42, 5.0);
    // Still returns default 0.0
    assert_eq!(solver.voltage(42), 0.0);
}

#[test]
fn nodal_solver_empty_network_converges() {
    let mut solver = NodalSolver::new();
    assert!(solver.solve(), "Empty network should converge trivially");
}

#[test]
fn nodal_solver_single_fixed_node() {
    let mut solver = NodalSolver::new();
    solver.add_node(0, "VDD".to_string(), 5.0, 0.0, true);
    assert!(solver.solve());
    assert_eq!(solver.voltage(0), 5.0);
}

#[test]
fn nodal_solver_conductance_to_missing_node() {
    let mut solver = NodalSolver::new();
    solver.add_node(0, "N0".to_string(), 2.5, 1.0, false);
    // Add conductance referencing a node that does not exist
    solver.add_conductance(0, 999, 0.001);
    // Should converge without panic -- missing neighbor is simply skipped
    let converged = solver.solve();
    assert!(converged);
    // Node voltage unchanged because the neighbor does not exist
    assert_eq!(solver.voltage(0), 2.5);
}

#[test]
fn nodal_solver_zero_conductance() {
    let mut solver = NodalSolver::new();
    solver.add_node(0, "VDD".to_string(), 5.0, 0.0, true);
    solver.add_node(1, "N".to_string(), 0.0, 1.0, false);
    // Zero conductance = open circuit
    solver.add_conductance(0, 1, 0.0);
    assert!(solver.solve());
    // Node should remain at initial voltage (no current flow)
    assert_eq!(solver.voltage(1), 0.0);
}

// --- TransistorSimulator error paths ---

#[test]
fn transistor_sim_missing_node_returns_low() {
    let vdd = NodeId::new(0);
    let vss = NodeId::new(1);
    let circuit = Circuit::new(vdd, vss);
    let sim = TransistorSimulator::new(circuit);

    // Nonexistent node should return default VoltageLevel::Low
    let missing = NodeId::new(999);
    assert_eq!(sim.node_voltage(missing), VoltageLevel::Low);
}

#[test]
fn transistor_sim_drive_missing_node_no_panic() {
    let vdd = NodeId::new(0);
    let vss = NodeId::new(1);
    let circuit = Circuit::new(vdd, vss);
    let mut sim = TransistorSimulator::new(circuit);

    // Driving a nonexistent node should not panic
    let missing = NodeId::new(999);
    let converged = sim.drive_input(missing, VoltageLevel::High);
    // Empty circuit (no transistors) should converge trivially
    assert!(converged);
}

#[test]
fn transistor_sim_empty_circuit_converges() {
    let vdd = NodeId::new(0);
    let vss = NodeId::new(1);
    let circuit = Circuit::new(vdd, vss);
    let mut sim = TransistorSimulator::new(circuit);

    let converged = sim.drive_input(vdd, VoltageLevel::High);
    assert!(converged);
    assert_eq!(sim.iteration_count(), 1);
}

#[test]
fn transistor_sim_power_rail_voltages() {
    let vdd = NodeId::new(0);
    let vss = NodeId::new(1);
    let circuit = Circuit::new(vdd, vss);
    let sim = TransistorSimulator::new(circuit);

    assert_eq!(sim.node_voltage(vdd), VoltageLevel::High);
    assert_eq!(sim.node_voltage(vss), VoltageLevel::Low);
}

#[test]
fn transistor_sim_initial_time_is_zero() {
    let vdd = NodeId::new(0);
    let vss = NodeId::new(1);
    let circuit = Circuit::new(vdd, vss);
    let sim = TransistorSimulator::new(circuit);

    assert_eq!(sim.time(), 0.0);
}

#[test]
fn transistor_sim_iteration_count_starts_zero() {
    let vdd = NodeId::new(0);
    let vss = NodeId::new(1);
    let circuit = Circuit::new(vdd, vss);
    let sim = TransistorSimulator::new(circuit);

    assert_eq!(sim.iteration_count(), 0);
}
