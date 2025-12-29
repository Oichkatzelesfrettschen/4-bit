//! Two-phase non-overlapping clock generation
//!
//! The MCS-4 uses a two-phase clock with PHI1 and PHI2 that must not overlap.
//! Typical clock frequency is 740 kHz (1.35 us period).

use mcs4_core::prelude::*;

/// Clock configuration parameters
#[derive(Clone, Debug)]
pub struct ClockConfig {
    /// Clock period (PHI1 rising to next PHI1 rising)
    pub period: Time,

    /// PHI1 pulse width
    pub phi1_width: Time,

    /// PHI2 pulse width
    pub phi2_width: Time,

    /// Delay from PHI1 falling to PHI2 rising
    pub phi1_to_phi2_delay: Time,

    /// Delay from PHI2 falling to PHI1 rising
    pub phi2_to_phi1_delay: Time,

    /// Clock rise time
    pub rise_time: Time,

    /// Clock fall time
    pub fall_time: Time,
}

impl Default for ClockConfig {
    fn default() -> Self {
        // Typical 740 kHz clock from datasheet
        Self {
            period: clock_spec::TCY_TYP,
            phi1_width: clock_spec::T0PW_MIN,
            phi2_width: clock_spec::T0PW_MIN,
            phi1_to_phi2_delay: clock_spec::T0D1_MIN,
            phi2_to_phi1_delay: clock_spec::T0D2_MIN,
            rise_time: clock_spec::T0R,
            fall_time: clock_spec::T0F,
        }
    }
}

impl ClockConfig {
    /// Create a clock configuration for a specific frequency
    pub fn for_frequency(hz: u64) -> Self {
        let period = 1_000_000_000_000 / hz; // Period in ps

        Self {
            period,
            phi1_width: period / 3,
            phi2_width: period / 3,
            phi1_to_phi2_delay: period / 6,
            phi2_to_phi1_delay: period / 6,
            ..Default::default()
        }
    }

    /// Create a 740 kHz clock (typical 4004)
    pub fn mcs4_typical() -> Self {
        Self::for_frequency(740_000)
    }

    /// Create a 500 kHz clock (slow 4004)
    pub fn mcs4_slow() -> Self {
        Self::for_frequency(500_000)
    }
}

/// Two-phase clock generator
#[derive(Clone, Debug)]
pub struct TwoPhaseClockTwoPhaseClock {
    /// Configuration
    pub config: ClockConfig,

    /// PHI1 signal
    pub phi1: Signal,

    /// PHI2 signal
    pub phi2: Signal,

    /// Current time within the clock period
    phase_time: Time,

    /// Total cycles generated
    cycle_count: u64,
}

impl TwoPhaseClockTwoPhaseClock {
    /// Create a new clock generator
    pub fn new(config: ClockConfig) -> Self {
        Self {
            config,
            phi1: Signal::new("PHI1", SignalLevel::Low),
            phi2: Signal::new("PHI2", SignalLevel::Low),
            phase_time: 0,
            cycle_count: 0,
        }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(ClockConfig::default())
    }

    /// Get current clock cycle count
    pub fn cycle_count(&self) -> u64 {
        self.cycle_count
    }

    /// Check if PHI1 is high
    pub fn phi1_high(&self) -> bool {
        self.phi1.current == SignalLevel::High
    }

    /// Check if PHI2 is high
    pub fn phi2_high(&self) -> bool {
        self.phi2.current == SignalLevel::High
    }

