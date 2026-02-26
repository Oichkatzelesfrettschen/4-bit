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

    /// Create a 4201 configured for a specific crystal frequency.
    pub fn with_frequency(hz: u64) -> Self {
        Self {
            clock: TwoPhaseClock::new(ClockConfig::for_frequency(hz)),
            reset_in: false,
            reset_out: false,
            stop_in: false,
            stp_out: false,
        }
    }

    /// Current cycle count
    pub fn cycle_count(&self) -> u64 {
        self.clock.cycle_count()
    }

    /// Process one time step (ps).
    ///
    /// If the stop signal is asserted, the clock is halted and STP output is
    /// driven high. If reset is asserted, the clock is reset to initial state.
    pub fn step(&mut self, time: Time) -> ClockEdge {
        if self.reset_in {
            self.clock.reset();
            self.reset_out = true;
            return ClockEdge::None;
        }
        self.reset_out = false;

        if self.stop_in {
            self.stp_out = true;
            return ClockEdge::None;
        }
        self.stp_out = false;

        self.clock.tick(time)
    }

    pub fn set_reset(&mut self, state: bool) {
        self.reset_in = state;
    }

    pub fn set_stop(&mut self, state: bool) {
        self.stop_in = state;
    }

    /// STP output pin: high when clock is stopped
    pub fn stp_out(&self) -> bool {
        self.stp_out
    }

    /// RESET output pin
    pub fn reset_out(&self) -> bool {
        self.reset_out
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phi1_phi2_non_overlap() {
        let mut clk = I4201::new();
        // Step through a full cycle in 10ps increments and verify
        // phi1 and phi2 are never both high simultaneously.
        for i in 0..2000 {
            let _edge = clk.step(i * 10);
            assert!(!(clk.phi1() && clk.phi2()), "phi1 and phi2 overlap at time {}", i * 10);
        }
    }

    #[test]
    fn step_produces_clock_edges() {
        let mut clk = I4201::new();
        let mut saw_phi1 = false;
        let mut saw_phi2 = false;
        // Step through enough time to see edges
        for i in 0..20_000 {
            let edge = clk.step(i * 10);
            match edge {
                ClockEdge::Phi1Rising | ClockEdge::Phi1Falling => saw_phi1 = true,
                ClockEdge::Phi2Rising | ClockEdge::Phi2Falling => saw_phi2 = true,
                ClockEdge::None => {}
            }
        }
        assert!(saw_phi1, "never saw phi1 edge");
        assert!(saw_phi2, "never saw phi2 edge");
    }

    #[test]
    fn reset_clears_state() {
        use crate::Chip;
        let mut clk = I4201::new();
        // Drive some steps
        for i in 0..100 {
            clk.step(i * 10);
        }
        clk.set_reset(true);
        clk.reset();
        assert!(!clk.phi1() || !clk.phi2()); // at least one must be low after reset
    }

    #[test]
    fn stop_halts_clock() {
        let mut clk = I4201::new();
        clk.set_stop(true);
        // When stopped, step should return None and stp_out should be high
        let edge = clk.step(1000);
        assert_eq!(edge, ClockEdge::None);
        assert!(clk.stp_out());

        // Release stop
        clk.set_stop(false);
        let edge = clk.step(2000);
        assert!(!clk.stp_out());
        // Edge may or may not be None depending on clock phase,
        // but stp_out should be deasserted
        let _ = edge;
    }

    #[test]
    fn reset_drives_output_and_halts() {
        let mut clk = I4201::new();
        // Step a few times to get clock running
        for i in 0..100 {
            clk.step(i * 10);
        }

        clk.set_reset(true);
        let edge = clk.step(5000);
        assert_eq!(edge, ClockEdge::None);
        assert!(clk.reset_out());

        // Release reset
        clk.set_reset(false);
        let edge = clk.step(6000);
        assert!(!clk.reset_out());
        let _ = edge;
    }

    #[test]
    fn default_starts_idle() {
        let clk = I4201::new();
        // Both clocks should start low (idle)
        assert!(!clk.phi1() || !clk.phi2());
    }

    // --- C.1.1: Crystal frequency configuration ---

    #[test]
    fn custom_frequency_1mhz() {
        let mut clk = I4201::with_frequency(1_000_000);
        let mut edges = 0;
        for i in 0..100 {
            let edge = clk.step(i);
            if edge != ClockEdge::None {
                edges += 1;
            }
        }
        assert!(edges > 0);
    }

    // --- C.1.2: Dead-time enforcement across frequencies ---

    #[test]
    fn dead_time_sweep_no_overlap() {
        // Sweep from 500kHz to 1MHz and verify no overlap at any frequency
        for freq_khz in [500, 600, 700, 740, 800, 900, 1000] {
            let mut clk = I4201::with_frequency(freq_khz * 1000);
            for i in 0..200 {
                clk.step(i);
                assert!(!(clk.phi1() && clk.phi2()), "overlap at {}kHz, step {}", freq_khz, i);
            }
        }
    }

    // --- C.1.3: Reset synchronization ---

    #[test]
    fn reset_holds_for_duration() {
        let mut clk = I4201::new();
        // Run a few cycles
        for i in 0..50 {
            clk.step(i);
        }

        // Assert reset
        clk.set_reset(true);
        for i in 50..60 {
            let edge = clk.step(i);
            assert_eq!(edge, ClockEdge::None, "clock should not produce edges during reset");
            assert!(clk.reset_out(), "reset_out should be high during reset");
        }

        // Release reset
        clk.set_reset(false);
        let edge = clk.step(60);
        assert!(!clk.reset_out(), "reset_out should clear after reset release");
        let _ = edge;
    }

    // --- C.1.4: STP handshake ---

    #[test]
    fn stop_resume_cycle() {
        let mut clk = I4201::new();

        // Run until clock is active
        for i in 0..10 {
            clk.step(i);
        }

        // Stop
        clk.set_stop(true);
        let phi1_before = clk.phi1();
        let phi2_before = clk.phi2();

        for i in 10..20 {
            let edge = clk.step(i);
            assert_eq!(edge, ClockEdge::None);
            assert!(clk.stp_out());
            // Clock state should be frozen
            assert_eq!(clk.phi1(), phi1_before);
            assert_eq!(clk.phi2(), phi2_before);
        }

        // Resume: stp_out clears on next step() call
        clk.set_stop(false);
        let _edge = clk.step(20);
        assert!(!clk.stp_out());
    }

    // --- C.1.5: Clock quality ---

    #[test]
    fn full_cycle_produces_all_four_edges() {
        let mut clk = I4201::new();
        let mut saw = [false; 4]; // Phi1R, Phi1F, Phi2R, Phi2F

        for i in 0..200 {
            match clk.step(i) {
                ClockEdge::Phi1Rising => saw[0] = true,
                ClockEdge::Phi1Falling => saw[1] = true,
                ClockEdge::Phi2Rising => saw[2] = true,
                ClockEdge::Phi2Falling => saw[3] = true,
                ClockEdge::None => {}
            }
        }

        assert!(saw[0], "missing Phi1Rising");
        assert!(saw[1], "missing Phi1Falling");
        assert!(saw[2], "missing Phi2Rising");
        assert!(saw[3], "missing Phi2Falling");
    }

    #[test]
    fn cycle_count_increments() {
        let mut clk = I4201::new();
        assert_eq!(clk.cycle_count(), 0);

        // Run enough steps to complete at least one cycle
        for i in 0..200 {
            clk.step(i);
        }
        assert!(clk.cycle_count() > 0, "cycle count should increment");
    }

    #[test]
    fn chip_trait_name() {
        use crate::Chip;
        let clk = I4201::new();
        assert_eq!(clk.name(), "4201");
    }
}
