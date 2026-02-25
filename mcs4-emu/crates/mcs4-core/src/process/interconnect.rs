//! Interconnect resistance and capacitance models.
//!
//! Models for polysilicon, diffusion, and metal (aluminum) interconnects
//! used in the Intel 10um pMOS process. Includes Elmore delay estimation
//! for RC-dominated signal paths.

use super::silicon::EPSILON_OX;

/// Sheet resistance of polysilicon gate material (ohm/square).
/// Intel 10um pMOS SGT process: ~30-50 ohm/sq.
pub const R_SHEET_POLY: f64 = 40.0;

/// Sheet resistance of p+ diffusion (ohm/square).
/// Typical for 1970s pMOS: ~50-100 ohm/sq.
pub const R_SHEET_DIFFUSION: f64 = 75.0;

/// Sheet resistance of aluminum metal (ohm/square).
/// Typical: ~0.03-0.08 ohm/sq.
pub const R_SHEET_METAL: f64 = 0.05;

/// Wire resistance from sheet resistance (ohms).
///
/// R = R_sheet * L / W
///
/// # Arguments
/// * `r_sheet` - sheet resistance (ohm/square)
/// * `length` - wire length (same units as width)
/// * `width` - wire width (same units as length)
pub fn wire_resistance(r_sheet: f64, length: f64, width: f64) -> f64 {
    r_sheet * length / width
}

/// Parallel-plate capacitance per unit area (F/m^2).
///
/// C = epsilon / d
///
/// # Arguments
/// * `epsilon` - permittivity of dielectric (F/m)
/// * `d` - dielectric thickness (m)
pub fn parallel_plate_cap(epsilon: f64, d: f64) -> f64 {
    epsilon / d
}

/// Total wire capacitance (F) including parallel-plate and fringing.
///
/// C_wire = C_pp * W * L + C_fringe * 2 * L
///
/// Fringing capacitance is estimated as epsilon_ox / pi * ln(1 + t/d)
/// where t is the conductor thickness.
///
/// # Arguments
/// * `width` - wire width (m)
/// * `length` - wire length (m)
/// * `thickness` - conductor thickness (m)
/// * `dielectric_d` - dielectric thickness to substrate (m)
pub fn wire_capacitance(width: f64, length: f64, thickness: f64, dielectric_d: f64) -> f64 {
    let c_pp = parallel_plate_cap(EPSILON_OX, dielectric_d);
    let c_fringe_per_m = EPSILON_OX / std::f64::consts::PI * (1.0 + thickness / dielectric_d).ln();
    c_pp * width * length + c_fringe_per_m * 2.0 * length
}

/// Elmore delay for an RC wire segment (seconds).
///
/// tau = R * C / 2 (for distributed RC line, first pole approximation)
///
/// For a lumped RC:
/// tau = R * C
///
/// # Arguments
/// * `r` - total wire resistance (ohms)
/// * `c` - total wire capacitance (F)
/// * `distributed` - if true, use R*C/2 for distributed line
pub fn elmore_delay(r: f64, c: f64, distributed: bool) -> f64 {
    if distributed {
        r * c / 2.0
    } else {
        r * c
    }
}

/// Elmore delay for a chain of RC segments (seconds).
///
/// tau = sum_i(R_i * sum_j>=i(C_j))
///
/// # Arguments
/// * `segments` - slice of (resistance, capacitance) pairs in signal order
pub fn elmore_delay_chain(segments: &[(f64, f64)]) -> f64 {
    let mut delay = 0.0;
    for (i, &(r_i, _)) in segments.iter().enumerate() {
        let downstream_cap: f64 = segments[i..].iter().map(|&(_, c)| c).sum();
        delay += r_i * downstream_cap;
    }
    delay
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_resistance_poly() {
        // 100um long, 10um wide poly wire
        let r = wire_resistance(R_SHEET_POLY, 100.0, 10.0);
        // 40 * 10 = 400 ohm
        assert!((r - 400.0).abs() < 0.1, "R_poly = {:.1} ohm for 100um/10um", r);
    }

    #[test]
    fn wire_resistance_metal_much_lower() {
        let r_poly = wire_resistance(R_SHEET_POLY, 100.0, 10.0);
        let r_metal = wire_resistance(R_SHEET_METAL, 100.0, 10.0);
        assert!(
            r_metal < r_poly / 100.0,
            "Metal should be >100x lower resistance than poly"
        );
    }

    #[test]
    fn wire_capacitance_positive() {
        let c = wire_capacitance(10e-6, 100e-6, 0.5e-6, 1e-6);
        assert!(c > 0.0 && c.is_finite());
    }

    #[test]
    fn wire_capacitance_scales_with_length() {
        let c_short = wire_capacitance(10e-6, 50e-6, 0.5e-6, 1e-6);
        let c_long = wire_capacitance(10e-6, 100e-6, 0.5e-6, 1e-6);
        // Capacitance should roughly double with length
        assert!((c_long / c_short - 2.0).abs() < 0.2);
    }

    #[test]
    fn elmore_delay_lumped() {
        // 1k ohm, 1pF -> 1ns
        let tau = elmore_delay(1e3, 1e-12, false);
        assert!(
            (tau - 1e-9).abs() < 1e-12,
            "Elmore delay = {:.3e} s, expected 1e-9 s",
            tau
        );
    }

    #[test]
    fn elmore_delay_distributed_half() {
        let tau_lumped = elmore_delay(1e3, 1e-12, false);
        let tau_dist = elmore_delay(1e3, 1e-12, true);
        assert!(((tau_dist / tau_lumped) - 0.5).abs() < 0.01);
    }

    #[test]
    fn elmore_chain_single_segment_matches_lumped() {
        let tau_chain = elmore_delay_chain(&[(1e3, 1e-12)]);
        let tau_lumped = elmore_delay(1e3, 1e-12, false);
        assert!((tau_chain - tau_lumped).abs() < 1e-15);
    }

    #[test]
    fn elmore_chain_two_segments() {
        // R1=1k, C1=1pF, R2=1k, C2=1pF
        // tau = R1*(C1+C2) + R2*C2 = 1k*2p + 1k*1p = 3ns
        let tau = elmore_delay_chain(&[(1e3, 1e-12), (1e3, 1e-12)]);
        assert!(
            (tau - 3e-9).abs() < 1e-12,
            "Elmore chain delay = {:.3e} s, expected 3e-9 s",
            tau
        );
    }
}
