//! Legacy clock compatibility facsimile named I4209.
//!
//! Intel catalogs identify the 4209 as general-purpose I/O, not a clock
//! converter. This retained module preserves a preexisting repository API and
//! is quarantined from source-bound historical profiles.
//!
//! This facsimile takes a single-phase clock input (e.g. from the 4207) and
//! produces two-phase non-overlapping clocks (PHI1, PHI2) needed by the
//! 4040 CPU. It enforces a dead-time gap between phases to prevent
//! shoot-through in the dynamic logic.

use mcs4_bus::BusCycle;
use mcs4_core::Time;

/// Legacy repository clock facsimile named I4209.
#[derive(Clone, Debug)]
pub struct I4209 {
    /// Input clock state (from 4207 or external source)
    clock_in: bool,

    /// Previous clock input (for edge detection)
    prev_clock_in: bool,

    /// PHI1 output
    phi1: bool,

    /// PHI2 output
    phi2: bool,

    /// Dead-time enforcement state machine
    state: PhaseState,

    /// Dead-time in picoseconds (non-overlap gap)
    dead_time_ps: Time,

    /// Time of last input edge
    last_edge_time: Time,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PhaseState {
    /// Both clocks low (dead time after PHI2 falling)
    DeadTimePhi1,
    /// PHI1 high
    Phi1High,
    /// Both clocks low (dead time after PHI1 falling)
    DeadTimePhi2,
    /// PHI2 high
    Phi2High,
}

impl I4209 {
    /// Create with specified dead-time in picoseconds.
    pub fn new(dead_time_ps: Time) -> Self {
        Self {
            clock_in: false,
            prev_clock_in: false,
            phi1: false,
            phi2: false,
            state: PhaseState::DeadTimePhi1,
            dead_time_ps,
            last_edge_time: 0,
        }
    }

    /// Default: 50ns dead-time (matches 4201 spec)
    pub fn default_timing() -> Self {
        Self::new(50_000) // 50ns = 50000 ps
    }

    /// Set input clock state. Call on each edge from the 4207.
    pub fn set_clock_in(&mut self, state: bool) {
        self.prev_clock_in = self.clock_in;
        self.clock_in = state;
    }

    /// Step the converter. Call after updating clock_in with current time.
    pub fn step(&mut self, time: Time) {
        let rising = !self.prev_clock_in && self.clock_in;
        let falling = self.prev_clock_in && !self.clock_in;

        match self.state {
            PhaseState::DeadTimePhi1 => {
                if rising && time >= self.last_edge_time + self.dead_time_ps {
                    self.phi1 = true;
                    self.state = PhaseState::Phi1High;
                    self.last_edge_time = time;
                }
            }
            PhaseState::Phi1High => {
                if falling {
                    self.phi1 = false;
                    self.state = PhaseState::DeadTimePhi2;
                    self.last_edge_time = time;
                }
            }
            PhaseState::DeadTimePhi2 => {
                if rising && time >= self.last_edge_time + self.dead_time_ps {
                    self.phi2 = true;
                    self.state = PhaseState::Phi2High;
                    self.last_edge_time = time;
                }
            }
            PhaseState::Phi2High => {
                if falling {
                    self.phi2 = false;
                    self.state = PhaseState::DeadTimePhi1;
                    self.last_edge_time = time;
                }
            }
        }
    }

    pub fn phi1(&self) -> bool {
        self.phi1
    }

    pub fn phi2(&self) -> bool {
        self.phi2
    }
}

impl Default for I4209 {
    fn default() -> Self {
        Self::default_timing()
    }
}

impl super::Chip for I4209 {
    fn name(&self) -> &'static str {
        "4209"
    }

    fn reset(&mut self) {
        self.clock_in = false;
        self.prev_clock_in = false;
        self.phi1 = false;
        self.phi2 = false;
        self.state = PhaseState::DeadTimePhi1;
        self.last_edge_time = 0;
    }

    fn tick(&mut self, _phase: BusCycle) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phi1_phi2_never_overlap() {
        let mut conv = I4209::new(100); // 100ps dead-time for fast test
                                        // Simulate input clock edges
        for i in 0..200u64 {
            let clock_state = (i / 50) % 2 == 0; // toggle every 50 steps
            conv.set_clock_in(clock_state);
            conv.step(i * 100);
            assert!(
                !(conv.phi1() && conv.phi2()),
                "phi1 and phi2 overlap at time {}",
                i * 100
            );
        }
    }

    #[test]
    fn starts_with_both_low() {
        let conv = I4209::default_timing();
        assert!(!conv.phi1());
        assert!(!conv.phi2());
    }

    #[test]
    fn produces_phi1_and_phi2() {
        let mut conv = I4209::new(10); // 10ps dead-time
        let mut saw_phi1 = false;
        let mut saw_phi2 = false;

        for i in 0..10_000u64 {
            let clock = (i / 500) % 2 == 0;
            conv.set_clock_in(clock);
            conv.step(i * 10);
            if conv.phi1() {
                saw_phi1 = true;
            }
            if conv.phi2() {
                saw_phi2 = true;
            }
        }
        assert!(saw_phi1, "never saw phi1");
        assert!(saw_phi2, "never saw phi2");
    }

    #[test]
    fn reset_clears_outputs() {
        use crate::Chip;
        let mut conv = I4209::new(10);
        conv.set_clock_in(true);
        conv.step(1000);
        conv.reset();
        assert!(!conv.phi1());
        assert!(!conv.phi2());
    }

    #[test]
    fn chip_trait_name() {
        use crate::Chip;
        let conv = I4209::default_timing();
        assert_eq!(conv.name(), "4209");
    }
}
