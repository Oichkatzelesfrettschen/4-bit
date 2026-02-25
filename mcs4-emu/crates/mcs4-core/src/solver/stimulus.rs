//! Stimulus waveform generation for transient simulation.
//!
//! Provides piecewise-linear (PWL) and pulse waveforms for driving
//! circuit inputs during transient analysis. Used to generate the
//! two-phase clock (phi1/phi2) and digital input patterns for the
//! MCS-4 chip family.

/// A single point in a piecewise-linear waveform.
#[derive(Clone, Copy, Debug)]
pub struct PwlPoint {
    /// Time (seconds).
    pub time: f64,
    /// Voltage (V).
    pub voltage: f64,
}

/// Piecewise-linear voltage source.
///
/// Interpolates linearly between defined time/voltage breakpoints.
/// Before the first point, returns the first voltage.
/// After the last point, returns the last voltage.
#[derive(Clone, Debug)]
pub struct PwlSource {
    /// Breakpoints, must be sorted by time (ascending).
    points: Vec<PwlPoint>,
}

impl PwlSource {
    /// Create a PWL source from a list of (time, voltage) pairs.
    ///
    /// # Panics
    /// Panics if the list is empty or times are not monotonically increasing.
    pub fn new(pairs: &[(f64, f64)]) -> Self {
        assert!(!pairs.is_empty(), "PWL source needs at least one point");
        for w in pairs.windows(2) {
            assert!(
                w[1].0 > w[0].0,
                "PWL times must be monotonically increasing: {} >= {}",
                w[0].0,
                w[1].0
            );
        }
        let points = pairs
            .iter()
            .map(|&(time, voltage)| PwlPoint { time, voltage })
            .collect();
        Self { points }
    }

    /// Evaluate the waveform at a given time.
    pub fn voltage_at(&self, t: f64) -> f64 {
        if t <= self.points[0].time {
            return self.points[0].voltage;
        }
        let last = self.points.len() - 1;
        if t >= self.points[last].time {
            return self.points[last].voltage;
        }
        // Binary search for the interval containing t
        let idx = self.points.partition_point(|p| p.time <= t).saturating_sub(1);
        let p0 = &self.points[idx];
        let p1 = &self.points[idx + 1];
        let frac = (t - p0.time) / (p1.time - p0.time);
        p0.voltage + frac * (p1.voltage - p0.voltage)
    }

    /// Return the time of the last breakpoint.
    pub fn end_time(&self) -> f64 {
        self.points.last().map_or(0.0, |p| p.time)
    }
}

/// Periodic pulse waveform.
///
/// Generates a repeating pulse with configurable rise/fall times,
/// high/low voltages, pulse width, and period.
#[derive(Clone, Debug)]
pub struct PulseSource {
    /// Low voltage level (V).
    pub v_low: f64,
    /// High voltage level (V).
    pub v_high: f64,
    /// Delay before first pulse (s).
    pub delay: f64,
    /// Rise time (s).
    pub t_rise: f64,
    /// Fall time (s).
    pub t_fall: f64,
    /// Pulse width at high level (s).
    pub t_width: f64,
    /// Period (s).
    pub period: f64,
}

impl PulseSource {
    /// Evaluate the waveform at a given time.
    pub fn voltage_at(&self, t: f64) -> f64 {
        if t < self.delay {
            return self.v_low;
        }
        // Time within current period
        let t_rel = (t - self.delay) % self.period;

        if t_rel < self.t_rise {
            // Rising edge
            let frac = t_rel / self.t_rise;
            self.v_low + frac * (self.v_high - self.v_low)
        } else if t_rel < self.t_rise + self.t_width {
            // High plateau
            self.v_high
        } else if t_rel < self.t_rise + self.t_width + self.t_fall {
            // Falling edge
            let frac = (t_rel - self.t_rise - self.t_width) / self.t_fall;
            self.v_high + frac * (self.v_low - self.v_high)
        } else {
            // Low plateau
            self.v_low
        }
    }
}

