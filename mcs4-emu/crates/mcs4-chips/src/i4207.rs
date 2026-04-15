//! Intel 4207 Single-Phase Clock Generator
//!
//! The 4207 generates a single-phase clock output from a crystal oscillator.
//! It is designed to drive the 4209 single-to-two-phase converter, which
//! produces the two-phase non-overlapping clocks needed by the 4040.
//!
//! The 4207 is the simplest clock source: crystal in, single clock out.

use mcs4_bus::BusCycle;
use mcs4_core::Time;

/// Intel 4207: Single-Phase Crystal Clock Generator
#[derive(Clone, Debug)]
pub struct I4207 {
    /// Crystal oscillator frequency (Hz)
    frequency_hz: u64,

    /// Period in picoseconds
    period_ps: Time,

    /// Current clock state
    clock_high: bool,

    /// Time accumulator for edge generation
    time_accum: Time,

    /// Half-period for 50% duty cycle
    half_period_ps: Time,
}

impl I4207 {
    /// Create a 4207 with the specified crystal frequency.
    pub fn new(frequency_hz: u64) -> Self {
        let period_ps = 1_000_000_000_000u64.checked_div(frequency_hz).unwrap_or(1_000_000); // default 1MHz equivalent
        Self {
            frequency_hz,
            period_ps,
            clock_high: false,
            time_accum: 0,
            half_period_ps: period_ps / 2,
        }
    }

    /// Default: 5.185 MHz crystal (yields ~740 kHz after divide-by-7 in 4201)
    pub fn default_crystal() -> Self {
        Self::new(5_185_000)
    }

    /// Step one time increment (ps). Returns true on clock edges.
    pub fn step(&mut self, time: Time) -> bool {
        let prev = self.clock_high;
        if time >= self.time_accum + self.half_period_ps {
            self.clock_high = !self.clock_high;
            self.time_accum = time;
        }
        self.clock_high != prev
    }

    /// Current clock output state
    pub fn clock_out(&self) -> bool {
        self.clock_high
    }

    /// Oscillator frequency
    pub fn frequency(&self) -> u64 {
        self.frequency_hz
    }

    /// Clock period in picoseconds
    pub fn period(&self) -> Time {
        self.period_ps
    }
}

impl Default for I4207 {
    fn default() -> Self {
        Self::default_crystal()
    }
}

impl super::Chip for I4207 {
    fn name(&self) -> &'static str {
        "4207"
    }

    fn reset(&mut self) {
        self.clock_high = false;
        self.time_accum = 0;
    }

    fn tick(&mut self, _phase: BusCycle) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_crystal_frequency() {
        let clk = I4207::default_crystal();
        assert_eq!(clk.frequency(), 5_185_000);
    }

    #[test]
    fn starts_low() {
        let clk = I4207::new(1_000_000);
        assert!(!clk.clock_out());
    }

    #[test]
    fn produces_edges() {
        let mut clk = I4207::new(1_000_000); // 1 MHz = 1000000 ps period
        let mut edges = 0;
        for i in 0..10_000 {
            if clk.step(i * 100) {
                edges += 1;
            }
        }
        assert!(edges > 0, "no edges produced");
    }

    #[test]
    fn clock_toggles() {
        let mut clk = I4207::new(1_000_000);
        let period_ps: u64 = 1_000_000; // 1 MHz
        let half = period_ps / 2;

        // Step past first half-period
        clk.step(0);
        let initial = clk.clock_out();
        clk.step(half + 1);
        assert_ne!(clk.clock_out(), initial);
    }

    #[test]
    fn reset_returns_to_low() {
        use crate::Chip;
        let mut clk = I4207::new(1_000_000);
        clk.step(600_000); // past half period
        clk.reset();
        assert!(!clk.clock_out());
    }

    #[test]
    fn chip_trait_name() {
        use crate::Chip;
        let clk = I4207::new(1_000_000);
        assert_eq!(clk.name(), "4207");
    }
}
