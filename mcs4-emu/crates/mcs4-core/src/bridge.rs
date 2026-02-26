//! Solver-to-chip bridge trait for multi-fidelity simulation.
//!
//! WHY: Behavioral chip models (Phase 1-3) run fast but lack analog accuracy.
//! The solver infrastructure (Phase 4) provides transistor-level simulation
//! but has no connection to the chip behavioral models. This bridge connects
//! them, allowing any chip to expose subcircuits for analog characterization.
//!
//! WHAT: A trait that chip implementations can implement to expose named
//! subcircuits as `CircuitGraph` instances for DC/transient analysis.
//!
//! HOW: Each chip enumerates its available subcircuits (e.g., "clock_buffer",
//! "alu_adder") and returns a `CircuitGraph` for the requested subcircuit.
//! The graph can then be fed to `DcSolver` or `TransientSolver`.

use crate::{circuit::graph::CircuitGraph, fidelity::SimulationFidelity};

/// Direction of a physical chip pin
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PinDirection {
    /// Driven by the system, read by the chip
    Input,
    /// Driven by the chip, read by the system
    Output,
    /// Can be driven by either depending on bus phase
    Bidirectional,
}

/// Mapping from a physical pin name to a node in the circuit graph
#[derive(Clone, Debug)]
pub struct PinMapping {
    /// Human-readable pin name (e.g., "DATA0", "SYNC")
    pub name: String,
    /// Internal node ID in the extracted netlist
    pub node_id: u32,
    /// Directionality of the pin
    pub direction: PinDirection,
}

/// Bridge between chip behavioral models and the circuit solver.
///
/// Implementors expose named subcircuits that can be extracted and
/// simulated at the transistor level for analog validation.
pub trait ChipSolverBridge {
    /// Current simulation fidelity level for this chip.
    fn fidelity(&self) -> SimulationFidelity;

    /// Set the simulation fidelity level.
    fn set_fidelity(&mut self, fidelity: SimulationFidelity);

    /// List all available subcircuit names.
    ///
    /// These names are stable identifiers used with `subcircuit()` to
    /// retrieve the transistor-level graph for a specific functional block.
    fn subcircuit_names(&self) -> Vec<&str>;

    /// Retrieve the `CircuitGraph` for a named subcircuit.
    ///
    /// Returns `None` if the subcircuit name is not recognized.
    /// The returned graph is fully populated with nodes, transistors,
    /// power rails, and initial voltages ready for solver consumption.
    fn subcircuit(&self, name: &str) -> Option<CircuitGraph>;

    /// Return the physical pin mapping for the full chip.
    fn pin_map(&self) -> Vec<PinMapping>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal test implementation to verify the trait compiles and works
    struct MockChip {
        fidelity: SimulationFidelity,
    }

    impl ChipSolverBridge for MockChip {
        fn fidelity(&self) -> SimulationFidelity {
            self.fidelity
        }

        fn set_fidelity(&mut self, fidelity: SimulationFidelity) {
            self.fidelity = fidelity;
        }

        fn subcircuit_names(&self) -> Vec<&str> {
            vec!["inverter"]
        }

        fn subcircuit(&self, name: &str) -> Option<CircuitGraph> {
            if name == "inverter" {
                Some(CircuitGraph::new())
            } else {
                None
            }
        }
    }

    #[test]
    fn mock_chip_bridge() {
        let mut chip = MockChip {
            fidelity: SimulationFidelity::Behavioral,
        };
        assert_eq!(chip.fidelity(), SimulationFidelity::Behavioral);

        chip.set_fidelity(SimulationFidelity::TransistorLevel);
        assert_eq!(chip.fidelity(), SimulationFidelity::TransistorLevel);

        assert_eq!(chip.subcircuit_names(), vec!["inverter"]);
        assert!(chip.subcircuit("inverter").is_some());
        assert!(chip.subcircuit("nonexistent").is_none());
    }
}
