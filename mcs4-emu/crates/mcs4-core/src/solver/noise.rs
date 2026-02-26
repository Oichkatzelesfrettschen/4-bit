//! Noise analysis for MOSFET circuits.
//!
//! Computes per-transistor noise spectral density (thermal and flicker)
//! and propagates noise through the AC small-signal transfer function
//! to produce output-referred and input-referred noise spectra.
//!
//! # Noise Sources
//!
//! - **Thermal noise**: Si_th = 4 * kT * (2/3) * gm (A^2/Hz)
//!   Frequency-independent (white noise). The 2/3 factor is the
//!   classical long-channel value.
//!
//! - **Flicker (1/f) noise**: Si_fl = Kf * Ids^Af / (Cox * W * L * f) (A^2/Hz)
//!   Inversely proportional to frequency. Dominant at low frequencies.
//!   Kf and Af are technology-dependent fitting parameters.
//!
//! # Output noise
//!
//! The output noise spectral density at node j is:
//!   Sout_j(f) = sum_i |H_ij(f)|^2 * Si_i(f)
//!
//! where H_ij(f) is the AC transfer function from noise source i to output j.

use crate::process::silicon::K_B;

/// Flicker noise fitting parameters.
#[derive(Clone, Debug)]
pub struct FlickerParams {
    /// Flicker noise coefficient Kf (A*Hz).
    pub kf: f64,
    /// Flicker noise current exponent Af (dimensionless).
    pub af: f64,
}

impl Default for FlickerParams {
    fn default() -> Self {
        Self {
            kf: 1e-24, // Typical for pMOS, technology-dependent
            af: 1.0,   // Ids^1 dependence (common model)
        }
    }
}

/// Configuration for noise analysis.
#[derive(Clone, Debug)]
pub struct NoiseConfig {
    /// Start frequency (Hz).
    pub f_start: f64,
    /// Stop frequency (Hz).
    pub f_stop: f64,
    /// Number of frequency points per decade.
    pub points_per_decade: usize,
    /// Flicker noise parameters.
    pub flicker: FlickerParams,
    /// Temperature (K).
    pub temp: f64,
}

impl Default for NoiseConfig {
    fn default() -> Self {
        Self {
            f_start: 1.0,
            f_stop: 1e8,
            points_per_decade: 10,
            flicker: FlickerParams::default(),
            temp: 300.0,
        }
    }
}

/// Per-transistor noise source data.
#[derive(Clone, Debug)]
pub struct TransistorNoise {
    /// Transistor index in the circuit graph.
    pub transistor_idx: usize,
    /// Small-signal transconductance gm (S).
    pub gm: f64,
    /// Drain current at operating point (A).
    pub ids: f64,
    /// Gate oxide capacitance per unit area (F/m^2).
    pub cox: f64,
    /// Gate width (m).
    pub w: f64,
    /// Gate length (m).
    pub l: f64,
}

impl TransistorNoise {
    /// Thermal noise current spectral density (A^2/Hz).
    ///
    /// Si_th = 4 * kT * (2/3) * gm
    pub fn thermal_sid(&self, temp: f64) -> f64 {
        4.0 * K_B * temp * (2.0 / 3.0) * self.gm
    }

    /// Flicker noise current spectral density at frequency f (A^2/Hz).
    ///
    /// Si_fl = Kf * Ids^Af / (Cox * W * L * f)
    pub fn flicker_sid(&self, f: f64, params: &FlickerParams) -> f64 {
        if f <= 0.0 || self.cox <= 0.0 || self.w <= 0.0 || self.l <= 0.0 {
            return 0.0;
        }
        params.kf * self.ids.abs().powf(params.af) / (self.cox * self.w * self.l * f)
    }

    /// Total drain current noise spectral density at frequency f (A^2/Hz).
    pub fn total_sid(&self, f: f64, temp: f64, params: &FlickerParams) -> f64 {
        self.thermal_sid(temp) + self.flicker_sid(f, params)
    }

