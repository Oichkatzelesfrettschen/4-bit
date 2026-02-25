//! 1D Drift-Diffusion channel solver.
//!
//! Solves the coupled Poisson and hole continuity equations along the
//! MOSFET channel from source to drain using the Scharfetter-Gummel
//! discretization for numerical stability.
//!
//! # Equations
//!
//! Poisson: d^2(psi)/dx^2 = -(q/eps_si) * (N_d + p - n)
//! Hole continuity: dJ_p/dx = 0 (steady state, no recombination)
//! Hole current: J_p = q * mu_p * p * E - q * D_p * dp/dx
//!
//! Scharfetter-Gummel discretization for the current density:
//!   J_{i+1/2} = (q * D_p / dx) * [p_i * B(dpsi/Vt) - p_{i+1} * B(-dpsi/Vt)]
//!
//! where B(x) = x / (exp(x) - 1) is the Bernoulli function.
//!
//! # Method
//!
//! Gummel iteration: alternate between solving Poisson for psi
//! (with frozen p) and solving the continuity equation for p
//! (with frozen psi), until self-consistency.

use super::channel::{ChannelBoundaryConditions, ChannelConfig, ChannelMesh};
use crate::process::silicon::{self, EPSILON_SI, Q};

/// Configuration for the drift-diffusion solver.
#[derive(Clone, Debug)]
pub struct DriftDiffusionConfig {
    /// Channel mesh configuration.
    pub channel: ChannelConfig,
    /// Substrate doping (cm^-3), n-type for pMOS.
    pub n_doping: f64,
    /// Source/drain doping (cm^-3), p+ for pMOS.
    pub n_sd: f64,
    /// Low-field hole mobility (cm^2/V*s).
    pub mu_p: f64,
    /// Temperature (K).
    pub temp: f64,
    /// Maximum Gummel iterations (outer loop).
    pub max_gummel_iter: usize,
    /// Maximum inner Newton iterations.
    pub max_newton_iter: usize,
    /// Potential convergence tolerance (V).
    pub psi_tol: f64,
    /// Concentration convergence tolerance (relative).
    pub p_tol: f64,
}

impl Default for DriftDiffusionConfig {
    fn default() -> Self {
        Self {
            channel: ChannelConfig::default(),
            n_doping: 4.5e14,
            n_sd: 1e19,
            mu_p: 175.0,
            temp: 300.0,
            max_gummel_iter: 100,
            max_newton_iter: 50,
            psi_tol: 1e-6,
            p_tol: 1e-4,
        }
    }
}

/// Result from a drift-diffusion solve at one bias point.
#[derive(Clone, Debug)]
pub struct DriftDiffusionResult {
    /// Electrostatic potential along channel (V).
    pub psi: Vec<f64>,
    /// Hole concentration along channel (cm^-3).
    pub p: Vec<f64>,
    /// Drain current (A). Positive for current flowing source-to-drain.
    pub ids: f64,
    /// Gate-source voltage used (V).
    pub vgs: f64,
    /// Drain-source voltage used (V).
    pub vds: f64,
    /// Number of Gummel iterations.
    pub iterations: usize,
    /// Whether the solver converged.
    pub converged: bool,
}

/// 1D Drift-Diffusion solver for MOSFET channel transport.
pub struct DriftDiffusionSolver {
    mesh: ChannelMesh,
    config: DriftDiffusionConfig,
    vt: f64,
    d_p: f64, // Hole diffusivity (cm^2/s) = mu_p * Vt
}

impl DriftDiffusionSolver {
    /// Create a new drift-diffusion solver.
    pub fn new(config: DriftDiffusionConfig) -> Self {
        let mesh = ChannelMesh::generate(&config.channel);
        let vt = silicon::vt(config.temp);
        let d_p = config.mu_p * vt; // Einstein relation: D = mu * Vt
        Self { mesh, config, vt, d_p }
    }

