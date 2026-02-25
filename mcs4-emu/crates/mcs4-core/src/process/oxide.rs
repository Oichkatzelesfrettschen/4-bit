//! Gate oxide capacitance and breakdown models.
//!
//! The Intel 10um pMOS process used thermally grown SiO2 gate oxide,
//! approximately 80nm (800 angstrom) thick. This module computes
//! oxide capacitance per unit area and total gate capacitance from
//! oxide thickness and transistor geometry.

use super::silicon::EPSILON_OX;

/// Oxide capacitance per unit area (F/m^2).
///
/// C_ox = epsilon_ox / t_ox
///
/// # Arguments
/// * `t_ox` - oxide thickness in meters
pub fn cox(t_ox: f64) -> f64 {
    EPSILON_OX / t_ox
}

/// Total gate capacitance (F) for a transistor.
///
/// C_gate = C_ox * W * L
///
/// # Arguments
/// * `cox_per_m2` - oxide capacitance per unit area (F/m^2)
/// * `w` - gate width in meters
/// * `l` - gate length in meters
pub fn cgate(cox_per_m2: f64, w: f64, l: f64) -> f64 {
    cox_per_m2 * w * l
}

/// Oxide breakdown electric field (V/m).
///
/// Typical SiO2 breakdown: ~10 MV/cm = 1e9 V/m.
/// For reliability, devices operate well below this.
pub const OXIDE_BREAKDOWN_FIELD: f64 = 1.0e9;

/// Maximum safe gate voltage for given oxide thickness (V).
///
/// Uses a 2x safety margin below breakdown.
pub fn max_gate_voltage(t_ox: f64) -> f64 {
    OXIDE_BREAKDOWN_FIELD * t_ox / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cox_80nm_oxide() {
        // 80nm = 80e-9 m
        let c = cox(80e-9);
        // C_ox = 3.9 * 8.854e-12 / 80e-9 = ~4.32e-4 F/m^2
        let expected = 3.9 * 8.854_187_812_8e-12 / 80e-9;
        assert!(
            (c - expected).abs() / expected < 1e-6,
            "C_ox(80nm) = {:.4e} F/m^2, expected {:.4e}",
            c,
            expected
        );
    }

    #[test]
    fn cox_inversely_proportional_to_thickness() {
        let c_thin = cox(50e-9);
        let c_thick = cox(100e-9);
        // Thinner oxide -> higher capacitance
        assert!(c_thin > c_thick);
        // Ratio should be 100/50 = 2x
        assert!(((c_thin / c_thick) - 2.0).abs() < 0.01);
    }

    #[test]
    fn cgate_10um_transistor() {
        // 10um x 10um transistor with 80nm oxide
        let c_ox = cox(80e-9);
        let w = 10e-6; // 10um
        let l = 10e-6; // 10um
        let cg = cgate(c_ox, w, l);
        // Should be in the femtofarad range (1e-15 to 1e-13)
        assert!(cg > 1e-16 && cg < 1e-12, "C_gate(10um x 10um) = {:.3e} F", cg);
    }

    #[test]
    fn cgate_scales_with_area() {
        let c_ox = cox(80e-9);
        let cg_small = cgate(c_ox, 5e-6, 10e-6);
        let cg_large = cgate(c_ox, 20e-6, 10e-6);
        // 4x wider -> 4x capacitance
        assert!(((cg_large / cg_small) - 4.0).abs() < 0.01);
    }

    #[test]
    fn max_voltage_80nm_oxide() {
        let v_max = max_gate_voltage(80e-9);
        // 1e9 * 80e-9 / 2 = 40V (with 2x safety margin)
        assert!(
            (v_max - 40.0).abs() < 0.1,
            "Max gate voltage = {:.1} V for 80nm oxide",
            v_max
        );
        // MCS-4 uses -15V, well within safe range
        assert!(v_max.abs() > 15.0);
    }
}
