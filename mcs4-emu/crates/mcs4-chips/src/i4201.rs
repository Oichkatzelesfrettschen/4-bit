//! Intel 4201 Clock Generator
//!
//! The 4201 provides the two-phase non-overlapping clock (PHI1, PHI2)
//! for the 4004/4040. It also handles RESET and STOP/STP logic.

use mcs4_bus::prelude::*;
use mcs4_core::Time;

/// Intel 4201 Clock Generator
#[derive(Clone, Debug)]
pub struct I4201 {
    /// Internal clock logic (delegated to bus crate for consistency)
    clock: TwoPhaseClock,

    /// Reset signal (Active High for 4040, but 4201 can generate either)
    reset_in: bool,
    reset_out: bool,

    /// Stop control (for 4040)
    stop_in: bool,
    stp_out: bool,
}

impl I4201 {
    pub fn new() -> Self {
        Self {
            clock: TwoPhaseClock::default_config(),
            reset_in: false,
            reset_out: false,
            stop_in: false,
            stp_out: false,
        }
    }

    /// Process one time step (ps)
    pub fn step(&mut self, time: Time) -> ClockEdge {
        self.clock.tick(time)
    }

    pub fn set_reset(&mut self, state: bool) {
        self.reset_in = state;
        self.reset_out = state; // Simple behavioral pass-through
    }

    pub fn set_stop(&mut self, state: bool) {
        self.stop_in = state;
    }

    pub fn phi1(&self) -> bool {
        self.clock.phi1_high()
    }
    pub fn phi2(&self) -> bool {
        self.clock.phi2_high()
    }
}

impl Default for I4201 {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Chip for I4201 {
    fn name(&self) -> &'static str {
        "4201"
    }
    fn reset(&mut self) {
        self.clock.reset();
        self.reset_out = false;
        self.stp_out = false;
    }
    fn tick(&mut self, _phase: BusCycle) {
        // Ticking the clock is handled by the System driver or step()
    }
}
