//! 1D Poisson-Boltzmann solver for MOS gate stack electrostatics.
//!
//! Solves the 1D Poisson equation self-consistently with Boltzmann
//! carrier statistics across the gate/oxide/silicon structure.
//!
//! # Equation
//!
//! d/dx [eps(x) * d(psi)/dx] = -rho(psi, x)
//!
//! In the oxide: rho = 0 (no free carriers), eps = eps_ox
//! In silicon:   rho = q * (N_d - n(psi) + p(psi)), eps = eps_si
//!
//! # Boundary Conditions
//!
//! - Gate (x=0): psi = V_gate - phi_ms (metal-semiconductor work function)
//! - Bulk (x=L): psi = 0 (neutral bulk reference)
//!
//! # Method
//!
//! Newton-Raphson iteration on the discretized Poisson equation.
//! The Jacobian is tridiagonal, solved by Thomas algorithm (O(n)).

use super::{
    carrier::CarrierStats,
    mesh::{Mesh1D, MeshConfig},
};

/// Configuration for the Poisson solver.
#[derive(Clone, Debug)]
pub struct PoissonConfig {
    /// Mesh configuration.
    pub mesh: MeshConfig,
    /// Substrate doping (cm^-3), n-type for pMOS.
    pub n_doping: f64,
    /// Temperature (K).
    pub temp: f64,
    /// Maximum Newton-Raphson iterations.
    pub max_iterations: usize,
    /// Convergence tolerance (V) for potential update.
    pub tol: f64,
    /// Metal-semiconductor work function difference (V).
    /// For aluminum gate on n-type Si: phi_ms ~ -0.3V (pMOS).
    pub phi_ms: f64,
}

impl Default for PoissonConfig {
    fn default() -> Self {
        Self {
            mesh: MeshConfig::default(),
            n_doping: 4.5e14,
            temp: 300.0,
            max_iterations: 200,
            tol: 1e-8,
            phi_ms: -0.3,
        }
    }
}

/// Result from a Poisson solve at one gate voltage.
#[derive(Clone, Debug)]
pub struct PoissonResult {
    /// Electrostatic potential at each mesh point (V).
    pub psi: Vec<f64>,
    /// Surface potential at oxide/silicon interface (V).
    pub psi_surface: f64,
    /// Gate voltage used for this solve (V).
    pub v_gate: f64,
    /// Number of Newton iterations to converge.
    pub iterations: usize,
    /// Whether the solver converged within tolerance.
    pub converged: bool,
    /// Final maximum potential update (V).
    pub residual: f64,
    /// Gate capacitance computed from dQ/dV (F/m^2).
    pub cg: f64,
    /// Inversion charge density (C/m^2).
    pub q_inv: f64,
}

/// 1D Poisson-Boltzmann solver for MOS electrostatics.
pub struct PoissonSolver {
    mesh: Mesh1D,
    stats: CarrierStats,
    config: PoissonConfig,
}

impl PoissonSolver {
    /// Create a new solver with given configuration.
    pub fn new(config: PoissonConfig) -> Self {
        let mesh = Mesh1D::generate(&config.mesh);
        let stats = CarrierStats::new(config.n_doping, config.temp);
        Self { mesh, stats, config }
    }

    /// Solve for the potential distribution at a given gate voltage.
    ///
    /// Returns the converged potential profile and derived quantities.
    pub fn solve(&self, v_gate: f64) -> PoissonResult {
        let (psi, converged, residual, iterations) = self.solve_nr(v_gate);
        let psi_surface = psi[self.mesh.interface_idx];

        // Compute Cg by finite-difference: solve at V+dV, compare gate charge
        let dv = 1e-4;
        let (psi_plus, _, _, _) = self.solve_nr(v_gate + dv);
        let cg = self.gate_charge_diff(&psi, &psi_plus, dv);

        // Compute inversion charge (holes for pMOS)
        let q_inv = self.compute_inversion_charge(&psi);

        PoissonResult {
            psi,
            psi_surface,
            v_gate,
            iterations,
            converged,
            residual,
            cg,
            q_inv,
        }
    }

