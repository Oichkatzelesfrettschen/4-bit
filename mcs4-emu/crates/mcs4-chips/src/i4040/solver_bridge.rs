//! Solver bridge for the Intel 4040 CPU.

use mcs4_core::{
    bridge::{ChipSolverBridge, PinMapping, PinDirection},
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
        vec!["clock_buffer"]
    }

    fn subcircuit(&self, name: &str) -> Option<CircuitGraph> {
        match name {
            // For now, share the 4004's reference subcircuits
            "clock_buffer" => Some(I4004::build_clock_buffer()),
            _ => None,
        }
    }

    fn pin_map(&self) -> Vec<PinMapping> {
        // 4040 pin mapping (extends 4004)
        vec![
            PinMapping { name: "CLK1".to_string(), node_id: 415, direction: PinDirection::Input },
            PinMapping { name: "CLK2".to_string(), node_id: 1230, direction: PinDirection::Input },
            PinMapping { name: "SYNC".to_string(), node_id: 1261, direction: PinDirection::Output },
            PinMapping { name: "RESET".to_string(), node_id: 1232, direction: PinDirection::Input },
            PinMapping { name: "TEST".to_string(), node_id: 1267, direction: PinDirection::Input },
            PinMapping { name: "CMROM".to_string(), node_id: 716, direction: PinDirection::Output },
            PinMapping { name: "D0".to_string(), node_id: 3442, direction: PinDirection::Bidirectional },
            PinMapping { name: "D1".to_string(), node_id: 598, direction: PinDirection::Bidirectional },
            PinMapping { name: "D2".to_string(), node_id: 3426, direction: PinDirection::Bidirectional },
            PinMapping { name: "D3".to_string(), node_id: 2815, direction: PinDirection::Bidirectional },
            PinMapping { name: "CMRAM0".to_string(), node_id: 3431, direction: PinDirection::Output },
            PinMapping { name: "CMRAM1".to_string(), node_id: 3428, direction: PinDirection::Output },
            PinMapping { name: "CMRAM2".to_string(), node_id: 2997, direction: PinDirection::Output },
            PinMapping { name: "CMRAM3".to_string(), node_id: 405, direction: PinDirection::Output },
            // 4040 extensions (node IDs estimated for now)
            PinMapping { name: "INT".to_string(), node_id: 9999, direction: PinDirection::Input },
            PinMapping { name: "STOP".to_string(), node_id: 9998, direction: PinDirection::Input },
        ]
    }
}
