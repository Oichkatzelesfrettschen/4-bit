//! Parameter sensitivity analysis via finite-difference perturbation.
//!
//! Computes dV/dp: the sensitivity of node voltages to changes in
//! process parameters. Uses the direct finite-difference method:
//! perturb each parameter, re-solve, compute the derivative.
//!
//! For a circuit with N nodes and P parameters, this requires
//! P+1 DC solves (one baseline + one per parameter). The adjoint
//! method (deferred to Phase IV) would reduce this to 2 solves
//! regardless of P, at the cost of implementation complexity.

use crate::{
    circuit::CircuitGraph,
    process::ProcessParams,
    solver::{dc_op::DcSolver, SolverConfig},
};

/// A named process parameter that can be perturbed.
#[derive(Clone, Debug)]
pub enum SensitivityParam {
    /// Gate oxide thickness (m).
    Tox,
    /// Enhancement-mode threshold voltage (V).
    VthEnhancement,
    /// Depletion-mode threshold voltage (V).
    VthDepletion,
    /// Low-field mobility (cm^2/V*s).
    Mu0,
    /// Substrate doping (cm^-3).
    NSub,
    /// Channel-length modulation (1/V).
    Lambda,
    /// Temperature (K).
    Temp,
    /// DIBL coefficient (V/V).
    EtaDibl,
    /// Mobility degradation theta.
    Theta,
}

impl SensitivityParam {
    /// Human-readable name for display.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Tox => "t_ox",
            Self::VthEnhancement => "Vth_enh",
            Self::VthDepletion => "Vth_dep",
            Self::Mu0 => "mu_0",
            Self::NSub => "N_sub",
            Self::Lambda => "lambda",
            Self::Temp => "T",
            Self::EtaDibl => "eta_dibl",
            Self::Theta => "theta",
        }
    }

    /// Default relative perturbation size for this parameter.
    pub fn default_delta(&self) -> f64 {
        match self {
            Self::Tox => 0.01,            // 1% of t_ox
            Self::VthEnhancement => 0.01, // 1% of Vth
            Self::VthDepletion => 0.01,
            Self::Mu0 => 0.01,
            Self::NSub => 0.01,
            Self::Lambda => 0.05, // 5% (lambda is small)
            Self::Temp => 0.01,
            Self::EtaDibl => 0.1, // 10% (eta_dibl is very small)
            Self::Theta => 0.05,
        }
    }

    /// Get the current value of this parameter from a ProcessParams struct.
    pub fn get_value(&self, params: &ProcessParams) -> f64 {
        match self {
            Self::Tox => params.t_ox,
            Self::VthEnhancement => params.vth_enhancement,
            Self::VthDepletion => params.vth_depletion,
            Self::Mu0 => params.mu_0,
            Self::NSub => params.n_sub,
            Self::Lambda => params.lambda,
            Self::Temp => params.temp,
            Self::EtaDibl => params.eta_dibl,
            Self::Theta => params.theta,
        }
    }

    /// Set the value of this parameter in a ProcessParams struct.
    pub fn set_value(&self, params: &mut ProcessParams, value: f64) {
        match self {
            Self::Tox => params.t_ox = value,
            Self::VthEnhancement => params.vth_enhancement = value,
            Self::VthDepletion => params.vth_depletion = value,
            Self::Mu0 => params.mu_0 = value,
            Self::NSub => params.n_sub = value,
            Self::Lambda => params.lambda = value,
            Self::Temp => params.temp = value,
            Self::EtaDibl => params.eta_dibl = value,
            Self::Theta => params.theta = value,
        }
    }

    /// Create a perturbed copy of process parameters.
    pub fn perturb(&self, params: &ProcessParams, relative_delta: f64) -> ProcessParams {
        let mut perturbed = params.clone();
        let base_value = self.get_value(params);
        let abs_delta = if base_value.abs() < 1e-15 {
            relative_delta // absolute perturbation when base is ~0
        } else {
            base_value * relative_delta
        };
        self.set_value(&mut perturbed, base_value + abs_delta);
        perturbed
    }
}