    /// Input-referred noise voltage spectral density (V^2/Hz).
    ///
    /// Svg = Sid / gm^2
    pub fn input_referred_svn(&self, f: f64, temp: f64, params: &FlickerParams) -> f64 {
        if self.gm.abs() < 1e-30 {
            return 0.0;
        }
        self.total_sid(f, temp, params) / (self.gm * self.gm)
    }
}

/// Result of a noise analysis.
#[derive(Clone, Debug)]
pub struct NoiseResult {
    /// Frequency points (Hz).
    pub frequencies: Vec<f64>,
    /// Per-transistor noise sources.
    pub sources: Vec<TransistorNoise>,
    /// Total output noise spectral density at each frequency (V^2/Hz or A^2/Hz).
    /// Indexed by frequency position.
    pub output_noise: Vec<f64>,
    /// Total input-referred noise spectral density at each frequency (V^2/Hz).
    pub input_noise: Vec<f64>,
    /// Corner frequency where thermal = flicker (Hz). None if not found.
    pub corner_frequency: Option<f64>,
}

impl NoiseResult {
    /// Output noise in dB (10*log10(Sn/Sref)).
    pub fn output_noise_db(&self, freq_idx: usize, s_ref: f64) -> f64 {
        10.0 * (self.output_noise[freq_idx] / s_ref).log10()
    }

    /// Input-referred noise voltage spectral density in nV/sqrt(Hz).
    pub fn input_noise_nv_sqrthz(&self, freq_idx: usize) -> f64 {
        self.input_noise[freq_idx].sqrt() * 1e9
    }

    /// Integrated noise over a bandwidth (V_rms).
    ///
    /// Uses trapezoidal integration of the noise spectral density.
    pub fn integrated_noise(&self, f_low: f64, f_high: f64) -> f64 {
        let mut total = 0.0_f64;
        for i in 1..self.frequencies.len() {
            let f0 = self.frequencies[i - 1];
            let f1 = self.frequencies[i];
            if f1 < f_low || f0 > f_high {
                continue;
            }
            let fl = f0.max(f_low);
            let fh = f1.min(f_high);
            let s0 = self.output_noise[i - 1];
            let s1 = self.output_noise[i];
            // Trapezoidal rule
            total += 0.5 * (s0 + s1) * (fh - fl);
        }
        total.sqrt()
    }
}

/// Generate logarithmic frequency points.
pub fn log_freq_sweep(f_start: f64, f_stop: f64, points_per_decade: usize) -> Vec<f64> {
    let decades = (f_stop / f_start).log10();
    let n_points = (decades * points_per_decade as f64).ceil() as usize + 1;
    let log_start = f_start.log10();
    let log_stop = f_stop.log10();
    let d_log = (log_stop - log_start) / (n_points - 1).max(1) as f64;

    (0..n_points)
        .map(|i| 10.0_f64.powf(log_start + i as f64 * d_log))
        .collect()
}

