//! Temperature-dependent parameter models.
//!
//! Wraps the temperature dependencies of threshold voltage, mobility,
//! and intrinsic carrier concentration into a single interface for
//! simulation sweeps across the MCS-4 operating envelope (0-70 deg C).

use super::{mobility, silicon};

/// Temperature coefficient of threshold voltage (V/K).
///
/// For pMOS enhancement mode, Vth becomes less negative (smaller magnitude)
/// as temperature increases: dVth/dT ~ +1 to +3 mV/K.
pub const DVTH_DT: f64 = 2.0e-3; // +2 mV/K typical

/// Threshold voltage at temperature T (V).
///
/// Vth(T) = Vth(T_ref) + dVth/dT * (T - T_ref)
///
/// # Arguments
/// * `vth_ref` - threshold voltage at reference temperature (V)
/// * `temp_k` - temperature (K)
/// * `temp_ref` - reference temperature (K), typically 300
pub fn vth_at_temp(vth_ref: f64, temp_k: f64, temp_ref: f64) -> f64 {
    vth_ref + DVTH_DT * (temp_k - temp_ref)
}

/// Effective mobility at temperature T (cm^2/V*s).
///
/// Combines temperature-dependent low-field mobility with
/// vertical-field degradation.
///
/// # Arguments
/// * `mu_0_ref` - low-field mobility at 300K (cm^2/V*s)
/// * `vgs` - gate-source voltage (V)
/// * `vth` - threshold voltage at operating temperature (V)
/// * `t_ox` - oxide thickness (m)
/// * `theta` - mobility degradation coefficient
/// * `temp_k` - temperature (K)
pub fn mu_eff_at_temp(mu_0_ref: f64, vgs: f64, vth: f64, t_ox: f64, theta: f64, temp_k: f64) -> f64 {
    let mu_0_t = mobility::mu_temperature(mu_0_ref, temp_k);
    mobility::mu_eff_pmos(mu_0_t, vgs, vth, t_ox, theta)
}

/// Intrinsic carrier concentration at temperature (cm^-3).
///
/// Re-export of silicon::ni for convenience in thermal sweeps.
pub fn ni_at_temp(temp_k: f64) -> f64 {
    silicon::ni(temp_k)
}

/// Convert Celsius to Kelvin.
pub fn celsius_to_kelvin(celsius: f64) -> f64 {
    celsius + 273.15
}

/// Convert Kelvin to Celsius.
pub fn kelvin_to_celsius(kelvin: f64) -> f64 {
    kelvin - 273.15
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vth_shifts_positive_with_temperature() {
        let vth_300 = -3.0;
        let vth_350 = vth_at_temp(vth_300, 350.0, 300.0);
        // Should shift positive (less negative) by 50K * 2mV/K = +100mV
        assert!(
            vth_350 > vth_300,
            "Vth should become less negative: Vth(350K) = {:.3} V",
            vth_350
        );
        let shift = vth_350 - vth_300;
        assert!(
            (shift - 0.1).abs() < 0.001,
            "Shift should be ~100mV, got {:.1} mV",
            shift * 1000.0
        );
    }

    #[test]
    fn vth_unchanged_at_reference() {
        let vth_ref = -3.0;
        let vth = vth_at_temp(vth_ref, 300.0, 300.0);
        assert!((vth - vth_ref).abs() < 1e-10);
    }

    #[test]
    fn mu_eff_decreases_with_temperature() {
        let t_ox = 80e-9;
        let vgs = -8.0;
        let vth = -3.0;
        let theta = 0.03;

        let mu_300 = mu_eff_at_temp(175.0, vgs, vth, t_ox, theta, 300.0);
        let mu_350 = mu_eff_at_temp(175.0, vgs, vth, t_ox, theta, 350.0);

        assert!(
            mu_300 > mu_350,
            "Mobility should decrease: mu(300K)={:.1}, mu(350K)={:.1}",
            mu_300,
            mu_350
        );
    }

    #[test]
    fn temperature_conversions_roundtrip() {
        let c = 25.0;
        let k = celsius_to_kelvin(c);
        assert!((k - 298.15).abs() < 1e-10);
        let c2 = kelvin_to_celsius(k);
        assert!((c2 - c).abs() < 1e-10);
    }

    #[test]
    fn operating_range_0c_to_70c() {
        // Verify all parameters are well-behaved across MCS-4 operating range
        let t_min = celsius_to_kelvin(0.0);
        let t_max = celsius_to_kelvin(70.0);
        let vth_ref = -3.0;

        for temp_k in [t_min, 300.0, t_max] {
            let vth = vth_at_temp(vth_ref, temp_k, 300.0);
            let ni = ni_at_temp(temp_k);
            let mu = mu_eff_at_temp(175.0, -8.0, vth, 80e-9, 0.03, temp_k);

            assert!(vth.is_finite() && vth < 0.0, "Vth={:.3} at {:.0}K", vth, temp_k);
            assert!(ni.is_finite() && ni > 0.0, "ni={:.3e} at {:.0}K", ni, temp_k);
            assert!(mu.is_finite() && mu > 0.0, "mu={:.1} at {:.0}K", mu, temp_k);
        }
    }
}
