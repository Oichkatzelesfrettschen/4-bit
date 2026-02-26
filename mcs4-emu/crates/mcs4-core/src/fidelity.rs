//! Simulation fidelity levels for multi-resolution chip modeling.
//!
//! WHY: Different use cases need different simulation accuracy. Behavioral
//! simulation runs fast for functional testing; gate-level adds timing;
//! transistor-level provides full analog accuracy at the cost of speed.
//!
//! WHAT: A three-level enum controlling how deeply the solver models
//! each chip's internal circuitry.
//!
//! HOW: Chips implementing `ChipSolverBridge` report their available
//! fidelity levels and return subcircuit graphs only at transistor level.

/// Simulation fidelity level controlling model depth.
///
/// Each level includes the capabilities of lower levels:
/// - `Behavioral` runs fastest (no analog simulation)
/// - `GateLevel` adds timing and fanout modeling
/// - `TransistorLevel` provides full SPICE-class accuracy
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SimulationFidelity {
    /// Functional simulation only. Instructions execute with correct
    /// logical behavior but no analog or timing modeling. This is the
    /// default for differential testing and ROM validation.
    #[default]
    Behavioral,

    /// Gate-level simulation with propagation delays and fanout.
    /// Uses the event-driven simulator with calibrated gate delays
    /// derived from transistor-level characterization.
    GateLevel,

    /// Full transistor-level SPICE-class simulation. Each subcircuit
    /// is represented as a `CircuitGraph` and solved with the DC/transient
    /// solvers. Slowest but most accurate; required for analog validation
    /// and process corner analysis.
    TransistorLevel,
}

impl SimulationFidelity {
    /// True if this level includes analog transistor-level simulation.
    pub fn is_analog(&self) -> bool {
        *self == Self::TransistorLevel
    }

    /// True if this level includes timing information.
    pub fn has_timing(&self) -> bool {
        *self >= Self::GateLevel
    }
}

impl std::fmt::Display for SimulationFidelity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Behavioral => write!(f, "Behavioral"),
            Self::GateLevel => write!(f, "Gate-Level"),
            Self::TransistorLevel => write!(f, "Transistor-Level"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering() {
        assert!(SimulationFidelity::Behavioral < SimulationFidelity::GateLevel);
        assert!(SimulationFidelity::GateLevel < SimulationFidelity::TransistorLevel);
    }

    #[test]
    fn is_analog() {
        assert!(!SimulationFidelity::Behavioral.is_analog());
        assert!(!SimulationFidelity::GateLevel.is_analog());
        assert!(SimulationFidelity::TransistorLevel.is_analog());
    }

    #[test]
    fn has_timing() {
        assert!(!SimulationFidelity::Behavioral.has_timing());
        assert!(SimulationFidelity::GateLevel.has_timing());
        assert!(SimulationFidelity::TransistorLevel.has_timing());
    }

    #[test]
    fn default_is_behavioral() {
        assert_eq!(SimulationFidelity::default(), SimulationFidelity::Behavioral);
    }

    #[test]
    fn display() {
        assert_eq!(format!("{}", SimulationFidelity::Behavioral), "Behavioral");
        assert_eq!(format!("{}", SimulationFidelity::GateLevel), "Gate-Level");
        assert_eq!(format!("{}", SimulationFidelity::TransistorLevel), "Transistor-Level");
    }
}