/// Compute noise for a single transistor across a frequency sweep.
///
/// Returns (output_sid per frequency, corner_frequency).
pub fn analyze_single_transistor(source: &TransistorNoise, config: &NoiseConfig) -> (Vec<f64>, Vec<f64>, Option<f64>) {
    let freqs = log_freq_sweep(config.f_start, config.f_stop, config.points_per_decade);

    let output_sid: Vec<f64> = freqs
        .iter()
        .map(|&f| source.total_sid(f, config.temp, &config.flicker))
        .collect();

    let input_svn: Vec<f64> = freqs
        .iter()
        .map(|&f| source.input_referred_svn(f, config.temp, &config.flicker))
        .collect();

    // Find corner frequency: where thermal = flicker
    let s_thermal = source.thermal_sid(config.temp);
    let corner = if s_thermal > 0.0 && source.ids.abs() > 0.0 {
        // At corner: Kf * Ids^Af / (Cox * W * L * f_c) = s_thermal
        // f_c = Kf * Ids^Af / (Cox * W * L * s_thermal)
        let numerator = config.flicker.kf * source.ids.abs().powf(config.flicker.af);
        let denominator = source.cox * source.w * source.l * s_thermal;
        if denominator > 0.0 {
            Some(numerator / denominator)
        } else {
            None
        }
    } else {
        None
    };

    (output_sid, input_svn, corner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_transistor() -> TransistorNoise {
        TransistorNoise {
            transistor_idx: 0,
            gm: 1e-4,     // 100 uS
            ids: 1e-4,    // 100 uA
            cox: 4.32e-4, // F/m^2 (80nm oxide)
            w: 10e-6,     // 10um
            l: 10e-6,     // 10um
        }
    }

    #[test]
    fn thermal_noise_positive() {
        let t = test_transistor();
        let sid = t.thermal_sid(300.0);
        assert!(sid > 0.0, "thermal Sid should be > 0");
    }

    #[test]
    fn thermal_noise_proportional_to_gm() {
        let mut t1 = test_transistor();
        let mut t2 = test_transistor();
        t1.gm = 1e-4;
        t2.gm = 2e-4;
        let s1 = t1.thermal_sid(300.0);
        let s2 = t2.thermal_sid(300.0);
        assert!(
            (s2 / s1 - 2.0).abs() < 1e-6,
            "thermal noise should scale with gm: ratio = {:.3}",
            s2 / s1
        );
    }

    #[test]
    fn thermal_noise_order_of_magnitude() {
        let t = test_transistor();
        let sid = t.thermal_sid(300.0);
        // 4 * kT * 2/3 * gm ~ 4 * 4.14e-21 * 0.667 * 1e-4 ~ 1.1e-24 A^2/Hz
        assert!(
            sid > 1e-26 && sid < 1e-22,
            "thermal Sid = {:.3e}, expected 1e-26 to 1e-22",
            sid
        );
    }

    #[test]
    fn flicker_noise_inversely_proportional_to_freq() {
        let t = test_transistor();
        let params = FlickerParams::default();
        let s10 = t.flicker_sid(10.0, &params);
        let s100 = t.flicker_sid(100.0, &params);
        // 1/f: s10 should be 10x s100
        assert!(
            (s10 / s100 - 10.0).abs() < 0.1,
            "1/f scaling: ratio = {:.3}",
            s10 / s100
        );
    }

    #[test]
    fn flicker_noise_scales_with_area() {
        let mut t1 = test_transistor();
        let mut t2 = test_transistor();
        t1.w = 10e-6;
        t1.l = 10e-6;
        t2.w = 20e-6;
        t2.l = 20e-6;
        let params = FlickerParams::default();
        let s1 = t1.flicker_sid(100.0, &params);
        let s2 = t2.flicker_sid(100.0, &params);
        // 4x area -> 1/4 flicker noise
        assert!((s1 / s2 - 4.0).abs() < 0.1, "area scaling: ratio = {:.3}", s1 / s2);
    }

    #[test]
    fn total_noise_dominated_by_flicker_at_low_freq() {
        let t = test_transistor();
        let params = FlickerParams::default();
        let s_th = t.thermal_sid(300.0);
        let s_fl = t.flicker_sid(1.0, &params);
        let s_total = t.total_sid(1.0, 300.0, &params);
        assert!(
            s_fl > s_th,
            "flicker should dominate at 1Hz: flicker={:.3e}, thermal={:.3e}",
            s_fl,
            s_th
        );
        assert!((s_total - (s_th + s_fl)).abs() / s_total < 1e-6);
    }

    #[test]
    fn total_noise_dominated_by_thermal_at_high_freq() {
        let t = test_transistor();
        let params = FlickerParams::default();
        let s_th = t.thermal_sid(300.0);
        // Corner frequency for this transistor is ~2.1 GHz, so test well above that.
        let s_fl = t.flicker_sid(1e11, &params);
        assert!(
            s_th > s_fl,
            "thermal should dominate at 100GHz: thermal={:.3e}, flicker={:.3e}",
            s_th,
            s_fl
        );
    }

    #[test]
    fn input_referred_noise_positive() {
        let t = test_transistor();
        let params = FlickerParams::default();
        let svn = t.input_referred_svn(1000.0, 300.0, &params);
        assert!(svn > 0.0);
    }

    #[test]
    fn corner_frequency_found() {
        let t = test_transistor();
        let config = NoiseConfig::default();
        let (_, _, corner) = analyze_single_transistor(&t, &config);
        assert!(corner.is_some(), "should find corner frequency");
        let fc = corner.expect("corner frequency exists");
        // Corner frequency depends on Kf/(Cox*W*L*S_thermal).
        // For our test transistor (10um x 10um, Kf=1e-24): fc ~ 2.1 GHz.
        assert!(
            fc > 0.01 && fc < 1e11,
            "corner frequency = {:.3e} Hz, expected 0.01 to 1e11",
            fc
        );
    }

    #[test]
    fn log_freq_sweep_endpoints() {
        let freqs = log_freq_sweep(1.0, 1e6, 10);
        assert!(freqs[0] > 0.99 && freqs[0] < 1.01, "start: {:.3}", freqs[0]);
        let last = freqs[freqs.len() - 1];
        assert!((last - 1e6).abs() / 1e6 < 0.01, "end: {:.3e}", last);
    }

    #[test]
    fn log_freq_sweep_monotonic() {
        let freqs = log_freq_sweep(1.0, 1e8, 10);
        for i in 1..freqs.len() {
            assert!(freqs[i] > freqs[i - 1]);
        }
    }

    #[test]
    fn noise_result_integrated() {
        let t = test_transistor();
        let config = NoiseConfig::default();
        let (output_sid, input_svn, corner) = analyze_single_transistor(&t, &config);
        let freqs = log_freq_sweep(config.f_start, config.f_stop, config.points_per_decade);

        let result = NoiseResult {
            frequencies: freqs,
            sources: vec![t],
            output_noise: output_sid,
            input_noise: input_svn,
            corner_frequency: corner,
        };

        // Integrated noise over 1Hz-1MHz should be positive and finite
        let v_rms = result.integrated_noise(1.0, 1e6);
        assert!(v_rms > 0.0 && v_rms.is_finite(), "integrated noise = {:.3e}", v_rms);
    }

    #[test]
    fn noise_increases_with_temperature() {
        let t = test_transistor();
        let s_300 = t.thermal_sid(300.0);
        let s_400 = t.thermal_sid(400.0);
        assert!(
            s_400 > s_300,
            "thermal noise should increase with T: S(400K)={:.3e}, S(300K)={:.3e}",
            s_400,
            s_300
        );
    }

    #[test]
    fn input_noise_nv_sqrthz_reasonable() {
        let t = test_transistor();
        let config = NoiseConfig::default();
        let (_, input_svn, corner) = analyze_single_transistor(&t, &config);
        let freqs = log_freq_sweep(config.f_start, config.f_stop, config.points_per_decade);

        let result = NoiseResult {
            frequencies: freqs,
            sources: vec![t],
            output_noise: vec![0.0; input_svn.len()], // placeholder
            input_noise: input_svn,
            corner_frequency: corner,
        };

        // At 1kHz, input-referred noise should be in nV/sqrt(Hz) to uV/sqrt(Hz) range
        let mid_idx = result.frequencies.len() / 2;
        let nv = result.input_noise_nv_sqrthz(mid_idx);
        assert!(nv > 0.1 && nv < 1e6, "input noise = {:.1} nV/sqrt(Hz)", nv);
    }
}
