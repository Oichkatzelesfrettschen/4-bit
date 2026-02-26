//! TCAD-to-Circuit Bridge
//!
//! Provides a high-level interface for circuit simulators to query device physics.
//! This module bridges the 1D mesh solvers (Poisson, Drift-Diffusion) with nodal
//! analysis by computing currents and Jacobian conductances from process parameters.

use crate::{
    circuit::graph::TransistorKind,
    process::ProcessParams,
    tcad::{
        channel::ChannelConfig,
        drift_diffusion::DriftDiffusionConfig,
        mesh::MeshConfig,
        poisson::{PoissonConfig, PoissonSolver},
    },
};

/// Result of a device-level physics solve
#[derive(Clone, Debug, Default)]
pub struct DeviceResult {
    /// Drain-source current (A)
    pub ids: f64,
    /// Transconductance (dIds/dVgs) (S)
    pub gm: f64,
    /// Output conductance (dIds/dVds) (S)
    pub gds: f64,
    /// Body transconductance (dIds/dVbs) (S)
    pub gmb: f64,
    /// Gate capacitance (F)
    pub cg: f64,
}

/// Bridge for computing device physics from process parameters
pub struct TCADBridge {
    /// Simulation configuration derived from process parameters
    p_config: PoissonConfig,
    dd_config: DriftDiffusionConfig,
    /// Target thresholds for mode switching
    vth_enh: f64,
    vth_dep: f64,
    /// Die-level stochastic seed for process variation
    noise_seed: u64,
}

impl TCADBridge {
    /// Create a new bridge from process parameters
    pub fn new(params: &ProcessParams) -> Self {
        Self::with_seed(params, 0)
    }

    /// Create a new bridge with a specific noise seed for process variation
    pub fn with_seed(params: &ProcessParams, seed: u64) -> Self {
        // Derive TCAD configurations from ProcessParams
        let mesh_config = MeshConfig {
            t_ox: params.t_ox,
            si_depth: 1e-6, // 1um bulk depth
            n_oxide: 20,
            n_silicon: 50,
            si_grading: 3.0,
        };

        let channel_config = ChannelConfig {
            length: params.l_min,
            width: 10e-6, // Unit width for base config
            n_points: 50,
            grading: 2.0,
        };

        let p_config = PoissonConfig {
            mesh: mesh_config,
            n_doping: params.n_sub,
            temp: params.temp,
            max_iterations: 100,
            tol: 1e-6,
            phi_ms: -0.3, // Aluminum gate on n-Si (approx enhancement base)
        };

        let dd_config = DriftDiffusionConfig {
            channel: channel_config,
            n_doping: params.n_sub,
            n_sd: params.n_sd,
            mu_p: params.mu_0,
            temp: params.temp,
            max_gummel_iter: 50,
            max_newton_iter: 20,
            psi_tol: 1e-6,
            p_tol: 1e-6,
        };

        Self {
            p_config,
            dd_config,
            vth_enh: params.vth_enhancement,
            vth_dep: params.vth_depletion,
            noise_seed: seed,
        }
    }

    /// Solve for device characteristics at given operating point
    pub fn solve_device(&self, v_g: f64, v_s: f64, v_d: f64, _v_b: f64, w: f64, kind: TransistorKind) -> DeviceResult {
        // Perturb doping based on seed (simulating local lattice frustration)
        let mut p_cfg = self.p_config.clone();
        if self.noise_seed != 0 {
            let variation = ((self.noise_seed as f64).sin() * 0.05) + 1.0; // +/- 5% variation
            p_cfg.n_doping *= variation;
        }

        // Adjust phi_ms for depletion mode to shift Vth
        if kind == TransistorKind::Depletion {
            p_cfg.phi_ms += self.vth_dep - self.vth_enh;
        }

        // pMOS: Carrier mobility (holes)
        let mu = self.dd_config.mu_p * 1e-4; // cm^2/Vs to m^2/Vs
        let l = self.dd_config.channel.length;

        // Use the Pao-Sah Integral Model: Ids = (W/L) * mu * integral(Qinv dV)
        // 1. Solve Poisson at source end (V_channel = Vs)
        let p_solver = PoissonSolver::new(p_cfg.clone());
        let res_s = p_solver.solve(v_g - v_s);
        if !res_s.converged || res_s.q_inv.is_nan() {
            return DeviceResult::default();
        }
        let q_inv_s = res_s.q_inv;

        // 2. Solve Poisson at drain end (V_channel = Vd)
        let res_d = p_solver.solve(v_g - v_d);
        if !res_d.converged || res_d.q_inv.is_nan() {
            return DeviceResult::default();
        }
        let q_inv_d = res_d.q_inv;

        // 3. Integrate Qinv(V)
        let ids = (w / l) * mu * (q_inv_s + q_inv_d) * 0.5 * (v_s - v_d);

        // 4. Compute small-signal parameters via finite difference
        let dv = 1e-3; // 1mV perturbation

        let res_s_plus = p_solver.solve(v_g + dv - v_s);
        let res_d_plus = p_solver.solve(v_g + dv - v_d);
        let ids_g_plus = (w / l) * mu * (res_s_plus.q_inv + res_d_plus.q_inv) * 0.5 * (v_s - v_d);
        let gm = (ids_g_plus - ids).abs() / dv;

        let res_d_vplus = p_solver.solve(v_g - (v_d + dv));
        let ids_d_plus = (w / l) * mu * (q_inv_s + res_d_vplus.q_inv) * 0.5 * (v_s - (v_d + dv));
        let gds = (ids_d_plus - ids).abs() / dv;

        DeviceResult {
            ids,
            gm,
            gds,
            gmb: 0.0,
            cg: res_s.cg * w * l,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_instantiation() {
        let params = ProcessParams::default();
        let bridge = TCADBridge::new(&params);
        assert_eq!(bridge.p_config.temp, 300.0);
    }

    #[test]
    fn test_inverter_gate_physics() {
        let params = ProcessParams::default();
        let bridge = TCADBridge::new(&params);

        // pMOS: Source at 0V, VDD at -15V.
        // Logic High (0V) -> Vgs = 0V -> Cutoff

        // Let's also check the Poisson result directly to see psi_surface
        let p_solver = PoissonSolver::new(bridge.p_config.clone());
        let p_res = p_solver.solve(0.0);
        eprintln!("DEBUG: Vgs=0, psi_surface={}", p_res.psi_surface);

        let res_off = bridge.solve_device(0.0, 0.0, -15.0, 0.0, 10e-6, TransistorKind::Enhancement);
        eprintln!("DEBUG: res_off.ids = {}", res_off.ids);
        assert!(res_off.ids.abs() < 1e-6);

        // Logic Low (-15V) -> Vgs = -15V -> Saturation
        let res_on = bridge.solve_device(-15.0, 0.0, -15.0, 0.0, 10e-6, TransistorKind::Enhancement);
        eprintln!("DEBUG: res_on.ids = {}", res_on.ids);
        assert!(res_on.ids.abs() > 1e-6);
        assert!(res_on.gm > 0.0);
    }
}
