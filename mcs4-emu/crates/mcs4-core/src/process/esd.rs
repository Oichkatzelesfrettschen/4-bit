//! ESD protection diode model for MCS-4 I/O pads.
//!
//! WHY: Every chip pin has ESD protection diodes that clamp voltage
//! excursions. These diodes affect signal integrity (leakage, capacitance)
//! and must be modeled for accurate pad-level simulation.
//!
//! WHAT: Forward exponential and reverse leakage + breakdown models
//! for p+/n-well protection diodes at each I/O pad.
//!
//! HOW: Standard Shockley diode equation with reverse breakdown.

use super::ProcessParams;

/// Saturation current for ESD protection diode (A).
/// Typical for large-area p+/n junction at 300K.
const IS_ESD: f64 = 1e-14;

/// Ideality factor for the diode.
const N_DIODE: f64 = 1.05;

/// Reverse breakdown voltage (V). Estimated for 10um process.
/// Typically 15-25V for p+/n junctions with N_sub ~ 4.5e14.
const V_BREAKDOWN: f64 = 20.0;

/// Breakdown softness factor. Smaller = sharper breakdown knee.
const BREAKDOWN_KNEE: f64 = 0.5;

/// Junction capacitance of ESD diode at zero bias (F).
/// Larger than internal junctions due to large pad area.
const CJ0_ESD: f64 = 0.5e-12; // 0.5pF

/// Forward diode current (A) using Shockley equation.
///
/// I = Is * (exp(V / (n * Vt)) - 1)
///
/// # Arguments
/// * `v_forward` - forward bias voltage (V, positive)
/// * `params` - process parameters (for thermal voltage)
pub fn forward_current(v_forward: f64, params: &ProcessParams) -> f64 {
    let vt = params.vt();
    IS_ESD * ((v_forward / (N_DIODE * vt)).exp() - 1.0)
}

/// Reverse leakage current (A).
///
/// In reverse bias, current is approximately -Is plus avalanche
/// breakdown contribution near V_BREAKDOWN.
///
/// # Arguments
/// * `v_reverse` - reverse bias voltage magnitude (V, positive)
/// * `params` - process parameters
pub fn reverse_current(v_reverse: f64, params: &ProcessParams) -> f64 {
    let _ = params;
    // Leakage component
    let i_leak = IS_ESD;

    // Avalanche breakdown component (empirical)
    let breakdown_factor = if v_reverse > V_BREAKDOWN * 0.8 {
        let x = (v_reverse - V_BREAKDOWN) / BREAKDOWN_KNEE;
        if x > 0.0 {
            // Beyond breakdown: exponential increase
            (x.exp() - 1.0) * 1e-3
        } else {
            // Pre-breakdown: slight increase
            0.0
        }
    } else {
        0.0
    };

    i_leak + breakdown_factor
}

/// Forward voltage drop at a given current (V).
///
/// V = n * Vt * ln(I/Is + 1)
///
/// # Arguments
/// * `current` - forward current (A)
/// * `params` - process parameters
pub fn forward_voltage(current: f64, params: &ProcessParams) -> f64 {
    if current <= 0.0 {
        return 0.0;
    }
    let vt = params.vt();
    N_DIODE * vt * (current / IS_ESD + 1.0).ln()
}

/// Junction capacitance under reverse bias (F).
///
/// Cj = Cj0 / (1 + V_reverse / phi_b)^0.5
///
/// # Arguments
/// * `v_reverse` - reverse bias voltage (V, positive)
/// * `params` - process parameters
pub fn junction_capacitance(v_reverse: f64, params: &ProcessParams) -> f64 {
    let phi_b = params.phi_b();
    if phi_b <= 0.0 {
        return CJ0_ESD;
    }
    CJ0_ESD / (1.0 + v_reverse / phi_b).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_drop_in_expected_range() {
        let p = ProcessParams::default();
        // At 1mA forward current, expect 0.5-0.8V drop
        let vf = forward_voltage(1e-3, &p);
        assert!(
            vf > 0.4 && vf < 1.0,
            "Forward drop at 1mA = {:.3}V, expected 0.4-1.0V",
            vf
        );
    }

    #[test]
    fn forward_current_exponential() {
        let p = ProcessParams::default();
        let i1 = forward_current(0.5, &p);
        let i2 = forward_current(0.6, &p);
        // Each 0.1V increase should give ~50x more current (exp(0.1/0.026))
        assert!(i2 > i1 * 10.0, "Current should increase exponentially");
    }

    #[test]
    fn reverse_leakage_small() {
        let p = ProcessParams::default();
        let i_rev = reverse_current(5.0, &p);
        // Reverse leakage at 5V should be near Is (~fA)
        assert!(
            i_rev < 1e-9,
            "Reverse leakage at 5V = {:.2e} A, should be sub-nA",
            i_rev
        );
    }

    #[test]
    fn breakdown_increases_current() {
        let p = ProcessParams::default();
        let i_pre = reverse_current(V_BREAKDOWN * 0.9, &p);
        let i_post = reverse_current(V_BREAKDOWN * 1.1, &p);
        assert!(
            i_post > i_pre * 10.0,
            "Current should increase sharply past breakdown: pre={:.2e}, post={:.2e}",
            i_pre,
            i_post
        );
    }

    #[test]
    fn junction_cap_decreases_with_reverse_bias() {
        let p = ProcessParams::default();
        let c0 = junction_capacitance(0.0, &p);
        let c5 = junction_capacitance(5.0, &p);
        let c10 = junction_capacitance(10.0, &p);

        assert!(c0 > c5, "Cap should decrease with reverse bias");
        assert!(c5 > c10, "Cap should decrease with reverse bias");
        assert!((c0 - CJ0_ESD).abs() < 1e-15, "Zero-bias cap should equal CJ0_ESD");
    }
}