/// Sensitivity of a node voltage to a process parameter.
#[derive(Clone, Debug)]
pub struct NodeSensitivity {
    /// Node index.
    pub node_idx: usize,
    /// Baseline voltage (V).
    pub v_baseline: f64,
    /// Perturbed voltage (V).
    pub v_perturbed: f64,
    /// Absolute sensitivity dV/dp (V per parameter unit).
    pub dv_dp: f64,
    /// Normalized sensitivity: (dV/V) / (dp/p).
    pub normalized: f64,
}

/// Result of a sensitivity analysis for one parameter.
#[derive(Clone, Debug)]
pub struct ParamSensitivityResult {
    /// Parameter that was perturbed.
    pub param: SensitivityParam,
    /// Relative perturbation size used.
    pub delta: f64,
    /// Per-node sensitivities.
    pub nodes: Vec<NodeSensitivity>,
    /// Node with maximum absolute sensitivity.
    pub max_sensitivity_node: usize,
    /// Maximum absolute sensitivity value.
    pub max_sensitivity: f64,
}

/// Complete sensitivity analysis result across all parameters.
#[derive(Clone, Debug)]
pub struct SensitivityResult {
    /// Per-parameter results.
    pub params: Vec<ParamSensitivityResult>,
    /// Baseline node voltages.
    pub baseline_voltages: Vec<f64>,
}

impl SensitivityResult {
    /// Get the sensitivity of a specific node to a specific parameter.
    pub fn sensitivity(&self, param_idx: usize, node_idx: usize) -> Option<&NodeSensitivity> {
        self.params
            .get(param_idx)
            .and_then(|pr| pr.nodes.iter().find(|n| n.node_idx == node_idx))
    }

    /// Find the most sensitive parameter for a given node.
    pub fn most_sensitive_param_for_node(&self, node_idx: usize) -> Option<(usize, f64)> {
        let mut best_idx = 0;
        let mut best_sens = 0.0_f64;
        for (i, pr) in self.params.iter().enumerate() {
            if let Some(ns) = pr.nodes.iter().find(|n| n.node_idx == node_idx) {
                if ns.dv_dp.abs() > best_sens {
                    best_sens = ns.dv_dp.abs();
                    best_idx = i;
                }
            }
        }
        if best_sens > 0.0 {
            Some((best_idx, best_sens))
        } else {
            None
        }
    }
}

/// Compute finite-difference sensitivity for a single parameter.
///
/// # Arguments
/// * `baseline_voltages` - Node voltages from baseline DC solve
/// * `perturbed_voltages` - Node voltages from perturbed DC solve
/// * `param` - The parameter that was perturbed
/// * `base_params` - The original process parameters
/// * `delta` - Relative perturbation size used
pub fn compute_param_sensitivity(
    baseline_voltages: &[f64],
    perturbed_voltages: &[f64],
    param: SensitivityParam,
    base_params: &ProcessParams,
    delta: f64,
) -> ParamSensitivityResult {
    let base_value = param.get_value(base_params);
    let abs_delta = if base_value.abs() < 1e-15 {
        delta
    } else {
        base_value * delta
    };

    let mut nodes = Vec::new();
    let mut max_sensitivity = 0.0_f64;
    let mut max_node = 0;

    for (i, (&v_base, &v_pert)) in baseline_voltages.iter().zip(perturbed_voltages.iter()).enumerate() {
        let dv = v_pert - v_base;
        let dv_dp = if abs_delta.abs() > 1e-30 { dv / abs_delta } else { 0.0 };

        let normalized = if v_base.abs() > 1e-10 && base_value.abs() > 1e-15 {
            (dv / v_base) / delta
        } else {
            0.0
        };

        if dv_dp.abs() > max_sensitivity {
            max_sensitivity = dv_dp.abs();
            max_node = i;
        }

        nodes.push(NodeSensitivity {
            node_idx: i,
            v_baseline: v_base,
            v_perturbed: v_pert,
            dv_dp,
            normalized,
        });
    }

    ParamSensitivityResult {
        param,
        delta,
        nodes,
        max_sensitivity_node: max_node,
        max_sensitivity,
    }
}