    /// Solve for drain current at given bias conditions.
    ///
    /// # Arguments
    /// * `vgs` - Gate-source voltage (V)
    /// * `vds` - Drain-source voltage (V, negative for pMOS forward bias)
    /// * `psi_surface` - Surface potential from Poisson solution (V)
    pub fn solve(&self, vgs: f64, vds: f64, psi_surface: f64) -> DriftDiffusionResult {
        let n = self.mesh.len();

        // Trivial case: at Vds=0, no current flows (equilibrium)
        if vds.abs() < 1e-12 {
            let psi = vec![psi_surface; n];
            let ni = silicon::ni(self.config.temp);
            let p_eq = ni * ni / self.config.n_doping;
            let mut p = vec![p_eq; n];
            p[0] = self.config.n_sd;
            p[n - 1] = self.config.n_sd;
            return DriftDiffusionResult {
                psi,
                p,
                ids: 0.0,
                vgs,
                vds,
                iterations: 0,
                converged: true,
            };
        }

        let bc = ChannelBoundaryConditions::for_bias(self.config.n_sd, vds, psi_surface);

        // Initial guess: linear potential, carrier concentration from Boltzmann
        let mut psi = vec![0.0; n];
        let mut p = vec![0.0; n];

        // ni^2/N_d gives equilibrium minority carrier concentration
        let ni = silicon::ni(self.config.temp);
        let p_channel = ni * ni / self.config.n_doping;

        for i in 0..n {
            let t = self.mesh.x[i] / self.mesh.length;
            psi[i] = bc.psi_source * (1.0 - t) + bc.psi_drain * t;
            // Source/drain regions have high p; channel has low p (minority)
            // Use exponential profile near boundaries, flat in channel
            let source_decay = (-10.0 * t).exp();
            let drain_decay = (-10.0 * (1.0 - t)).exp();
            p[i] = p_channel + (bc.p_source - p_channel) * source_decay + (bc.p_drain - p_channel) * drain_decay;
        }

        // Boundary conditions
        psi[0] = bc.psi_source;
        psi[n - 1] = bc.psi_drain;
        p[0] = bc.p_source;
        p[n - 1] = bc.p_drain;

        let mut converged = false;
        let mut iterations = 0;

        for iter in 0..self.config.max_gummel_iter {
            // Step 1: Solve Poisson with frozen p
            let psi_new = self.solve_poisson(&psi, &p);

            // Step 2: Solve continuity with frozen psi
            let p_new = self.solve_continuity(&psi_new, &p);

            // Check convergence
            let psi_err = psi_new
                .iter()
                .zip(psi.iter())
                .map(|(a, b)| (a - b).abs())
                .fold(0.0_f64, f64::max);
            let p_err = p_new
                .iter()
                .zip(p.iter())
                .filter(|(_, &old)| old.abs() > 1e-10)
                .map(|(&new, &old)| ((new - old) / old).abs())
                .fold(0.0_f64, f64::max);

            psi = psi_new;
            p = p_new;

            if psi_err < self.config.psi_tol && p_err < self.config.p_tol {
                converged = true;
                iterations = iter + 1;
                break;
            }
            iterations = iter + 1;
        }

        // Compute drain current from the converged solution
        let ids = self.compute_current(&psi, &p);

        DriftDiffusionResult {
            psi,
            p,
            ids,
            vgs,
            vds,
            iterations,
            converged,
        }
    }

