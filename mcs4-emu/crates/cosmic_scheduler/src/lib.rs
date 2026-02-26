//! Cosmic Scheduler
//!
//! Provides abstract two-phase clock ($\phi_1$ / $\phi_2$) scheduling and execution mechanisms.
//! This bridges the discrete timing of silicon circuits (like the Intel 4004) with the
//! topological timestep iterations of lattice physics models (like Lattice Boltzmann Methods).

use std::fmt::Debug;

/// Represents a distinct temporal phase in the Cosmic Engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CosmicPhase {
    /// Phase 1: Interaction, collision, or logic gating.
    /// In MCS-4, this corresponds to $\phi_1$.
    /// In LBM, this corresponds to the local collision operator (BGK).
    Phi1,
    /// Phase 2: Propagation, streaming, or latching.
    /// In MCS-4, this corresponds to $\phi_2$.
    /// In LBM, this corresponds to the spatial streaming step.
    Phi2,
}

impl CosmicPhase {
    /// Returns the alternating phase.
    pub fn next(&self) -> Self {
        match self {
            Self::Phi1 => Self::Phi2,
            Self::Phi2 => Self::Phi1,
        }
    }
}

/// A trait defining any deterministic system that evolves via a strict two-phase clock.
pub trait PhaseScheduler {
    /// The state type being mutated during the phase cycle.
    type State;
    /// An error type for invalid transitions.
    type Error: Debug;

    /// Execute the interaction phase ($\phi_1$).
    /// For the 4-bit CPU, this evaluates logic gates.
    /// For physics, this resolves local collisions.
    fn execute_phi1(&self, state: &mut Self::State) -> Result<(), Self::Error>;

    /// Execute the propagation phase ($\phi_2$).
    /// For the 4-bit CPU, this latches the results into sequential logic.
    /// For physics, this streams quantities across the lattice.
    fn execute_phi2(&self, state: &mut Self::State) -> Result<(), Self::Error>;

    /// Advance the system by one full clock cycle ($\phi_1$ followed by $\phi_2$).
    fn tick(&self, state: &mut Self::State) -> Result<(), Self::Error> {
        self.execute_phi1(state)?;
        self.execute_phi2(state)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockLattice;
    struct MockScheduler;

    impl PhaseScheduler for MockScheduler {
        type State = MockLattice;
        type Error = ();

        fn execute_phi1(&self, _state: &mut Self::State) -> Result<(), Self::Error> {
            Ok(())
        }

        fn execute_phi2(&self, _state: &mut Self::State) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn test_phase_alternation() {
        assert_eq!(CosmicPhase::Phi1.next(), CosmicPhase::Phi2);
        assert_eq!(CosmicPhase::Phi2.next(), CosmicPhase::Phi1);
    }

    #[test]
    fn test_tick_cycle() {
        let scheduler = MockScheduler;
        let mut state = MockLattice;
        assert!(scheduler.tick(&mut state).is_ok());
    }
}