/// All default sensitivity parameters for the MCS-4 process.
pub fn default_params() -> Vec<SensitivityParam> {
    vec![
        SensitivityParam::Tox,
        SensitivityParam::VthEnhancement,
        SensitivityParam::Mu0,
        SensitivityParam::NSub,
        SensitivityParam::Lambda,
        SensitivityParam::Temp,
    ]
}

impl SensitivityResult {
    /// Return the sensitivity matrix as a 2-D `Vec[node_idx][param_idx]` of normalized sensitivities.
    ///
    /// Shape: N x P, where N = number of nodes and P = number of parameters.
    /// Each entry is the normalized sensitivity (dV/V) / (dp/p) for that node-parameter pair.
    pub fn sensitivity_matrix(&self) -> Vec<Vec<f64>> {
        let num_nodes = self.baseline_voltages.len();
        let num_params = self.params.len();
        let mut matrix = vec![vec![0.0_f64; num_params]; num_nodes];
        for (p_idx, pr) in self.params.iter().enumerate() {
            for ns in &pr.nodes {
                if ns.node_idx < num_nodes {
                    matrix[ns.node_idx][p_idx] = ns.normalized;
                }
            }
        }
        matrix
    }

    /// Return parameters ranked by maximum absolute sensitivity across all nodes.
    ///
    /// Returns up to `top_n` entries of (SensitivityParam, max_abs_sensitivity).
    /// Sorted descending (most sensitive first).
    pub fn most_sensitive_parameters(&self, top_n: usize) -> Vec<(&SensitivityParam, f64)> {
        let mut ranked: Vec<(&SensitivityParam, f64)> =
            self.params.iter().map(|pr| (&pr.param, pr.max_sensitivity)).collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(top_n);
        ranked
    }

    /// Return tornado chart data for a specific node.
    ///
    /// Returns a Vec of (param_name, low_delta_V, high_delta_V) tuples, one per parameter.
    /// `low_delta_V` is the voltage change for a -delta perturbation (negative).
    /// `high_delta_V` is the voltage change for a +delta perturbation (positive).
    ///
    /// For a finite-difference sensitivity stored in dv_dp, the approximate changes are:
    ///   high_delta_V = dv_dp * abs_delta
    ///   low_delta_V  = -dv_dp * abs_delta
    pub fn tornado_data(&self, node_idx: usize) -> Vec<(String, f64, f64)> {
        self.params
            .iter()
            .map(|pr| {
                let base_value = pr.param.get_value(&ProcessParams::default());
                let abs_delta = if base_value.abs() < 1e-15 {
                    pr.delta
                } else {
                    base_value * pr.delta
                };
                let dv_dp = pr
                    .nodes
                    .iter()
                    .find(|ns| ns.node_idx == node_idx)
                    .map(|ns| ns.dv_dp)
                    .unwrap_or(0.0);
                let high_delta = dv_dp * abs_delta;
                let low_delta = -high_delta;
                (pr.param.name().to_string(), low_delta, high_delta)
            })
            .collect()
    }
}