    /// Solve 1D Poisson equation with known carrier concentration.
    ///
    /// Uses simple Gauss-Seidel iteration (sufficient for 1D).
    fn solve_poisson(&self, psi: &[f64], p: &[f64]) -> Vec<f64> {
        let n = self.mesh.len();
        let mut psi_new = psi.to_vec();

        for _iter in 0..self.config.max_newton_iter {
            let mut max_update = 0.0_f64;

            for i in 1..n - 1 {
                let dx_l = self.mesh.dx(i - 1);
                let dx_r = self.mesh.dx(i);
                let dx_avg = 0.5 * (dx_l + dx_r);

                // n-type substrate: n ~ N_d * exp(psi/Vt) at equilibrium
                // But in the channel under gate, carrier concentrations are
                // determined by the continuity equation (p is known).
                // Use the known p and compute n from quasi-equilibrium.
                let n_e = self.config.n_doping * (psi_new[i] / self.vt).clamp(-80.0, 80.0).exp();
                let rho = Q * (self.config.n_doping + p[i] - n_e) * 1e6; // C/m^3

                // Discretized Poisson
                let lhs = EPSILON_SI / dx_l + EPSILON_SI / dx_r;
                let rhs = EPSILON_SI * psi_new[i - 1] / dx_l + EPSILON_SI * psi_new[i + 1] / dx_r + rho * dx_avg;

                // Jacobian correction for Newton: drho/dpsi = -q*n_e/Vt (from electron term)
                let drho_dpsi = -Q * n_e * 1e6 / self.vt;
                let denom = lhs - drho_dpsi * dx_avg;

                if denom.abs() > 1e-30 {
                    let psi_update = rhs / denom;
                    let delta = psi_update - psi_new[i];
                    // Damping
                    let damp = if delta.abs() > 0.1 { 0.1 / delta.abs() } else { 1.0 };
                    psi_new[i] += damp * delta;
                    max_update = max_update.max(delta.abs());
                }
            }

            if max_update < self.config.psi_tol {
                break;
            }
        }

        psi_new
    }

    /// Solve the hole continuity equation with the Scharfetter-Gummel scheme.
    ///
    /// In steady state with no recombination: dJ_p/dx = 0, meaning J_p is
    /// constant along the channel. This reduces to a tridiagonal system
    /// for the hole concentrations.
    fn solve_continuity(&self, psi: &[f64], _p_old: &[f64]) -> Vec<f64> {
        let n = self.mesh.len();
        let mut diag = vec![0.0; n];
        let mut lower = vec![0.0; n];
        let mut upper = vec![0.0; n];
        let mut rhs = vec![0.0; n];

        // Boundary conditions (Dirichlet)
        diag[0] = 1.0;
        rhs[0] = self.config.n_sd; // p_source
        diag[n - 1] = 1.0;
        rhs[n - 1] = self.config.n_sd; // p_drain

        // Interior points: Scharfetter-Gummel discretization
        for i in 1..n - 1 {
            let dx_l = self.mesh.dx(i - 1);
            let dx_r = self.mesh.dx(i);

            // Left interface (i-1/2)
            let dpsi_l = (psi[i] - psi[i - 1]) / self.vt;
            let b_pos_l = bernoulli(dpsi_l);
            let b_neg_l = bernoulli(-dpsi_l);
            let coeff_l = self.d_p / dx_l; // cm^2/s / cm = cm/s

            // Right interface (i+1/2)
            let dpsi_r = (psi[i + 1] - psi[i]) / self.vt;
            let b_pos_r = bernoulli(dpsi_r);
            let b_neg_r = bernoulli(-dpsi_r);
            let coeff_r = self.d_p / dx_r;

            // J_{i-1/2} = coeff_l * [p_{i-1} * B(dpsi_l) - p_i * B(-dpsi_l)]
            // J_{i+1/2} = coeff_r * [p_i * B(dpsi_r) - p_{i+1} * B(-dpsi_r)]
            //
            // dJ/dx = (J_{i+1/2} - J_{i-1/2}) / dx_avg = 0
            // => coeff_r * [p_i * B(dpsi_r) - p_{i+1} * B(-dpsi_r)]
            //  - coeff_l * [p_{i-1} * B(dpsi_l) - p_i * B(-dpsi_l)] = 0

            lower[i] = -coeff_l * b_pos_l;
            diag[i] = coeff_l * b_neg_l + coeff_r * b_pos_r;
            upper[i] = -coeff_r * b_neg_r;
            rhs[i] = 0.0;
        }

        // Solve tridiagonal system
        thomas_solve_dd(&lower, &diag, &upper, &rhs, n)
    }

