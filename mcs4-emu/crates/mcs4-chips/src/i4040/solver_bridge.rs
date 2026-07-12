//! Solver bridge contract for the Intel 4040 CPU.
//!
//! No extracted Intel 4040 transistor netlist exists in this repository.
//! The bridge therefore exposes no physical 4040 pin map or physical 4040
//! subcircuit. It exposes one explicitly labeled 4004 reference graph so
//! callers can exercise the solver interface without mistaking the graph for
//! 4040 silicon evidence.

use mcs4_core::{
    bridge::{ChipSolverBridge, PinMapping},
    circuit::graph::CircuitGraph,
    fidelity::SimulationFidelity,
};

use super::I4040;
use crate::i4004::I4004;

impl ChipSolverBridge for I4040 {
    fn fidelity(&self) -> SimulationFidelity {
        self.fidelity
    }

    fn set_fidelity(&mut self, fidelity: SimulationFidelity) {
        self.fidelity = fidelity;
    }

    fn subcircuit_names(&self) -> Vec<&str> {
        vec!["reference_4004_clock_buffer"]
    }

    fn subcircuit(&self, name: &str) -> Option<CircuitGraph> {
        match name {
            // This graph is a solver reference only. It is not an extracted
            // Intel 4040 subcircuit.
            "reference_4004_clock_buffer" => Some(I4004::build_clock_buffer()),
            _ => None,
        }
    }

    fn pin_map(&self) -> Vec<PinMapping> {
        // No extracted 4040 netlist establishes physical pin-to-node IDs.
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use mcs4_core::{
        bridge::ChipSolverBridge,
        process::ProcessParams,
        solver::{DcSolver, SolverConfig},
        SimulationFidelity,
    };

    use super::*;

    #[test]
    fn bridge_exposes_only_labeled_reference_subcircuit() {
        let cpu = I4040::new();
        assert_eq!(cpu.subcircuit_names(), vec!["reference_4004_clock_buffer"]);
        assert!(cpu.subcircuit("clock_buffer").is_none());
        assert!(cpu.subcircuit("unknown").is_none());
    }

    #[test]
    fn reference_graph_has_expected_solver_structure() {
        let cpu = I4040::new();
        let graph = cpu
            .subcircuit("reference_4004_clock_buffer")
            .expect("labeled reference graph");

        assert_eq!(graph.nodes.len(), 6);
        assert_eq!(graph.transistor_count(), 6);
        assert!(graph.vdd_idx.is_some());
        assert!(graph.vss_idx.is_some());
    }

    #[test]
    fn reference_graph_converges_without_claiming_i4040_silicon() {
        let cpu = I4040::new();
        let mut graph = cpu
            .subcircuit("reference_4004_clock_buffer")
            .expect("labeled reference graph");
        let solver = DcSolver::new(SolverConfig::small_circuit(), ProcessParams::default());
        let result = solver.solve(&mut graph);

        assert!(result.converged, "reference graph must converge");
    }

    #[test]
    fn physical_pin_map_stays_empty_without_i4040_netlist_evidence() {
        let cpu = I4040::new();
        assert!(cpu.pin_map().is_empty());
    }

    #[test]
    fn fidelity_transitions_remain_available() {
        let mut cpu = I4040::new();
        cpu.set_fidelity(SimulationFidelity::NodalLevel);
        assert_eq!(cpu.fidelity(), SimulationFidelity::NodalLevel);
    }
}
