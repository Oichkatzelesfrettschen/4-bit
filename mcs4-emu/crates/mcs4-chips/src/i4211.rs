//! Legacy clock compatibility facsimile named I4211.
//!
//! Intel catalogs identify the 4211 as general-purpose I/O, not a clock
//! generator. This retained module preserves a preexisting repository API and
//! is quarantined from source-bound historical profiles.
//!
//! This facsimile is a self-contained clock generator that uses an external RC
//! network to set the oscillation frequency and internally generates
//! two-phase non-overlapping clocks (PHI1, PHI2). It combines the functions
//! of both the 4207 and 4209 into a single chip.
//!
//! Frequency is determined by: f ~ 1 / (2.2 * R * C)

use mcs4_bus::BusCycle;
use mcs4_core::Time;

/// Legacy repository clock facsimile named I4211.
#[derive(Clone, Debug)]
pub struct I4211 {
    /// Oscillator frequency determined by external RC (Hz)
    frequency_hz: u64,

    /// Period in picoseconds
    period_ps: Time,

    /// PHI1 output
    phi1: bool,

    /// PHI2 output
    phi2: bool,

    /// Phase state machine
    phase_counter: u8,

    /// Time accumulator
    time_accum: Time,

    /// Quarter-period for phase generation (period/4 gives symmetric phases)
    quarter_period_ps: Time,
}

impl I4211 {
    /// Create with specified RC-determined frequency.
    pub fn new(frequency_hz: u64) -> Self {
        let period_ps = 1_000_000_000_000u64.checked_div(frequency_hz).unwrap_or(1_000_000);
        Self {
            frequency_hz,
            period_ps,
            phi1: false,
            phi2: false,
            // Start at 3 so first transition produces phase 0 (phi1 high)
            phase_counter: 3,
            time_accum: 0,
            quarter_period_ps: period_ps / 4,
        }
    }

    /// Default: ~740 kHz (typical MCS-40 clock rate)
    pub fn default_frequency() -> Self {
        Self::new(740_000)
    }

    /// Step the clock generator. Produces PHI1/PHI2 with non-overlap.
    ///
    /// Phase sequence per period:
    /// - 0: PHI1 high, PHI2 low (first quarter)
    /// - 1: PHI1 low, PHI2 low (dead time)
    /// - 2: PHI1 low, PHI2 high (third quarter)
    /// - 3: PHI1 low, PHI2 low (dead time)
    pub fn step(&mut self, time: Time) {
        if time >= self.time_accum + self.quarter_period_ps {
            self.phase_counter = (self.phase_counter + 1) % 4;
            self.time_accum = time;

            match self.phase_counter {
                0 => {
                    self.phi1 = true;
                    self.phi2 = false;
                }
                1 => {
                    self.phi1 = false;
                    self.phi2 = false;
                }
                2 => {
                    self.phi1 = false;
                    self.phi2 = true;
                }
                // phase_counter is always mod-4; the final arm is the
                // second dead-time quarter (counter == 3).
                _ => {
                    self.phi1 = false;
                    self.phi2 = false;
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

    pub fn frequency(&self) -> u64 {
        self.frequency_hz
    }

    /// Clock period in picoseconds
    pub fn period(&self) -> Time {
        self.period_ps
    }
}

impl Default for I4211 {
    fn default() -> Self {
        Self::default_frequency()
    }
}

impl super::Chip for I4211 {
    fn name(&self) -> &'static str {
        "4211"
    }

    fn reset(&mut self) {
        self.phi1 = false;
        self.phi2 = false;
        self.phase_counter = 3;
        self.time_accum = 0;
    }

    fn tick(&mut self, _phase: BusCycle) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phi1_phi2_never_overlap() {
        let mut clk = I4211::new(1_000_000);
        for i in 0..10_000u64 {
            clk.step(i * 10);
            assert!(!(clk.phi1() && clk.phi2()), "overlap at time {}", i * 10);
        }
    }

    #[test]
    fn produces_both_phases() {
        let mut clk = I4211::new(1_000_000);
        let mut saw_phi1 = false;
        let mut saw_phi2 = false;

        for i in 0..100_000u64 {
            clk.step(i * 10);
            if clk.phi1() {
                saw_phi1 = true;
            }
            if clk.phi2() {
                saw_phi2 = true;
            }
        }
        assert!(saw_phi1, "never saw phi1");
        assert!(saw_phi2, "never saw phi2");
    }

    #[test]
    fn starts_with_both_low() {
        let clk = I4211::default_frequency();
        assert!(!clk.phi1());
        assert!(!clk.phi2());
    }

    #[test]
    fn default_frequency_is_740khz() {
        let clk = I4211::default_frequency();
        assert_eq!(clk.frequency(), 740_000);
    }

    #[test]
    fn reset_clears_state() {
        use crate::Chip;
        let mut clk = I4211::new(1_000_000);
        for i in 0..1000u64 {
            clk.step(i * 10);
        }
        clk.reset();
        assert!(!clk.phi1());
        assert!(!clk.phi2());
    }

    #[test]
    fn chip_trait_name() {
        use crate::Chip;
        let clk = I4211::new(1_000_000);
        assert_eq!(clk.name(), "4211");
    }
}