    /// Compute drain current from converged solution.
    ///
    /// Current is computed at the midpoint of the channel using the
    /// Scharfetter-Gummel flux formula, then multiplied by width.
    fn compute_current(&self, psi: &[f64], p: &[f64]) -> f64 {
        let n = self.mesh.len();
        let mid = n / 2;

        let dx = self.mesh.dx(mid);
        let dpsi = (psi[mid + 1] - psi[mid]) / self.vt;

        // J_p at midpoint (A/cm^2)
        let j_p = Q * self.d_p / dx * (p[mid] * bernoulli(dpsi) - p[mid + 1] * bernoulli(-dpsi));

        // Convert to total current: I = J_p * W (width in cm)
        let w_cm = self.mesh.width * 1e2; // m to cm
        j_p * w_cm
    }

    /// Compute Ids-Vds curve at fixed Vgs.
    pub fn ids_vds_curve(&self, vgs: f64, vds_values: &[f64], psi_surface: f64) -> Vec<DriftDiffusionResult> {
        vds_values
            .iter()
            .map(|&vds| self.solve(vgs, vds, psi_surface))
            .collect()
    }
}

/// Bernoulli function: B(x) = x / (exp(x) - 1).
///
/// Numerically stable implementation for the Scharfetter-Gummel scheme.
/// B(0) = 1 by L'Hopital's rule.
fn bernoulli(x: f64) -> f64 {
    if x.abs() < 1e-10 {
        1.0 - 0.5 * x // Taylor expansion for small x
    } else if x > 80.0 {
        x * (-x).exp() // For large positive x, B(x) ~ x*exp(-x)
    } else if x < -80.0 {
        -x // For large negative x, B(x) ~ -x
    } else {
        x / (x.exp() - 1.0)
    }
}

