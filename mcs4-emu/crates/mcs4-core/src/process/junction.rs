//! PN junction capacitance models.
//!
//! Computes voltage-dependent junction capacitance for source/drain
//! diffusions in pMOS transistors. Uses the standard abrupt-junction
//! model with grading coefficient.

use super::silicon::{built_in_potential, EPSILON_SI, Q};

/// Junction capacitance as a function of reverse bias (F/m^2).
///
/// C_j(V_r) = C_j0 / (1 + V_r / phi_bi)^mj
///
/// # Arguments
/// * `cj0` - zero-bias junction capacitance (F/m^2)
/// * `v_r` - reverse bias voltage (V, positive for reverse bias)
/// * `phi_bi` - built-in potential (V)
/// * `mj` - grading coefficient (0.5 for abrupt, 0.33 for linear graded)
pub fn cj(cj0: f64, v_r: f64, phi_bi: f64, mj: f64) -> f64 {
    // Clamp to avoid forward-bias singularity (limit to 80% of phi_bi)
    let v_eff = if v_r < -0.8 * phi_bi {
        -0.8 * phi_bi
    } else {
        v_r
    };
    cj0 / (1.0 + v_eff / phi_bi).powf(mj)
}

/// Zero-bias junction capacitance per unit area (F/m^2).
///
/// C_j0 = sqrt(q * epsilon_si * N_eff / (2 * phi_bi))
///
/// where N_eff = Na * Nd / (Na + Nd) for one-sided junctions.
///
/// For the MCS-4 pMOS process:
/// - p+ source/drain into n-type substrate
/// - Na (source/drain) >> Nd (substrate), so N_eff ~ Nd
pub fn cj0(n_a: f64, n_d: f64, temp_k: f64) -> f64 {
    let phi = built_in_potential(n_a, n_d, temp_k);
    let n_eff_cm3 = n_a * n_d / (n_a + n_d);
    // Convert to m^-3 for SI
    let n_eff = n_eff_cm3 * 1e6;
    (Q * EPSILON_SI * n_eff / (2.0 * phi)).sqrt()
}

/// Total source or drain junction capacitance (F).
///
/// C_total = C_j * A_bottom + C_jsw * P_sidewall
///
/// For simplified estimation without sidewall:
/// C_total = C_j * area
///
/// # Arguments
/// * `cj_val` - junction capacitance per unit area (F/m^2)
/// * `area` - junction area (m^2)
pub fn junction_cap_total(cj_val: f64, area: f64) -> f64 {
    cj_val * area
}

/// Sidewall junction capacitance per unit length (F/m).
///
/// For simplified estimation:
/// C_jsw ~ C_j0 * x_j
///
/// where x_j is the junction depth.
pub fn cjsw(cj0_val: f64, x_j: f64) -> f64 {
    cj0_val * x_j
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cj_at_zero_bias_equals_cj0() {
        let cj0_val = 1e-4; // F/m^2
        let phi_bi = 0.7;
        let c = cj(cj0_val, 0.0, phi_bi, 0.5);
        assert!((c - cj0_val).abs() / cj0_val < 1e-6,
            "C_j(0) should equal C_j0");
    }

    #[test]
    fn cj_decreases_with_reverse_bias() {
        let cj0_val = 1e-4;
        let phi_bi = 0.7;
        let mj = 0.5;

        let c_0v = cj(cj0_val, 0.0, phi_bi, mj);
        let c_5v = cj(cj0_val, 5.0, phi_bi, mj);
        let c_15v = cj(cj0_val, 15.0, phi_bi, mj);

        assert!(c_0v > c_5v, "C_j should decrease with reverse bias");
        assert!(c_5v > c_15v, "C_j should continue decreasing");
    }

    #[test]
    fn cj_forward_bias_clamped() {
        // Should not blow up in slight forward bias
        let cj0_val = 1e-4;
        let phi_bi = 0.7;
        let c = cj(cj0_val, -0.9 * phi_bi, phi_bi, 0.5);
        // Should be clamped, yielding a finite value larger than cj0
        assert!(c.is_finite());
        assert!(c > cj0_val);
    }

    #[test]
    fn cj0_mcs4_substrate() {
        // p+ source: ~1e19 cm^-3, n-substrate: ~1e15 cm^-3
        let c = cj0(1e19, 1e15, 300.0);
        // Should be in the range 1e-5 to 1e-3 F/m^2
        assert!(c > 1e-5 && c < 1e-2,
            "C_j0 = {:.3e} F/m^2", c);
    }

    #[test]
    fn junction_cap_total_scales_with_area() {
        let cj_val = 1e-4; // F/m^2
        let area_small = 10e-6 * 5e-6;  // 10um x 5um
        let area_large = 20e-6 * 5e-6;  // 20um x 5um

        let c_small = junction_cap_total(cj_val, area_small);
        let c_large = junction_cap_total(cj_val, area_large);

        assert!(((c_large / c_small) - 2.0).abs() < 0.01);
    }
}
