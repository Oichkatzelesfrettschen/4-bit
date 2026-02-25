//! Surface mobility models for pMOS transistors.
//!
//! Implements a Lombardi-style effective mobility model accounting for
//! vertical field degradation and temperature dependence. The 10um pMOS
//! process uses hole carriers with typical low-field mobility of
//! 150-200 cm^2/V*s at room temperature.

/// Default low-field hole mobility at 300K (cm^2/V*s).
///
/// Silicon surface mobility for holes is lower than bulk due to
/// interface scattering. Typical values for 1970s-era pMOS: 150-200.
pub const MU_0_HOLES_300K: f64 = 175.0;

/// Temperature exponent for mobility: mu ~ T^(-alpha).
/// For holes in silicon, alpha is approximately 1.5.
pub const MOBILITY_TEMP_EXPONENT: f64 = 1.5;

/// Effective surface mobility for pMOS (cm^2/V*s).
///
/// Simplified Lombardi model:
///   mu_eff = mu_0 / (1 + theta * |Vgs - Vth| / t_ox)
///
/// where theta is a fitting parameter for vertical-field degradation.
///
/// # Arguments
/// * `mu_0` - low-field mobility (cm^2/V*s)
/// * `vgs` - gate-source voltage (V, negative for pMOS)
/// * `vth` - threshold voltage (V, negative for enhancement pMOS)
/// * `t_ox` - oxide thickness (meters)
/// * `theta` - mobility degradation coefficient (typ. 0.02-0.05 for 10um)
pub fn mu_eff_pmos(mu_0: f64, vgs: f64, vth: f64, t_ox: f64, theta: f64) -> f64 {
    let vov = (vgs - vth).abs();
    // theta has units of m/V when multiplied by vov/t_ox
    let degradation = 1.0 + theta * vov / t_ox;
    mu_0 / degradation
}

/// Temperature-dependent low-field mobility (cm^2/V*s).
///
/// mu(T) = mu(300K) * (300/T)^alpha
///
/// Mobility decreases with rising temperature due to increased
/// phonon scattering.
pub fn mu_temperature(mu_300k: f64, temp_k: f64) -> f64 {
    mu_300k * (300.0 / temp_k).powf(MOBILITY_TEMP_EXPONENT)
}

/// Transconductance parameter beta = mu * Cox * W/L (A/V^2).
///
/// This is the Kp parameter used in the Shichman-Hodges model.
///
/// # Arguments
/// * `mu` - effective mobility (cm^2/V*s), will be converted to m^2/V*s
/// * `cox` - oxide capacitance per unit area (F/m^2)
/// * `w` - gate width (meters)
/// * `l` - gate length (meters)
pub fn beta(mu: f64, cox: f64, w: f64, l: f64) -> f64 {
    // Convert mu from cm^2/V*s to m^2/V*s
    let mu_si = mu * 1e-4;
    mu_si * cox * w / l
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mu_eff_no_degradation_at_threshold() {
        // At Vgs = Vth, overdrive is zero, so mu_eff should equal mu_0
        let mu = mu_eff_pmos(175.0, -3.0, -3.0, 80e-9, 0.03);
        assert!((mu - 175.0).abs() < 0.01,
            "mu_eff at Vth should equal mu_0, got {:.2}", mu);
    }

    #[test]
    fn mu_eff_degrades_with_overdrive() {
        let t_ox = 80e-9;
        let vth = -3.0;
        let theta = 0.03;

        let mu_low_vov = mu_eff_pmos(175.0, -5.0, vth, t_ox, theta);
        let mu_high_vov = mu_eff_pmos(175.0, -10.0, vth, t_ox, theta);

        // Higher overdrive -> more degradation -> lower mobility
        assert!(mu_low_vov > mu_high_vov,
            "mu at low Vov ({:.1}) should exceed mu at high Vov ({:.1})",
            mu_low_vov, mu_high_vov);
    }

    #[test]
    fn mu_temperature_decreases_with_heat() {
        let mu_300 = mu_temperature(175.0, 300.0);
        let mu_350 = mu_temperature(175.0, 350.0);
        let mu_400 = mu_temperature(175.0, 400.0);

        assert!((mu_300 - 175.0).abs() < 0.01);
        assert!(mu_350 < mu_300);
        assert!(mu_400 < mu_350);
    }

    #[test]
    fn mu_temperature_at_reference() {
        let mu = mu_temperature(175.0, 300.0);
        assert!((mu - 175.0).abs() < 0.01,
            "mu at reference temp should equal mu_0");
    }

    #[test]
    fn beta_10um_transistor() {
        // 10um/10um transistor, 80nm oxide, 175 cm^2/V*s
        let cox = 3.9 * 8.854e-12 / 80e-9; // ~4.3e-4 F/m^2
        let b = beta(175.0, cox, 10e-6, 10e-6);
        // beta = 175e-4 * 4.3e-4 * 1 = ~7.5e-6 A/V^2
        assert!(b > 1e-7 && b < 1e-4,
            "beta = {:.3e} A/V^2 for 10um/10um", b);
    }

    #[test]
    fn beta_scales_with_wl_ratio() {
        let cox = 3.9 * 8.854e-12 / 80e-9;
        let b_narrow = beta(175.0, cox, 10e-6, 10e-6); // W/L = 1
        let b_wide = beta(175.0, cox, 20e-6, 10e-6);   // W/L = 2

        assert!(((b_wide / b_narrow) - 2.0).abs() < 0.01,
            "beta should scale linearly with W/L");
    }
}
