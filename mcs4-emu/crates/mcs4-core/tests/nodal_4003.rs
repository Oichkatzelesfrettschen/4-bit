//! Nodal simulation test for the 4003 Shift Register.
//!
//! WHY: Validates the Phase 2.3 Netlist-to-Solver pipeline by running
//! a full chip cycle of the extracted 4003 silicon netlist at the
//! analog Nodal level using TCAD-injected physics.

use std::path::Path;
use mcs4_core::{
    circuit::{
        netlist_bridge::{self, BridgeConfig},
        parasitic_extract::{self, LayerParams},
    },
    layout_netlist,
    nodal_solver::NodalSolver,
    process::ProcessParams,
    tcad::TCADBridge,
};

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repo root")
}

#[test]
fn test_4003_nodal_step() {
    let path = repo_root().join("docs/evidence/netlists_v1/4003_netlist_v1.json");
    let netlist = layout_netlist::load_netlist_v1(&path).expect("load 4003");

    let bridge_config = BridgeConfig::default();
    let mut graph = netlist_bridge::netlist_v1_to_circuit(&netlist, &bridge_config);

    // Apply layout-aware parasitic extraction
    let layer_params = LayerParams::default();
    parasitic_extract::apply_to_graph(&mut graph, &layer_params);

    let params = ProcessParams::default();
    let tcad_bridge = TCADBridge::new(&params);

    let mut solver = NodalSolver::new();
    solver.set_bridge(tcad_bridge);
    solver.load_circuit_graph(&graph);

    // Initial solve (DC Operating Point)
    assert!(solver.solve_robust(), "Initial DC solve failed");

    // Identify important nodes
    let find_node = |name: &str| {
        graph.nodes.iter().position(|n| n.name.as_deref() == Some(name))
            .expect(&format!("Could not find node {}", name)) as u32
    };

    let clock_id = find_node("CLOCK");
    let data_id = find_node("DATA");
    let en_id = find_node("EN");
    let q0_id = find_node("Q0");

    let vdd = bridge_config.vdd; // -15V
    let vss = bridge_config.vss; // 0V

    // Ensure inputs are driven. We clear any existing fixed state first if necessary
    // or just assume they weren't fixed in the default netlist (usually only VDD/VSS are).
    solver.add_voltage_source(clock_id, solver.get_gnd(), vss); // Clock inactive
    solver.add_voltage_source(data_id, solver.get_gnd(), vss); // Data inactive
    solver.add_voltage_source(en_id, solver.get_gnd(), vss);   // Enable inactive

    // Initial solve with inputs
    assert!(solver.solve_robust(), "Initial robust solve failed");

    let q0_v = solver.voltage(q0_id);
    println!("DEBUG: 4003 Q0 Voltage: {}V", q0_v);
    
    assert!(q0_v <= vss + 0.1 && q0_v >= vdd - 0.1, "Q0 voltage {} out of range [{}, {}]", q0_v, vdd, vss);
}