/// A stimulus applied to a circuit node during transient simulation.
#[derive(Clone, Debug)]
pub struct Stimulus {
    /// Graph node index to drive.
    pub node_idx: usize,
    /// Waveform type.
    pub waveform: Waveform,
}

/// Supported waveform types.
#[derive(Clone, Debug)]
pub enum Waveform {
    /// Piecewise-linear.
    Pwl(PwlSource),
    /// Periodic pulse.
    Pulse(PulseSource),
    /// Constant DC.
    Dc(f64),
}

impl Stimulus {
    /// Evaluate this stimulus at a given time.
    pub fn voltage_at(&self, t: f64) -> f64 {
        match &self.waveform {
            Waveform::Pwl(pwl) => pwl.voltage_at(t),
            Waveform::Pulse(pulse) => pulse.voltage_at(t),
            Waveform::Dc(v) => *v,
        }
    }
}

/// Generate a two-phase non-overlapping clock stimulus pair (phi1, phi2).
///
/// The MCS-4 uses a two-phase clock where phi1 and phi2 are complementary
/// and non-overlapping. For the pMOS process, "active" means driven to VDD
/// (negative), and "inactive" means at VSS (0V).
///
/// # Arguments
/// * `phi1_node` - graph node index for phi1
/// * `phi2_node` - graph node index for phi2
/// * `vdd` - supply voltage (e.g., -15.0 V)
/// * `period` - clock period (s)
/// * `t_rise` - rise/fall time (s)
/// * `dead_time` - non-overlap dead time between phases (s)
pub fn mcs4_clock(
    phi1_node: usize,
    phi2_node: usize,
    vdd: f64,
    period: f64,
    t_rise: f64,
    dead_time: f64,
) -> (Stimulus, Stimulus) {
    // Each phase is active for slightly less than half the period
    let half = period / 2.0;
    let t_width = half - 2.0 * dead_time - t_rise;
    assert!(
        t_width > 0.0,
        "Clock period too short for given rise time and dead time"
    );

    let phi1 = Stimulus {
        node_idx: phi1_node,
        waveform: Waveform::Pulse(PulseSource {
            v_low: 0.0,
            v_high: vdd,
            delay: 0.0,
            t_rise,
            t_fall: t_rise,
            t_width,
            period,
        }),
    };

    let phi2 = Stimulus {
        node_idx: phi2_node,
        waveform: Waveform::Pulse(PulseSource {
            v_low: 0.0,
            v_high: vdd,
            delay: half,
            t_rise,
            t_fall: t_rise,
            t_width,
            period,
        }),
    };

    (phi1, phi2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pwl_constant() {
        let pwl = PwlSource::new(&[(0.0, 5.0), (1.0, 5.0)]);
        assert!((pwl.voltage_at(0.0) - 5.0).abs() < 1e-10);
        assert!((pwl.voltage_at(0.5) - 5.0).abs() < 1e-10);
        assert!((pwl.voltage_at(1.0) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn pwl_linear_ramp() {
        let pwl = PwlSource::new(&[(0.0, 0.0), (1.0, 10.0)]);
        assert!((pwl.voltage_at(0.0) - 0.0).abs() < 1e-10);
        assert!((pwl.voltage_at(0.5) - 5.0).abs() < 1e-10);
        assert!((pwl.voltage_at(1.0) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn pwl_before_and_after() {
        let pwl = PwlSource::new(&[(1.0, 3.0), (2.0, 7.0)]);
        // Before first point: hold first value
        assert!((pwl.voltage_at(0.0) - 3.0).abs() < 1e-10);
        // After last point: hold last value
        assert!((pwl.voltage_at(5.0) - 7.0).abs() < 1e-10);
    }

    #[test]
    fn pwl_multi_segment() {
        let pwl = PwlSource::new(&[(0.0, 0.0), (1.0, 5.0), (2.0, 5.0), (3.0, 0.0)]);
        assert!((pwl.voltage_at(0.5) - 2.5).abs() < 1e-10);
        assert!((pwl.voltage_at(1.5) - 5.0).abs() < 1e-10);
        assert!((pwl.voltage_at(2.5) - 2.5).abs() < 1e-10);
    }

    #[test]
    fn pwl_end_time() {
        let pwl = PwlSource::new(&[(0.0, 0.0), (5e-6, 10.0)]);
        assert!((pwl.end_time() - 5e-6).abs() < 1e-20);
    }

    #[test]
    fn pulse_basic_waveform() {
        let pulse = PulseSource {
            v_low: 0.0,
            v_high: 5.0,
            delay: 0.0,
            t_rise: 1.0,
            t_fall: 1.0,
            t_width: 3.0,
            period: 10.0,
        };

        // During rise
        assert!((pulse.voltage_at(0.5) - 2.5).abs() < 1e-10);
        // At high plateau
        assert!((pulse.voltage_at(2.0) - 5.0).abs() < 1e-10);
        // During fall
        assert!((pulse.voltage_at(4.5) - 2.5).abs() < 1e-10);
        // At low plateau
        assert!((pulse.voltage_at(7.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn pulse_with_delay() {
        let pulse = PulseSource {
            v_low: 0.0,
            v_high: -15.0,
            delay: 1e-6,
            t_rise: 0.1e-6,
            t_fall: 0.1e-6,
            t_width: 0.5e-6,
            period: 1.35e-6,
        };

        // Before delay: low
        assert!((pulse.voltage_at(0.0) - 0.0).abs() < 1e-10);
        // After delay, during high plateau
        assert!((pulse.voltage_at(1.2e-6) - (-15.0)).abs() < 1e-10);
    }

    #[test]
    fn pulse_periodic() {
        let pulse = PulseSource {
            v_low: 0.0,
            v_high: 5.0,
            delay: 0.0,
            t_rise: 0.0001,
            t_fall: 0.0001,
            t_width: 1.0,
            period: 2.0,
        };

        // First period high
        let v1 = pulse.voltage_at(0.5);
        // Second period high (should repeat)
        let v2 = pulse.voltage_at(2.5);
        assert!((v1 - v2).abs() < 1e-10, "Pulse should be periodic");
    }

    #[test]
    fn stimulus_dc() {
        let stim = Stimulus {
            node_idx: 0,
            waveform: Waveform::Dc(-15.0),
        };
        assert!((stim.voltage_at(0.0) - (-15.0)).abs() < 1e-10);
        assert!((stim.voltage_at(1e6) - (-15.0)).abs() < 1e-10);
    }

    #[test]
    fn mcs4_clock_non_overlapping() {
        let period = 1.35e-6; // ~740 kHz
        let t_rise = 10e-9;
        let dead_time = 20e-9;

        let (phi1, phi2) = mcs4_clock(0, 1, -15.0, period, t_rise, dead_time);

        // Sample at many points and verify non-overlap
        let steps = 1000;
        for i in 0..steps {
            let t = period * (i as f64) / (steps as f64);
            let v1 = phi1.voltage_at(t);
            let v2 = phi2.voltage_at(t);

            // Both should not be simultaneously at VDD (both negative)
            let threshold = -7.5; // midpoint
            let phi1_active = v1 < threshold;
            let phi2_active = v2 < threshold;
            assert!(
                !(phi1_active && phi2_active),
                "Clock overlap at t={:.3e}: phi1={:.1}V, phi2={:.1}V",
                t,
                v1,
                v2
            );
        }
    }

    #[test]
    fn mcs4_clock_both_active_sometime() {
        let period = 1.35e-6;
        let t_rise = 10e-9;
        let dead_time = 20e-9;

        let (phi1, phi2) = mcs4_clock(0, 1, -15.0, period, t_rise, dead_time);

        let threshold = -7.5;
        let mut phi1_ever_active = false;
        let mut phi2_ever_active = false;

        let steps = 1000;
        for i in 0..steps {
            let t = period * (i as f64) / (steps as f64);
            if phi1.voltage_at(t) < threshold {
                phi1_ever_active = true;
            }
            if phi2.voltage_at(t) < threshold {
                phi2_ever_active = true;
            }
        }

        assert!(phi1_ever_active, "phi1 should be active during the period");
        assert!(phi2_ever_active, "phi2 should be active during the period");
    }
}
