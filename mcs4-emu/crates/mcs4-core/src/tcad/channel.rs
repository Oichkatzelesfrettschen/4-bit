//! Source-to-drain channel mesh and boundary conditions.
//!
//! Provides the 1D mesh along the channel from source to drain,
//! plus boundary condition setup for drift-diffusion simulation.

/// 1D channel mesh from source to drain.
#[derive(Clone, Debug)]
pub struct ChannelMesh {
    /// Position of each mesh point along the channel (m), from source (x=0) to drain (x=L).
    pub x: Vec<f64>,
    /// Channel length (m).
    pub length: f64,
    /// Channel width (m).
    pub width: f64,
}

/// Configuration for channel mesh generation.
#[derive(Clone, Debug)]
pub struct ChannelConfig {
    /// Channel length (m).
    pub length: f64,
    /// Channel width (m).
    pub width: f64,
    /// Number of mesh points.
    pub n_points: usize,
    /// Grading ratio (> 1.0 = finer near source/drain junctions).
    pub grading: f64,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            length: 10e-6, // 10um for MCS-4 process
            width: 10e-6,
            n_points: 50,
            grading: 1.0, // uniform
        }
    }
}

impl ChannelMesh {
    /// Generate a channel mesh from configuration.
    ///
    /// Uniform spacing by default. Grading > 1 concentrates points
    /// near the source and drain ends.
    pub fn generate(cfg: &ChannelConfig) -> Self {
        assert!(cfg.n_points >= 3, "need at least 3 channel points");
        assert!(cfg.length > 0.0);
        assert!(cfg.width > 0.0);

        let n = cfg.n_points;
        let mut x = Vec::with_capacity(n);

        if (cfg.grading - 1.0).abs() < 1e-6 {
            // Uniform spacing
            let dx = cfg.length / (n - 1) as f64;
            for i in 0..n {
                x.push(i as f64 * dx);
            }
        } else {
            // Symmetric grading: fine at both ends, coarser in center.
            // Map normalized [0,1] through a stretched function.
            let alpha = cfg.grading;
            for i in 0..n {
                let t = i as f64 / (n - 1) as f64;
                // Symmetric stretching: compress toward 0 and 1
                // alpha > 1 makes spacing finer at endpoints
                let stretched = if t <= 0.5 {
                    0.5 * (2.0 * t).powf(alpha)
                } else {
                    1.0 - 0.5 * (2.0 * (1.0 - t)).powf(alpha)
                };
                x.push(stretched * cfg.length);
            }
        }

        ChannelMesh {
            x,
            length: cfg.length,
            width: cfg.width,
        }
    }

    /// Number of mesh points.
    pub fn len(&self) -> usize {
        self.x.len()
    }

    /// Whether the mesh is empty.
    pub fn is_empty(&self) -> bool {
        self.x.is_empty()
    }

    /// Spacing between points i and i+1 (m).
    pub fn dx(&self, i: usize) -> f64 {
        self.x[i + 1] - self.x[i]
    }
}

/// Boundary conditions for the channel drift-diffusion problem.
#[derive(Clone, Debug)]
pub struct ChannelBoundaryConditions {
    /// Hole concentration at source (cm^-3).
    pub p_source: f64,
    /// Hole concentration at drain (cm^-3).
    pub p_drain: f64,
    /// Electrostatic potential at source (V).
    pub psi_source: f64,
    /// Electrostatic potential at drain (V).
    pub psi_drain: f64,
}

impl ChannelBoundaryConditions {
    /// Compute boundary conditions for given bias.
    ///
    /// For pMOS: source and drain are p+ regions in n-type substrate.
    /// The source is at ground (VSS = 0V) and drain is at -Vds.
    ///
    /// # Arguments
    /// * `n_sd` - Source/drain doping concentration (cm^-3)
    /// * `vds` - Drain-to-source voltage (V, negative for pMOS in forward bias)
    /// * `psi_surface` - Surface potential from gate bias (V)
    pub fn for_bias(n_sd: f64, vds: f64, psi_surface: f64) -> Self {
        Self {
            p_source: n_sd,
            p_drain: n_sd,
            psi_source: psi_surface,
            psi_drain: psi_surface + vds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_generates_mesh() {
        let cfg = ChannelConfig::default();
        let mesh = ChannelMesh::generate(&cfg);
        assert_eq!(mesh.len(), cfg.n_points);
        assert!(!mesh.is_empty());
    }

    #[test]
    fn mesh_starts_at_zero_ends_at_length() {
        let cfg = ChannelConfig::default();
        let mesh = ChannelMesh::generate(&cfg);
        assert!(mesh.x[0].abs() < 1e-15);
        assert!((mesh.x[mesh.len() - 1] - cfg.length).abs() < 1e-15);
    }

    #[test]
    fn mesh_monotonic() {
        let cfg = ChannelConfig {
            grading: 2.0,
            ..ChannelConfig::default()
        };
        let mesh = ChannelMesh::generate(&cfg);
        for i in 1..mesh.len() {
            assert!(mesh.x[i] > mesh.x[i - 1]);
        }
    }

    #[test]
    fn graded_mesh_finer_at_ends() {
        let cfg = ChannelConfig {
            grading: 3.0,
            n_points: 51,
            ..ChannelConfig::default()
        };
        let mesh = ChannelMesh::generate(&cfg);

        // First spacing should be smaller than center spacing
        let dx_start = mesh.dx(0);
        let dx_mid = mesh.dx(mesh.len() / 2);
        assert!(
            dx_start < dx_mid,
            "start dx={:.3e} should be < mid dx={:.3e}",
            dx_start,
            dx_mid
        );

        // Last spacing should also be small
        let dx_end = mesh.dx(mesh.len() - 2);
        assert!(
            dx_end < dx_mid,
            "end dx={:.3e} should be < mid dx={:.3e}",
            dx_end,
            dx_mid
        );
    }

    #[test]
    fn boundary_conditions_consistent() {
        let bc = ChannelBoundaryConditions::for_bias(1e19, -5.0, -0.5);
        assert!((bc.p_source - 1e19).abs() < 1.0);
        assert!((bc.psi_drain - bc.psi_source - (-5.0)).abs() < 1e-10);
    }
}
