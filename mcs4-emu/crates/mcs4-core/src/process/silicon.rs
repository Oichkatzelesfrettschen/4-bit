//! Fundamental silicon and semiconductor physical constants.
//!
//! Provides temperature-dependent intrinsic carrier concentration,
//! thermal voltage, built-in potential, and Debye length calculations
//! for silicon substrates used in 1970s-era pMOS processes.

/// Elementary charge (Coulombs).
pub const Q: f64 = 1.602_176_634e-19;

/// Boltzmann constant (J/K).
pub const K_B: f64 = 1.380_649e-23;

/// Vacuum permittivity (F/m).
pub const EPSILON_0: f64 = 8.854_187_812_8e-12;

/// Relative permittivity of silicon.
pub const EPSILON_SI_REL: f64 = 11.7;

/// Absolute permittivity of silicon (F/m).
pub const EPSILON_SI: f64 = EPSILON_0 * EPSILON_SI_REL;

/// Relative permittivity of SiO2.
pub const EPSILON_OX_REL: f64 = 3.9;

/// Absolute permittivity of SiO2 (F/m).
pub const EPSILON_OX: f64 = EPSILON_0 * EPSILON_OX_REL;

/// Silicon bandgap at 300K (eV).
pub const E_G_300K: f64 = 1.12;

/// Intrinsic carrier concentration at 300K (cm^-3).
/// Standard textbook value for silicon.
pub const NI_300K: f64 = 1.5e10;

/// Hole saturation velocity in silicon (cm/s).
/// At room temperature, approximately 8e6 cm/s for holes.
/// Relevant for velocity saturation in short-channel devices,
/// though the 10um MCS-4 process is well above this limit.
pub const V_SAT_HOLES: f64 = 8.0e6;

/// Electron effective mass relative to free electron mass.
/// m_e* / m_0 for silicon (density-of-states effective mass).
pub const M_E_EFF_REL: f64 = 1.08;

/// Hole effective mass relative to free electron mass.
/// m_h* / m_0 for silicon (density-of-states effective mass).
pub const M_H_EFF_REL: f64 = 0.81;

/// Free electron mass (kg).
pub const M_0: f64 = 9.109_383_701_5e-31;

/// Thermal voltage at temperature T (Kelvin).
///
/// V_T = k_B * T / q
///
/// At 300K this is approximately 25.85 mV.
pub fn vt(temp_k: f64) -> f64 {
    K_B * temp_k / Q
}

/// Intrinsic carrier concentration as a function of temperature.
///
/// Uses the empirical relation:
///   ni(T) = ni(300K) * (T/300)^(3/2) * exp(-Eg/(2*kT) + Eg/(2*k*300))
///
/// This is a simplified model adequate for the temperature ranges
/// encountered in the MCS-4 operating envelope (0 to 70 deg C).
pub fn ni(temp_k: f64) -> f64 {
    let ratio = temp_k / 300.0;
    let exp_arg = -E_G_300K * Q / (2.0 * K_B) * (1.0 / temp_k - 1.0 / 300.0);
    NI_300K * ratio.powf(1.5) * exp_arg.exp()
}

/// Fermi potential for doped silicon (V).
///
/// phi_F = V_T * ln(N_doping / ni)
///
/// For n-type substrate (as used in pMOS), N_doping is the donor concentration.
/// Positive for n-type, negative for p-type.
pub fn phi_f(n_doping: f64, temp_k: f64) -> f64 {
    let vth = vt(temp_k);
    let ni_t = ni(temp_k);
    vth * (n_doping / ni_t).ln()
}

/// Debye length in silicon (meters).
///
/// L_D = sqrt(epsilon_si * V_T / (q * N_doping))
///
/// Characteristic screening length in the semiconductor.
pub fn debye_length(n_doping: f64, temp_k: f64) -> f64 {
    let vth = vt(temp_k);
    // n_doping is in cm^-3, convert to m^-3 for SI units
    let n_si = n_doping * 1e6;
    (EPSILON_SI * vth / (Q * n_si)).sqrt()
}

/// Built-in potential of a pn junction (V).
///
/// V_bi = V_T * ln(N_a * N_d / ni^2)
pub fn built_in_potential(n_a: f64, n_d: f64, temp_k: f64) -> f64 {
    let vth = vt(temp_k);
    let ni_t = ni(temp_k);
    vth * (n_a * n_d / (ni_t * ni_t)).ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thermal_voltage_at_300k() {
        let vt_300 = vt(300.0);
        // Should be approximately 25.85 mV
        assert!((vt_300 - 0.02585).abs() < 1e-4,
            "V_T at 300K = {:.5} V, expected ~0.02585 V", vt_300);
    }

    #[test]
    fn thermal_voltage_increases_with_temperature() {
        assert!(vt(400.0) > vt(300.0));
        assert!(vt(300.0) > vt(200.0));
    }

    #[test]
    fn intrinsic_carrier_at_300k() {
        let ni_val = ni(300.0);
        // Should be close to 1.5e10 cm^-3
        assert!((ni_val - NI_300K).abs() / NI_300K < 0.01,
            "ni(300K) = {:.3e}, expected ~{:.3e}", ni_val, NI_300K);
    }

    #[test]
    fn ni_increases_with_temperature() {
        // Carrier concentration rises sharply with temperature
        assert!(ni(400.0) > ni(300.0));
        // At 400K should be orders of magnitude larger
        assert!(ni(400.0) > 100.0 * ni(300.0));
    }

    #[test]
    fn fermi_potential_n_type_substrate() {
        // 10 ohm-cm n-type ~ 4.5e14 cm^-3 phosphorus
        let n_sub = 4.5e14;
        let phi = phi_f(n_sub, 300.0);
        // phi_F should be positive for n-type, around 0.27V
        assert!(phi > 0.2 && phi < 0.4,
            "phi_F = {:.3} V for n-type 10 ohm-cm substrate", phi);
    }

    #[test]
    fn debye_length_order_of_magnitude() {
        // For ~1e15 cm^-3 doping at 300K, Debye length ~ 0.1-1 um
        let ld = debye_length(1e15, 300.0);
        let ld_um = ld * 1e6;
        assert!(ld_um > 0.01 && ld_um < 10.0,
            "Debye length = {:.4} um, expected 0.01-10 um", ld_um);
    }

    #[test]
    fn built_in_potential_reasonable() {
        // Typical pn junction: Na=1e17, Nd=1e15
        let vbi = built_in_potential(1e17, 1e15, 300.0);
        // Should be around 0.7V for silicon
        assert!(vbi > 0.5 && vbi < 1.0,
            "V_bi = {:.3} V, expected 0.5-1.0 V", vbi);
    }

    #[test]
    fn permittivity_values() {
        // Verify derived constants
        assert!((EPSILON_SI / EPSILON_0 - 11.7).abs() < 0.01);
        assert!((EPSILON_OX / EPSILON_0 - 3.9).abs() < 0.01);
    }

    #[test]
    fn hole_saturation_velocity_reasonable() {
        // v_sat for holes in silicon ~ 8e6 cm/s
        // Compile-time check: constants are in valid range
        const { assert!(V_SAT_HOLES > 1e6 && V_SAT_HOLES < 1e8) };
    }

    #[test]
    fn effective_masses_reasonable() {
        // Effective masses should be close to free electron mass
        const { assert!(M_E_EFF_REL > 0.5 && M_E_EFF_REL < 2.0) };
        const { assert!(M_H_EFF_REL > 0.3 && M_H_EFF_REL < 2.0) };
        const { assert!(M_0 > 9e-31 && M_0 < 1e-30) };
    }
}