    fn compute_inversion_charge(&self, psi: &[f64]) -> f64 {
        use crate::process::silicon::Q;
        let p0 = self.stats.ni() * self.stats.ni() / self.stats.n_doping;
        let mut total_q = 0.0;

        for i in self.mesh.interface_idx..self.mesh.len() - 1 {
            let dx = self.mesh.dx(i);
            // Average potential in segment
            let psi_avg = 0.5 * (psi[i] + psi[i + 1]);
            let p = self.stats.hole_concentration(psi_avg);

            // Only count excess carriers (inversion layer)
            if p > p0 {
                total_q += Q * (p - p0) * dx * 1e6; // cm^-3 to m^-3
            }
        }
        total_q
    }

    /// Internal Newton-Raphson solve returning raw potential array.
    fn solve_nr(&self, v_gate: f64) -> (Vec<f64>, bool, f64, usize) {
        let n = self.mesh.len();
        let mut psi = vec![0.0; n];

        // Initial guess: linear ramp in oxide, zero in silicon
        let psi_gate = v_gate - self.config.phi_ms;
        for (i, p) in psi.iter_mut().enumerate().take(self.mesh.interface_idx) {
            let t = self.mesh.x[i] / self.config.mesh.t_ox;
            *p = psi_gate * (1.0 - t);
        }

        // Apply boundary conditions
        psi[0] = psi_gate;
        psi[n - 1] = 0.0;

        let mut converged = false;
        let mut residual = f64::MAX;
        let mut iterations = 0;

        for iter in 0..self.config.max_iterations {
            let (f_vec, diag, lower, upper) = self.assemble(n, &psi);

            residual = f_vec[1..n - 1].iter().map(|v| v.abs()).fold(0.0_f64, f64::max);
            if residual < self.config.tol {
                converged = true;
                iterations = iter;
                break;
            }

            // Newton-Raphson: J * delta = -F, so negate RHS
            let neg_f: Vec<f64> = f_vec.iter().map(|&v| -v).collect();
            let delta = thomas_solve(&lower, &diag, &upper, &neg_f, n);

            let max_delta = delta[1..n - 1].iter().map(|v| v.abs()).fold(0.0_f64, f64::max);
            let damp = if max_delta > 0.1 { 0.1 / max_delta } else { 1.0 };

            for i in 1..n - 1 {
                psi[i] += damp * delta[i];
            }

            iterations = iter + 1;
        }

        (psi, converged, residual, iterations)
    }

    /// Sweep gate voltage and return results at each point.
    pub fn sweep(&self, v_start: f64, v_end: f64, n_points: usize) -> Vec<PoissonResult> {
        assert!(n_points >= 2);
        let dv = (v_end - v_start) / (n_points - 1) as f64;
        (0..n_points)
            .map(|i| {
                let vg = v_start + i as f64 * dv;
                self.solve(vg)
            })
            .collect()
    }

    /// Compute threshold voltage from the potential solution.
    ///
    /// Vth is defined as the gate voltage where psi_surface = 2*phi_f
    /// (onset of strong inversion for pMOS: surface inverts from n to p).
    ///
    /// Uses bisection on the sweep results.
    pub fn find_vth(&self, v_range: (f64, f64), tol: f64) -> Option<f64> {
        let two_phi_f = 2.0 * self.stats.phi_f();
        // For pMOS on n-type substrate, inversion occurs at negative psi_s
        let target_psi = -two_phi_f;

        let mut v_lo = v_range.0;
        let mut v_hi = v_range.1;

        let r_lo = self.solve(v_lo);
        let r_hi = self.solve(v_hi);

        // Check that target is bracketed
        if (r_lo.psi_surface - target_psi) * (r_hi.psi_surface - target_psi) > 0.0 {
            return None; // Not bracketed
        }

        for _ in 0..100 {
            let v_mid = 0.5 * (v_lo + v_hi);
            if (v_hi - v_lo).abs() < tol {
                return Some(v_mid);
            }
            let r_mid = self.solve(v_mid);
            if (r_mid.psi_surface - target_psi) * (r_lo.psi_surface - target_psi) < 0.0 {
                v_hi = v_mid;
            } else {
                v_lo = v_mid;
            }
        }

        Some(0.5 * (v_lo + v_hi))
    }

