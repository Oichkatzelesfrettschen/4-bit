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
/// - `Behavioral`: ISA-level functional emulation (fastest).
/// - `PhaseAccurate`: State transitions locked to clock phases.
/// - `SwitchLevel`: Iterative switch-level transistor simulation.
/// - `NodalLevel`: SPICE-class analog nodal analysis (MNA).
/// - `TCADLevel`: Physics-driven simulation derived from semiconductor equations (most accurate).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SimulationFidelity {
    /// Functional simulation only. Instructions execute with correct
    /// logical behavior but no analog or timing modeling.
    #[default]
    Behavioral,

    /// Phase-accurate simulation. Execution is locked to PHI1/PHI2
    /// clock phases, but internal logic is still behavioral.
    PhaseAccurate,

    /// Switch-level simulation. Transistors modeled as ideal switches
    /// with basic RC delay estimations.
    SwitchLevel,

    /// Analog nodal analysis. Full MNA-based solver with non-linear
    /// device models and transient integration.
    NodalLevel,

    /// TCAD-level physics simulation. Device parameters derived
    /// from solving Poisson-Boltzmann and Drift-Diffusion equations.
    TCADLevel,
}

impl SimulationFidelity {
    /// True if this level includes analog transistor-level simulation.
    pub fn is_analog(&self) -> bool {
        *self >= Self::NodalLevel
    }

    /// True if this level includes timing information (gate or RC).
    pub fn has_timing(&self) -> bool {
        *self >= Self::PhaseAccurate
    }

    /// True if this level requires physical device physics (TCAD).
    pub fn is_physics_driven(&self) -> bool {
        *self == Self::TCADLevel
    }
}

impl std::fmt::Display for SimulationFidelity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Behavioral => write!(f, "Behavioral"),
            Self::PhaseAccurate => write!(f, "Phase-Accurate"),
            Self::SwitchLevel => write!(f, "Switch-Level"),
            Self::NodalLevel => write!(f, "Nodal-Level"),
            Self::TCADLevel => write!(f, "TCAD-Level"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering() {
        assert!(SimulationFidelity::Behavioral < SimulationFidelity::PhaseAccurate);
        assert!(SimulationFidelity::PhaseAccurate < SimulationFidelity::SwitchLevel);
        assert!(SimulationFidelity::SwitchLevel < SimulationFidelity::NodalLevel);
        assert!(SimulationFidelity::NodalLevel < SimulationFidelity::TCADLevel);
    }

    #[test]
    fn is_analog() {
        assert!(!SimulationFidelity::Behavioral.is_analog());
        assert!(!SimulationFidelity::PhaseAccurate.is_analog());
        assert!(SimulationFidelity::NodalLevel.is_analog());
        assert!(SimulationFidelity::TCADLevel.is_analog());
    }

    #[test]
    fn has_timing() {
        assert!(!SimulationFidelity::Behavioral.has_timing());
        assert!(SimulationFidelity::PhaseAccurate.has_timing());
        assert!(SimulationFidelity::TCADLevel.has_timing());
    }

    #[test]
    fn default_is_behavioral() {
        assert_eq!(SimulationFidelity::default(), SimulationFidelity::Behavioral);
    }

    #[test]
    fn display() {
        assert_eq!(format!("{}", SimulationFidelity::Behavioral), "Behavioral");
        assert_eq!(format!("{}", SimulationFidelity::TCADLevel), "TCAD-Level");
    }
}