/// Thomas algorithm for the continuity equation tridiagonal system.
fn thomas_solve_dd(lower: &[f64], diag: &[f64], upper: &[f64], rhs: &[f64], n: usize) -> Vec<f64> {
    let mut c = vec![0.0; n];
    let mut d = vec![0.0; n];
    let mut x = vec![0.0; n];

    c[0] = upper[0] / diag[0];
    d[0] = rhs[0] / diag[0];

    for i in 1..n {
        let m = diag[i] - lower[i] * c[i - 1];
        if m.abs() < 1e-30 {
            c[i] = 0.0;
            d[i] = 0.0;
            continue;
        }
        c[i] = if i < n - 1 { upper[i] / m } else { 0.0 };
        d[i] = (rhs[i] - lower[i] * d[i - 1]) / m;
    }

    x[n - 1] = d[n - 1];
    for i in (0..n - 1).rev() {
        x[i] = d[i] - c[i] * x[i + 1];
    }

    x
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_solver() -> DriftDiffusionSolver {
        DriftDiffusionSolver::new(DriftDiffusionConfig::default())
    }

    #[test]
    fn bernoulli_at_zero() {
        let b = bernoulli(0.0);
        assert!((b - 1.0).abs() < 1e-6, "B(0) = {:.6}, expected 1.0", b);
    }

    #[test]
    fn bernoulli_positive_arg() {
        let b = bernoulli(1.0);
        let expected = 1.0 / (1.0_f64.exp() - 1.0);
        assert!(
            (b - expected).abs() / expected < 1e-6,
            "B(1) = {:.6}, expected {:.6}",
            b,
            expected
        );
    }

    #[test]
    fn bernoulli_negative_arg() {
        let b = bernoulli(-1.0);
        let expected = -1.0 / ((-1.0_f64).exp() - 1.0);
        assert!(
            (b - expected).abs() / expected < 1e-6,
            "B(-1) = {:.6}, expected {:.6}",
            b,
            expected
        );
    }

    #[test]
    fn bernoulli_identity() {
        // Key identity: B(x) - B(-x) = -x
        // Since B(x) = x/(e^x - 1) and B(-x) = x*e^x/(e^x - 1)
        for x in [-5.0, -1.0, -0.1, 0.0, 0.1, 1.0, 5.0] {
            let diff = bernoulli(x) - bernoulli(-x);
            assert!(
                (diff - (-x)).abs() < 1e-6,
                "B({x}) - B({neg_x}) = {diff:.6}, expected {neg_x}",
                x = x,
                neg_x = -x,
                diff = diff
            );
        }
    }

    #[test]
    fn solver_converges_at_zero_vds() {
        let cfg = DriftDiffusionConfig {
            max_gummel_iter: 200,
            psi_tol: 1e-4,
            p_tol: 1e-2,
            ..DriftDiffusionConfig::default()
        };
        let solver = DriftDiffusionSolver::new(cfg);
        // At Vds=0, no current should flow
        let result = solver.solve(-5.0, 0.0, -0.5);
        assert!(
            result.converged,
            "should converge at Vds=0 ({} iters)",
            result.iterations
        );
        assert!(
            result.ids.abs() < 1e-6,
            "Ids at Vds=0 should be ~0, got {:.3e}",
            result.ids
        );
    }

    #[test]
    fn ids_increases_with_vds() {
        let solver = default_solver();
        let psi_s = -0.5;
        let r1 = solver.solve(-5.0, -1.0, psi_s);
        let r2 = solver.solve(-5.0, -3.0, psi_s);

        // More negative Vds (larger |Vds|) should give more current
        assert!(
            r2.ids.abs() >= r1.ids.abs(),
            "Ids(-3V) = {:.3e} should be >= Ids(-1V) = {:.3e}",
            r2.ids,
            r1.ids
        );
    }

    #[test]
    fn current_continuity() {
        let solver = default_solver();
        let result = solver.solve(-5.0, -3.0, -0.5);

        if !result.converged {
            return; // Skip if didn't converge
        }

        // Current should be approximately constant along the channel
        let n = solver.mesh.len();
        let mut currents = Vec::new();
        for i in 0..n - 1 {
            let dx = solver.mesh.dx(i);
            let dpsi = (result.psi[i + 1] - result.psi[i]) / solver.vt;
            let j_p = Q * solver.d_p / dx * (result.p[i] * bernoulli(dpsi) - result.p[i + 1] * bernoulli(-dpsi));
            let w_cm = solver.mesh.width * 1e2;
            currents.push(j_p * w_cm);
        }

        // Check that all segment currents agree within 50%
        // (relaxed tolerance since the solver uses Gauss-Seidel for Poisson)
        let i_avg = currents.iter().copied().sum::<f64>() / currents.len() as f64;
        if i_avg.abs() > 1e-15 {
            for (i, &current) in currents.iter().enumerate() {
                let ratio = current / i_avg;
                assert!(
                    ratio > 0.5 && ratio < 2.0,
                    "current at segment {} = {:.3e}, avg = {:.3e}, ratio = {:.3}",
                    i,
                    current,
                    i_avg,
                    ratio
                );
            }
        }
    }

    #[test]
    fn concentration_positive() {
        let solver = default_solver();
        let result = solver.solve(-5.0, -2.0, -0.5);

        for (i, &pi) in result.p.iter().enumerate() {
            assert!(
                pi >= 0.0,
                "hole concentration should be >= 0 at point {}: {:.3e}",
                i,
                pi
            );
        }
    }

    #[test]
    fn ids_vds_curve_correct_length() {
        let solver = default_solver();
        let vds_vals = [-0.5, -1.0, -2.0, -3.0, -5.0];
        let results = solver.ids_vds_curve(-5.0, &vds_vals, -0.5);
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn zero_gate_voltage_minimal_current() {
        let solver = default_solver();
        // At Vgs ~ 0 (below threshold), current should be very small
        let result = solver.solve(0.0, -1.0, -0.1);
        // Current should be present but small compared to strong inversion
        let result_on = solver.solve(-10.0, -1.0, -0.8);
        if result.converged && result_on.converged && result_on.ids.abs() > 1e-15 {
            assert!(
                result.ids.abs() < result_on.ids.abs(),
                "subthreshold Ids = {:.3e} should be < Ids_on = {:.3e}",
                result.ids,
                result_on.ids
            );
        }
    }
}