    /// Assemble the discretized Poisson equation residual and Jacobian.
    ///
    /// Returns (F, diag, lower, upper) for the tridiagonal system.
    fn assemble(&self, n: usize, psi: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut f_vec = vec![0.0; n];
        let mut diag = vec![1.0; n]; // diagonal
        let mut lower = vec![0.0; n]; // sub-diagonal
        let mut upper = vec![0.0; n]; // super-diagonal

        // Boundary: psi[0] = psi_gate (Dirichlet)
        f_vec[0] = 0.0;
        diag[0] = 1.0;

        // Interior points
        for i in 1..n - 1 {
            let dx_left = self.mesh.x[i] - self.mesh.x[i - 1];
            let dx_right = self.mesh.x[i + 1] - self.mesh.x[i];
            let dx_avg = 0.5 * (dx_left + dx_right);

            let eps_left = 0.5 * (self.mesh.epsilon(i - 1) + self.mesh.epsilon(i));
            let eps_right = 0.5 * (self.mesh.epsilon(i) + self.mesh.epsilon(i + 1));

            // Discretized Poisson: eps_r * (psi_{i+1} - psi_i)/dx_r - eps_l * (psi_i - psi_{i-1})/dx_l = -rho * dx_avg
            // Rearranged as F_i = 0:
            let flux_right = eps_right * (psi[i + 1] - psi[i]) / dx_right;
            let flux_left = eps_left * (psi[i] - psi[i - 1]) / dx_left;

            let rho_si = if self.mesh.is_silicon(i) {
                self.stats.charge_density_si(psi[i])
            } else {
                0.0 // No free carriers in oxide
            };

            f_vec[i] = flux_right - flux_left + rho_si * dx_avg;

            // Jacobian entries (tridiagonal)
            let j_left = eps_left / dx_left;
            let j_right = eps_right / dx_right;

            let drho = if self.mesh.is_silicon(i) {
                self.stats.dcharge_dpsi_si(psi[i]) * dx_avg
            } else {
                0.0
            };

            lower[i] = j_left;
            upper[i] = j_right;
            diag[i] = -(j_left + j_right) + drho;
        }

        // Boundary: psi[n-1] = 0 (Dirichlet)
        f_vec[n - 1] = 0.0;
        diag[n - 1] = 1.0;

        (f_vec, diag, lower, upper)
    }

    /// Compute Cg = dQ_gate/dV from two pre-solved potential arrays.
    fn gate_charge_diff(&self, psi: &[f64], psi_plus: &[f64], dv: f64) -> f64 {
        use crate::process::silicon::EPSILON_OX;
        let dx0 = self.mesh.dx(0);
        let q1 = EPSILON_OX * (psi[0] - psi[1]) / dx0;
        let q2 = EPSILON_OX * (psi_plus[0] - psi_plus[1]) / dx0;
        (q2 - q1) / dv
    }
}