    /// Generate clock events for simulator
    pub fn schedule_events(&self, sim: &mut Simulator, phi1_id: SignalId, phi2_id: SignalId, start_time: Time, num_cycles: u64) {
        let mut t = start_time;

        for _ in 0..num_cycles {
            // PHI1 rising edge
            sim.schedule(t, phi1_id, SignalLevel::High, EventSource::Clock);

            // PHI1 falling edge
            t += self.config.phi1_width;
            sim.schedule(t, phi1_id, SignalLevel::Low, EventSource::Clock);

            // PHI2 rising edge (after phi1-to-phi2 delay)
            t += self.config.phi1_to_phi2_delay;
            sim.schedule(t, phi2_id, SignalLevel::High, EventSource::Clock);

            // PHI2 falling edge
            t += self.config.phi2_width;
            sim.schedule(t, phi2_id, SignalLevel::Low, EventSource::Clock);

            // Wait for phi2-to-phi1 delay before next cycle
            t += self.config.phi2_to_phi1_delay;
        }
    }

    /// Advance clock by one step (for cycle-accurate mode)
    pub fn tick(&mut self, current_time: Time) -> ClockEdge {
        let t = self.phase_time;

        // PHI1 rising
        if t == 0 {
            self.phi1.update(current_time, SignalLevel::High);
            self.phase_time += self.config.phi1_width;
            return ClockEdge::Phi1Rising;
        }

        // PHI1 falling
        if t == self.config.phi1_width {
            self.phi1.update(current_time, SignalLevel::Low);
            self.phase_time += self.config.phi1_to_phi2_delay;
            return ClockEdge::Phi1Falling;
        }

        // PHI2 rising
        let phi2_start = self.config.phi1_width + self.config.phi1_to_phi2_delay;
        if t == phi2_start {
            self.phi2.update(current_time, SignalLevel::High);
            self.phase_time += self.config.phi2_width;
            return ClockEdge::Phi2Rising;
        }

        // PHI2 falling
        let phi2_end = phi2_start + self.config.phi2_width;
        if t == phi2_end {
            self.phi2.update(current_time, SignalLevel::Low);
            self.phase_time = 0;
            self.cycle_count += 1;
            return ClockEdge::Phi2Falling;
        }

        ClockEdge::None
    }

    /// Reset clock to initial state
    pub fn reset(&mut self) {
        self.phi1 = Signal::new("PHI1", SignalLevel::Low);
        self.phi2 = Signal::new("PHI2", SignalLevel::Low);
        self.phase_time = 0;
        self.cycle_count = 0;
    }
}

/// Clock edge events
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockEdge {
    None,
    Phi1Rising,
    Phi1Falling,
    Phi2Rising,
    Phi2Falling,
}

impl ClockEdge {
    pub fn is_rising(self) -> bool {
        matches!(self, ClockEdge::Phi1Rising | ClockEdge::Phi2Rising)
    }

    pub fn is_falling(self) -> bool {
        matches!(self, ClockEdge::Phi1Falling | ClockEdge::Phi2Falling)
    }

    pub fn is_phi1(self) -> bool {
        matches!(self, ClockEdge::Phi1Rising | ClockEdge::Phi1Falling)
    }

    pub fn is_phi2(self) -> bool {
        matches!(self, ClockEdge::Phi2Rising | ClockEdge::Phi2Falling)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_config_frequency() {
        let config = ClockConfig::for_frequency(1_000_000); // 1 MHz
        assert_eq!(config.period, 1_000_000); // 1 us = 1,000,000 ps
    }

    #[test]
    fn test_clock_tick() {
        let mut clock = TwoPhaseClockTwoPhaseClock::default_config();

        // First tick should be PHI1 rising
        let edge = clock.tick(0);
        assert_eq!(edge, ClockEdge::Phi1Rising);
        assert!(clock.phi1_high());
        assert!(!clock.phi2_high());
    }

    #[test]
    fn test_non_overlapping() {
        let mut clock = TwoPhaseClockTwoPhaseClock::default_config();
        let mut t = 0;

        // Run through multiple cycles
        for _ in 0..10 {
            let edge = clock.tick(t);
            t += 1;

            // PHI1 and PHI2 should never both be high
            assert!(!(clock.phi1_high() && clock.phi2_high()),
                "Clock overlap detected at edge {:?}", edge);
        }
    }
}
