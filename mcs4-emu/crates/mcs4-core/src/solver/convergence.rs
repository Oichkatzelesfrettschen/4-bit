//! Convergence aids for Newton-Raphson iteration.
//!
//! Provides voltage limiting, source stepping, and gmin stepping
//! to help the DC operating point solver converge for circuits with
//! strong nonlinearities (pMOS transistors).

/// Limit voltage change per Newton iteration to prevent oscillation.
///
/// Restricts the voltage update to avoid overshooting past the
/// transistor's operating regions.
///
/// # Arguments
/// * `v_new` - proposed new voltage
/// * `v_old` - previous iteration voltage
/// * `v_limit` - maximum allowed change magnitude
///
/// # Returns
/// Clamped voltage
pub fn voltage_limit(v_new: f64, v_old: f64, v_limit: f64) -> f64 {
    let delta = v_new - v_old;
    if delta.abs() > v_limit {
        v_old + delta.signum() * v_limit
    } else {
        v_new
    }
}

/// pMOS-specific voltage limiting for Vgs.
///
/// Prevents the gate-source voltage from jumping more than a critical
/// amount per iteration, which would cause the device model to switch
/// between regions abruptly.
pub fn pmos_vgs_limit(vgs_new: f64, vgs_old: f64, vth: f64) -> f64 {
    let vcrit = vth.abs() * 0.5;
    // Allow larger steps in cutoff, smaller steps near threshold
    let limit = if vgs_old.abs() > 2.0 * vth.abs() {
        // Deep in operating region: allow 2V steps
        2.0
    } else {
        // Near threshold: limit to fraction of |Vth|
        vcrit
    };
    voltage_limit(vgs_new, vgs_old, limit)
}

/// Source stepping ramp factor.
///
/// Returns a scaling factor [0, 1] for supply voltages during
/// source stepping convergence aid.
///
/// # Arguments
/// * `step` - current step number (0-based)
/// * `total_steps` - total number of ramp steps
pub fn source_step_factor(step: usize, total_steps: usize) -> f64 {
    if total_steps == 0 {
        return 1.0;
    }
    let s = (step + 1) as f64;
    let t = total_steps as f64;
    (s / t).min(1.0)
}

/// Gmin stepping schedule.
///
/// Returns a decreasing gmin value over iterations to gradually
/// remove the convergence aid.
///
/// # Arguments
/// * `gmin_initial` - starting gmin value
/// * `step` - current step (0-based)
/// * `total_steps` - total gmin reduction steps
pub fn gmin_schedule(gmin_initial: f64, step: usize, total_steps: usize) -> f64 {
    if total_steps == 0 || step >= total_steps {
        return gmin_initial;
    }
    let ratio = 10.0_f64; // decrease by 10x per step
    gmin_initial / ratio.powi(step as i32)
}

/// Check if Newton-Raphson has converged.
///
/// # Arguments
/// * `v_new` - new voltage vector
/// * `v_old` - previous voltage vector
/// * `v_tol` - voltage tolerance
///
/// # Returns
/// true if all voltages have converged
pub fn check_convergence(v_new: &[f64], v_old: &[f64], v_tol: f64) -> bool {
    v_new.iter().zip(v_old.iter()).all(|(&vn, &vo)| {
        (vn - vo).abs() < v_tol
    })
}

/// Compute the maximum voltage change across all nodes.
pub fn max_delta(v_new: &[f64], v_old: &[f64]) -> f64 {
    v_new.iter().zip(v_old.iter())
        .map(|(&vn, &vo)| (vn - vo).abs())
        .fold(0.0, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voltage_limit_clamps() {
        assert!((voltage_limit(10.0, 0.0, 2.0) - 2.0).abs() < 1e-10);
        assert!((voltage_limit(-10.0, 0.0, 2.0) - (-2.0)).abs() < 1e-10);
    }

    #[test]
    fn voltage_limit_passes_small_change() {
        assert!((voltage_limit(1.0, 0.0, 5.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn pmos_vgs_limit_near_threshold() {
        let vth = -3.0;
        // Near threshold: limit should be small (Vth/2 = 1.5V)
        let limited = pmos_vgs_limit(-5.0, -2.0, vth);
        assert!((limited - (-2.0)).abs() <= 1.5 + 1e-10);
    }

    #[test]
    fn source_step_ramps_linearly() {
        assert!((source_step_factor(0, 10) - 0.1).abs() < 1e-10);
        assert!((source_step_factor(4, 10) - 0.5).abs() < 1e-10);
        assert!((source_step_factor(9, 10) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn source_step_zero_total_returns_1() {
        assert!((source_step_factor(0, 0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn gmin_schedule_decreases() {
        let g0 = gmin_schedule(1e-6, 0, 5);
        let g1 = gmin_schedule(1e-6, 1, 5);
        let g2 = gmin_schedule(1e-6, 2, 5);
        assert!(g0 > g1);
        assert!(g1 > g2);
    }

    #[test]
    fn check_convergence_detects_converged() {
        let v_new = vec![1.0, 2.0, 3.0];
        let v_old = vec![1.0 + 1e-8, 2.0 - 1e-8, 3.0];
        assert!(check_convergence(&v_new, &v_old, 1e-6));
    }

    #[test]
    fn check_convergence_detects_not_converged() {
        let v_new = vec![1.0, 2.0, 3.0];
        let v_old = vec![1.0, 2.5, 3.0];
        assert!(!check_convergence(&v_new, &v_old, 1e-6));
    }

    #[test]
    fn max_delta_correct() {
        let v_new = vec![1.0, 5.0, 3.0];
        let v_old = vec![1.0, 2.0, 3.5];
        assert!((max_delta(&v_new, &v_old) - 3.0).abs() < 1e-10);
    }
}