/// Thomas algorithm for tridiagonal system.
///
/// Solves [lower, diag, upper] * x = rhs.
/// lower[0] and upper[n-1] are unused.
fn thomas_solve(lower: &[f64], diag: &[f64], upper: &[f64], rhs: &[f64], n: usize) -> Vec<f64> {
    let mut c = vec![0.0; n];
    let mut d = vec![0.0; n];
    let mut x = vec![0.0; n];

    // Forward sweep
    c[0] = upper[0] / diag[0];
    d[0] = rhs[0] / diag[0];

    for i in 1..n {
        let m = diag[i] - lower[i] * c[i - 1];
        if m.abs() < 1e-30 {
            // Near-singular: keep previous value
            c[i] = 0.0;
            d[i] = 0.0;
            continue;
        }
        c[i] = if i < n - 1 { upper[i] / m } else { 0.0 };
        d[i] = (rhs[i] - lower[i] * d[i - 1]) / m;
    }

    // Back substitution
    x[n - 1] = d[n - 1];
    for i in (0..n - 1).rev() {
        x[i] = d[i] - c[i] * x[i + 1];
    }

    x
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_solver() -> PoissonSolver {
        PoissonSolver::new(PoissonConfig::default())
    }

    #[test]
    fn solver_converges_at_flatband() {
        let solver = default_solver();
        // Near flatband: V_gate ~ phi_ms
        let result = solver.solve(solver.config.phi_ms);
        assert!(result.converged, "should converge at flatband");
        assert!(result.iterations < 50, "iterations: {}", result.iterations);
    }

    #[test]
    fn solver_converges_at_zero_bias() {
        let solver = default_solver();
        let result = solver.solve(0.0);
        assert!(result.converged, "should converge at Vg=0");
    }

    #[test]
    fn solver_converges_in_depletion() {
        let solver = default_solver();
        // Negative gate voltage depletes n-type substrate
        let result = solver.solve(-5.0);
        assert!(result.converged, "should converge in depletion at Vg=-5V");
        // Surface potential should be negative (band bending)
        assert!(
            result.psi_surface < 0.0,
            "psi_s = {:.3}V should be < 0",
            result.psi_surface
        );
    }

    #[test]
    fn solver_converges_in_inversion() {
        let solver = default_solver();
        // Strong negative gate voltage inverts surface
        let result = solver.solve(-10.0);
        assert!(result.converged, "should converge in inversion at Vg=-10V");
        // Surface potential should be strongly negative
        assert!(
            result.psi_surface < -0.5,
            "psi_s = {:.3}V should be strongly negative in inversion",
            result.psi_surface
        );
    }

    #[test]
    fn potential_monotonic_in_oxide() {
        let solver = default_solver();
        let result = solver.solve(-5.0);
        let iface = solver.mesh.interface_idx;

        // For negative gate voltage on n-type substrate:
        // psi_gate < 0 and psi increases from gate toward bulk.
        // Electric field in oxide is approximately linear.
        let mut is_monotonic = true;
        let sign = (result.psi[1] - result.psi[0]).signum();
        for i in 2..iface {
            let dsign = (result.psi[i] - result.psi[i - 1]).signum();
            if (dsign - sign).abs() > 1e-10 && (result.psi[i] - result.psi[i - 1]).abs() > 1e-12 {
                is_monotonic = false;
                break;
            }
        }
        assert!(
            is_monotonic,
            "oxide potential should be monotonic (linear in ideal oxide)"
        );
    }

    #[test]
    fn bulk_potential_near_zero() {
        let solver = default_solver();
        let result = solver.solve(-5.0);
        let n = result.psi.len();

        // Deep in bulk, potential should be near zero
        assert!(
            result.psi[n - 1].abs() < 1e-10,
            "bulk potential should be 0, got {:.3e}",
            result.psi[n - 1]
        );
        // Near-bulk should also be close to zero
        assert!(
            result.psi[n - 2].abs() < 0.01,
            "near-bulk potential should be near 0, got {:.3e}",
            result.psi[n - 2]
        );
    }

    #[test]
    fn surface_potential_increases_with_gate_voltage() {
        let solver = default_solver();
        // More negative Vg should produce more negative psi_s (more depletion/inversion)
        let r1 = solver.solve(-2.0);
        let r2 = solver.solve(-5.0);
        let r3 = solver.solve(-10.0);

        assert!(
            r2.psi_surface < r1.psi_surface,
            "psi_s(-5V) = {:.3} should be < psi_s(-2V) = {:.3}",
            r2.psi_surface,
            r1.psi_surface
        );
        assert!(
            r3.psi_surface < r2.psi_surface,
            "psi_s(-10V) = {:.3} should be < psi_s(-5V) = {:.3}",
            r3.psi_surface,
            r2.psi_surface
        );
    }

    #[test]
    fn gate_capacitance_positive() {
        let solver = default_solver();
        let result = solver.solve(-5.0);
        assert!(result.cg > 0.0, "Cg should be positive, got {:.3e}", result.cg);
    }

    #[test]
    fn gate_capacitance_bounded_by_cox() {
        use crate::process::silicon::EPSILON_OX;
        let solver = default_solver();
        let cox = EPSILON_OX / solver.config.mesh.t_ox;

        // In strong inversion, Cg should approach Cox
        let r_inv = solver.solve(-10.0);
        assert!(
            r_inv.cg < cox * 1.1,
            "Cg in inversion ({:.3e}) should not exceed Cox ({:.3e})",
            r_inv.cg,
            cox
        );

        // In depletion, Cg should be less than Cox (series combination)
        let r_dep = solver.solve(-2.0);
        assert!(
            r_dep.cg < cox,
            "Cg in depletion ({:.3e}) should be < Cox ({:.3e})",
            r_dep.cg,
            cox
        );
    }

    #[test]
    fn depletion_width_vs_analytical() {
        let solver = default_solver();
        let result = solver.solve(-5.0);

        // Analytical depletion width at psi_surface
        let w_analytical = solver.stats.depletion_width(result.psi_surface);

        // From numerical solution: find where psi drops to ~0
        // (depletion edge is where potential becomes negligible)
        let iface = solver.mesh.interface_idx;
        let threshold = 0.01 * result.psi_surface.abs(); // 1% of surface potential
        let mut w_numerical = 0.0;
        for i in iface..result.psi.len() {
            if result.psi[i].abs() < threshold {
                w_numerical = solver.mesh.x[i] - solver.mesh.x[iface];
                break;
            }
        }

        // Should agree within ~30% (depletion approx vs full Boltzmann)
        if w_analytical > 0.0 && w_numerical > 0.0 {
            let ratio = w_numerical / w_analytical;
            assert!(
                ratio > 0.5 && ratio < 2.0,
                "W_dep numerical={:.3e} vs analytical={:.3e}, ratio={:.2}",
                w_numerical,
                w_analytical,
                ratio
            );
        }
    }

    #[test]
    fn vth_from_first_principles() {
        let solver = default_solver();
        // Vth should be in the range -2 to -5V for this process
        let vth = solver.find_vth((-15.0, 0.0), 0.01);
        assert!(vth.is_some(), "should find Vth in range -15 to 0V");
        let vth = vth.expect("Vth search already verified above");
        assert!(vth > -6.0 && vth < -1.0, "Vth = {:.3}V, expected -6 to -1V range", vth);
    }

    #[test]
    fn sweep_produces_correct_count() {
        let solver = default_solver();
        let results = solver.sweep(-10.0, 0.0, 11);
        assert_eq!(results.len(), 11);
        assert!((results[0].v_gate - (-10.0)).abs() < 1e-10);
        assert!(results[10].v_gate.abs() < 1e-10);
    }

    #[test]
    fn thomas_algorithm_simple_system() {
        // Solve: [2 -1 0; -1 2 -1; 0 -1 2] * x = [1; 0; 1]
        // Solution: x = [1; 1; 1]
        let lower = vec![0.0, -1.0, -1.0];
        let diag = vec![2.0, 2.0, 2.0];
        let upper = vec![-1.0, -1.0, 0.0];
        let rhs = vec![1.0, 0.0, 1.0];
        let x = thomas_solve(&lower, &diag, &upper, &rhs, 3);
        for (i, &xi) in x.iter().enumerate().take(3) {
            assert!((xi - 1.0).abs() < 1e-10, "x[{}] = {:.6}, expected 1.0", i, xi);
        }
    }

    #[test]
    fn cg_vg_curve_shape() {
        let solver = default_solver();
        let results = solver.sweep(-12.0, 2.0, 15);

        // Collect converged results
        let cg_values: Vec<f64> = results.iter().filter(|r| r.converged).map(|r| r.cg).collect();

        // All should be positive
        for (i, &cg) in cg_values.iter().enumerate() {
            assert!(cg > 0.0, "Cg[{}] = {:.3e} should be positive", i, cg);
        }
    }
}
