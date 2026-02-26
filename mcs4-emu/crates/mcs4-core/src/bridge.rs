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
            vec!["inverter", "clock_buffer", "output_driver"]
        }

        fn subcircuit(&self, name: &str) -> Option<CircuitGraph> {
            match name {
                "inverter" | "clock_buffer" | "output_driver" => Some(CircuitGraph::new()),
                _ => None,
            }
        }

        fn pin_map(&self) -> Vec<PinMapping> {
            vec![
                PinMapping {
                    name: "D0".to_string(),
                    node_id: 10,
                    direction: PinDirection::Bidirectional,
                },
                PinMapping {
                    name: "D1".to_string(),
                    node_id: 11,
                    direction: PinDirection::Bidirectional,
                },
                PinMapping {
                    name: "SYNC".to_string(),
                    node_id: 20,
                    direction: PinDirection::Output,
                },
                PinMapping {
                    name: "CLK1".to_string(),
                    node_id: 30,
                    direction: PinDirection::Input,
                },
            ]
        }
    }

    #[test]
    fn mock_chip_bridge() {
        let mut chip = MockChip {
            fidelity: SimulationFidelity::Behavioral,
        };
        assert_eq!(chip.fidelity(), SimulationFidelity::Behavioral);

        chip.set_fidelity(SimulationFidelity::SwitchLevel);
        assert_eq!(chip.fidelity(), SimulationFidelity::SwitchLevel);

        assert_eq!(
            chip.subcircuit_names(),
            vec!["inverter", "clock_buffer", "output_driver"]
        );
        assert!(chip.subcircuit("inverter").is_some());
        assert!(chip.subcircuit("nonexistent").is_none());
    }

    #[test]
    fn fidelity_transitions_all_levels() {
        let mut chip = MockChip {
            fidelity: SimulationFidelity::Behavioral,
        };

        let levels = [
            SimulationFidelity::Behavioral,
            SimulationFidelity::PhaseAccurate,
            SimulationFidelity::SwitchLevel,
            SimulationFidelity::NodalLevel,
            SimulationFidelity::TCADLevel,
        ];

        for &level in &levels {
            chip.set_fidelity(level);
            assert_eq!(chip.fidelity(), level);
        }

        // Verify downgrade works
        chip.set_fidelity(SimulationFidelity::Behavioral);
        assert_eq!(chip.fidelity(), SimulationFidelity::Behavioral);
    }

    #[test]
    fn pin_map_returns_expected_mappings() {
        let chip = MockChip {
            fidelity: SimulationFidelity::Behavioral,
        };

        let pins = chip.pin_map();
        assert_eq!(pins.len(), 4);

        assert_eq!(pins[0].name, "D0");
        assert_eq!(pins[0].direction, PinDirection::Bidirectional);

        assert_eq!(pins[2].name, "SYNC");
        assert_eq!(pins[2].direction, PinDirection::Output);

        assert_eq!(pins[3].name, "CLK1");
        assert_eq!(pins[3].direction, PinDirection::Input);
    }

    #[test]
    fn multiple_subcircuits() {
        let chip = MockChip {
            fidelity: SimulationFidelity::Behavioral,
        };

        let names = chip.subcircuit_names();
        assert_eq!(names.len(), 3);

        for name in &names {
            assert!(chip.subcircuit(name).is_some(), "subcircuit '{}' should exist", name);
        }
    }
}