/// Run integrated sensitivity analysis: baseline DC solve + one perturbed solve per parameter.
///
/// WHY: The existing `compute_param_sensitivity` operates on pre-computed voltage vectors.
/// This function closes the loop by running the DC solves internally, connecting the sensitivity
/// framework to the actual circuit solver.
///
/// For N nodes and P parameters, runs P+1 DC solves total (1 baseline + 1 per parameter).
///
/// # Arguments
/// * `graph` - Circuit graph (mutated for each solve; node voltages are reset between solves).
/// * `base_params` - Process parameters at the nominal operating point.
/// * `params_to_sweep` - List of parameters to perturb.
pub fn run_sensitivity_analysis(
    graph: &mut CircuitGraph,
    base_params: &ProcessParams,
    params_to_sweep: &[SensitivityParam],
) -> SensitivityResult {
    // Save initial voltages for reset between solves.
    let initial_voltages: Vec<f64> = graph.nodes.iter().map(|n| n.voltage).collect();

    // Baseline solve.
    let config = SolverConfig::default();
    let baseline_result = DcSolver::new(config.clone(), base_params.clone()).solve(graph);
    let baseline_voltages = baseline_result.voltages.clone();

    let mut param_results = Vec::new();

    for param in params_to_sweep {
        // Reset node voltages before each perturbed solve.
        for (node, &v) in graph.nodes.iter_mut().zip(initial_voltages.iter()) {
            node.voltage = v;
        }

        let delta = param.default_delta();
        let perturbed_params = param.perturb(base_params, delta);
        let perturbed_result = DcSolver::new(config.clone(), perturbed_params).solve(graph);

        let pr = compute_param_sensitivity(
            &baseline_voltages,
            &perturbed_result.voltages,
            param.clone(),
            base_params,
            delta,
        );
        param_results.push(pr);
    }

    SensitivityResult {
        params: param_results,
        baseline_voltages,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_name_non_empty() {
        let params = default_params();
        for p in &params {
            assert!(!p.name().is_empty());
        }
    }

    #[test]
    fn param_get_set_roundtrip() {
        let pp = ProcessParams::default();
        let params = vec![
            SensitivityParam::Tox,
            SensitivityParam::VthEnhancement,
            SensitivityParam::Mu0,
            SensitivityParam::NSub,
            SensitivityParam::Lambda,
            SensitivityParam::Temp,
        ];
        for p in &params {
            let val = p.get_value(&pp);
            let mut pp2 = pp.clone();
            p.set_value(&mut pp2, val + 1.0);
            let val2 = p.get_value(&pp2);
            assert!(
                (val2 - (val + 1.0)).abs() < 1e-12,
                "{}: get={}, set to {}, got {}",
                p.name(),
                val,
                val + 1.0,
                val2
            );
        }
    }

    #[test]
    fn perturb_creates_modified_copy() {
        let pp = ProcessParams::default();
        let param = SensitivityParam::Tox;
        let perturbed = param.perturb(&pp, 0.01);
        let orig = param.get_value(&pp);
        let pert = param.get_value(&perturbed);
        assert!(
            (pert - orig * 1.01).abs() < 1e-15,
            "perturbed t_ox: orig={:.3e}, pert={:.3e}, expected {:.3e}",
            orig,
            pert,
            orig * 1.01
        );
    }

    #[test]
    fn compute_sensitivity_basic() {
        let pp = ProcessParams::default();
        let baseline = vec![0.0, -5.0, -7.5, -10.0];
        let perturbed = vec![0.0, -5.1, -7.6, -10.0]; // node 1 and 2 shifted

        let result = compute_param_sensitivity(&baseline, &perturbed, SensitivityParam::VthEnhancement, &pp, 0.01);

        assert_eq!(result.nodes.len(), 4);

        // Node 0 and 3 should have zero sensitivity
        assert!(result.nodes[0].dv_dp.abs() < 1e-10, "node 0 sensitivity should be ~0");

        // Node 1 shifted by -0.1V for 1% Vth change (Vth = -3.0, delta = -0.03V)
        assert!(result.nodes[1].dv_dp.abs() > 0.0, "node 1 should have sensitivity");
    }

    #[test]
    fn max_sensitivity_correct() {
        let pp = ProcessParams::default();
        let baseline = vec![0.0, -5.0, -10.0];
        let perturbed = vec![0.0, -5.2, -10.0];

        let result = compute_param_sensitivity(&baseline, &perturbed, SensitivityParam::Tox, &pp, 0.01);

        assert_eq!(result.max_sensitivity_node, 1);
        assert!(result.max_sensitivity > 0.0);
    }

    #[test]
    fn sensitivity_result_query() {
        let pp = ProcessParams::default();
        let baseline = vec![0.0, -5.0, -10.0];
        let perturbed = vec![0.0, -5.1, -10.05];

        let result = SensitivityResult {
            params: vec![compute_param_sensitivity(
                &baseline,
                &perturbed,
                SensitivityParam::VthEnhancement,
                &pp,
                0.01,
            )],
            baseline_voltages: baseline,
        };

        assert!(result.sensitivity(0, 1).is_some());
        assert!(result.sensitivity(1, 0).is_none()); // out of bounds

        let (best_idx, _) = result.most_sensitive_param_for_node(1).expect("should find");
        assert_eq!(best_idx, 0);
    }

    #[test]
    fn default_params_covers_key_parameters() {
        let params = default_params();
        assert!(params.len() >= 5, "should have at least 5 default params");
    }

    #[test]
    fn zero_perturbation_zero_sensitivity() {
        let pp = ProcessParams::default();
        let baseline = vec![0.0, -5.0, -10.0];
        let perturbed = baseline.clone(); // no change

        let result = compute_param_sensitivity(&baseline, &perturbed, SensitivityParam::Mu0, &pp, 0.01);

        for ns in &result.nodes {
            assert!(ns.dv_dp.abs() < 1e-20, "zero perturbation should give zero sensitivity");
        }
    }

    #[test]
    fn perturb_near_zero_param() {
        let pp = ProcessParams {
            eta_dibl: 0.0,
            ..ProcessParams::default()
        };

        let param = SensitivityParam::EtaDibl;
        let perturbed = param.perturb(&pp, 0.1);
        let pert_val = param.get_value(&perturbed);
        // Should use absolute perturbation when base is 0
        assert!(
            (pert_val - 0.1).abs() < 1e-12,
            "perturbed eta_dibl from 0: got {:.3e}",
            pert_val
        );
    }

    // --- Phase 11: Integrated sensitivity runner tests ---

    use crate::circuit::CircuitGraph;

    fn fixed_two_rail_graph() -> CircuitGraph {
        let mut g = CircuitGraph::new();
        let vss = g.add_node(0);
        let vdd = g.add_node(1);
        g.set_power_rail(vss, 0.0, "VSS");
        g.set_power_rail(vdd, -15.0, "VDD");
        g
    }

    #[test]
    fn sensitivity_matrix_dimensions() {
        // WHY: NxP matrix must have correct shape (N nodes x P params).
        let base = ProcessParams::default();
        let params_to_sweep = vec![SensitivityParam::VthEnhancement, SensitivityParam::Mu0];
        let mut graph = fixed_two_rail_graph();

        let result = run_sensitivity_analysis(&mut graph, &base, &params_to_sweep);
        let matrix = result.sensitivity_matrix();

        let num_nodes = result.baseline_voltages.len();
        let num_params = params_to_sweep.len();

        assert_eq!(matrix.len(), num_nodes, "Matrix rows must equal node count");
        for row in &matrix {
            assert_eq!(row.len(), num_params, "Matrix columns must equal param count");
        }
    }

    #[test]
    fn sensitivity_ranking_sorted_descending() {
        // WHY: most_sensitive_parameters must return entries in descending order.
        let base = ProcessParams::default();
        let params_to_sweep = vec![
            SensitivityParam::VthEnhancement,
            SensitivityParam::Mu0,
            SensitivityParam::Lambda,
        ];
        let mut graph = fixed_two_rail_graph();

        let result = run_sensitivity_analysis(&mut graph, &base, &params_to_sweep);
        let ranked = result.most_sensitive_parameters(3);

        // Verify sorted descending.
        for window in ranked.windows(2) {
            assert!(
                window[0].1 >= window[1].1,
                "Parameters should be sorted descending: {:.3e} vs {:.3e}",
                window[0].1,
                window[1].1
            );
        }
    }

    #[test]
    fn tornado_data_format() {
        // WHY: tornado_data must return (name, low_delta, high_delta) tuples for all params.
        let base = ProcessParams::default();
        let params_to_sweep = vec![SensitivityParam::VthEnhancement, SensitivityParam::Mu0];
        let mut graph = fixed_two_rail_graph();

        let result = run_sensitivity_analysis(&mut graph, &base, &params_to_sweep);
        let tornado = result.tornado_data(0); // node 0 = VSS

        assert_eq!(tornado.len(), params_to_sweep.len(), "One entry per parameter");
        for (name, low, high) in &tornado {
            assert!(!name.is_empty(), "Parameter name must be non-empty");
            // For a fixed node, all deltas should be zero.
            assert!(
                low.abs() < 1e-10,
                "Fixed node low_delta should be ~0: {}={:.3e}",
                name,
                low
            );
            assert!(
                high.abs() < 1e-10,
                "Fixed node high_delta should be ~0: {}={:.3e}",
                name,
                high
            );
        }
    }

    #[test]
    fn sensitivity_symmetric_perturbation() {
        // WHY: for a linear system, +delta and -delta should give approximately equal
        // and opposite sensitivities (linearity check). Fixed-rail circuit is trivially linear.
        let base = ProcessParams::default();
        let params_to_sweep = vec![SensitivityParam::VthEnhancement];
        let mut graph = fixed_two_rail_graph();

        // Run with positive perturbation (default via run_sensitivity_analysis).
        let result_pos = run_sensitivity_analysis(&mut graph, &base, &params_to_sweep);

        // Run with manual negative perturbation.
        let initial_voltages: Vec<f64> = graph.nodes.iter().map(|n| n.voltage).collect();
        for (node, &v) in graph.nodes.iter_mut().zip(initial_voltages.iter()) {
            node.voltage = v;
        }
        let delta = SensitivityParam::VthEnhancement.default_delta();
        let neg_params = SensitivityParam::VthEnhancement.perturb(&base, -delta);
        let baseline_result = DcSolver::new(SolverConfig::default(), base.clone()).solve(&mut graph);
        for (node, &v) in graph.nodes.iter_mut().zip(initial_voltages.iter()) {
            node.voltage = v;
        }
        let neg_result = DcSolver::new(SolverConfig::default(), neg_params).solve(&mut graph);

        // For fixed nodes, both pos and neg perturbations should give zero dV.
        let pos_dv: Vec<f64> = result_pos.params[0].nodes.iter().map(|ns| ns.dv_dp).collect();
        let neg_dv: Vec<f64> = baseline_result
            .voltages
            .iter()
            .zip(neg_result.voltages.iter())
            .map(|(&vb, &vp)| {
                let abs_delta = SensitivityParam::VthEnhancement.get_value(&base) * delta;
                (vb - vp) / abs_delta
            })
            .collect();

        for (pd, nd) in pos_dv.iter().zip(neg_dv.iter()) {
            assert!(
                (pd + nd).abs() < 1e-8 || (pd.abs() < 1e-10 && nd.abs() < 1e-10),
                "Symmetry: pos={:.3e}, neg={:.3e}, sum={:.3e}",
                pd,
                nd,
                pd + nd
            );
        }
    }

    #[test]
    fn run_sensitivity_analysis_fixed_circuit() {
        // WHY: Smoke test that run_sensitivity_analysis completes without panicking
        // and returns the correct number of parameter results.
        let base = ProcessParams::default();
        let sweep_params = default_params();
        let num_params = sweep_params.len();
        let mut graph = fixed_two_rail_graph();

        let result = run_sensitivity_analysis(&mut graph, &base, &sweep_params);

        assert_eq!(result.params.len(), num_params, "One result per param");
        assert_eq!(
            result.baseline_voltages.len(),
            graph.nodes.len(),
            "Baseline voltages must cover all nodes"
        );

        // For a fixed-only circuit, all dV should be zero.
        for pr in &result.params {
            for ns in &pr.nodes {
                assert!(
                    ns.dv_dp.abs() < 1e-10,
                    "Fixed nodes should have zero sensitivity for {}: dv_dp={:.3e}",
                    pr.param.name(),
                    ns.dv_dp
                );
            }
        }
    }
}
