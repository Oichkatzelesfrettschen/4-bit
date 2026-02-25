//! 1D non-uniform spatial mesh for TCAD solvers.
//!
//! Provides a mesh spanning from the gate electrode through the oxide,
//! across the oxide/silicon interface, and into the silicon bulk.
//! Mesh spacing is finest near the interface where field gradients
//! are steepest, and coarser in the bulk.

/// Material region at a mesh point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Region {
    /// Gate electrode (metal or polysilicon).
    Gate,
    /// SiO2 gate oxide.
    Oxide,
    /// Silicon substrate.
    Silicon,
}

/// A 1D spatial mesh for the MOS gate stack.
///
/// Points are ordered from gate surface (x=0) through oxide to silicon bulk.
/// The oxide/silicon interface is at x = t_ox.
#[derive(Clone, Debug)]
pub struct Mesh1D {
    /// Position of each mesh point (m), monotonically increasing.
    pub x: Vec<f64>,
    /// Material region at each point.
    pub region: Vec<Region>,
    /// Index of the oxide/silicon interface point.
    pub interface_idx: usize,
    /// Gate oxide thickness (m).
    pub t_ox: f64,
    /// Total mesh depth into silicon from interface (m).
    pub si_depth: f64,
}

/// Configuration for mesh generation.
#[derive(Clone, Debug)]
pub struct MeshConfig {
    /// Gate oxide thickness (m).
    pub t_ox: f64,
    /// Depth into silicon bulk from interface (m).
    pub si_depth: f64,
    /// Number of points in the oxide region.
    pub n_oxide: usize,
    /// Number of points in the silicon region.
    pub n_silicon: usize,
    /// Grading ratio for silicon mesh (> 1.0 = finer near interface).
    /// A value of 1.0 gives uniform spacing; 3.0 is typical.
    pub si_grading: f64,
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self {
            t_ox: 80e-9,
            si_depth: 5e-6,
            n_oxide: 20,
            n_silicon: 100,
            si_grading: 3.0,
        }
    }
}

impl Mesh1D {
    /// Generate a mesh from configuration.
    ///
    /// The oxide region is uniformly spaced. The silicon region uses
    /// geometric grading so that points are densest near the interface
    /// (where the inversion/depletion layer forms) and sparser in
    /// the neutral bulk.
    pub fn generate(cfg: &MeshConfig) -> Self {
        assert!(cfg.n_oxide >= 2, "need at least 2 oxide points");
        assert!(cfg.n_silicon >= 2, "need at least 2 silicon points");
        assert!(cfg.t_ox > 0.0);
        assert!(cfg.si_depth > 0.0);
        assert!(cfg.si_grading >= 1.0);

        let mut x = Vec::with_capacity(cfg.n_oxide + cfg.n_silicon);
        let mut region = Vec::with_capacity(cfg.n_oxide + cfg.n_silicon);

        // Oxide: uniform spacing from x=0 to x=t_ox
        let dx_ox = cfg.t_ox / (cfg.n_oxide - 1) as f64;
        for i in 0..cfg.n_oxide {
            x.push(i as f64 * dx_ox);
            region.push(Region::Oxide);
        }

        let interface_idx = cfg.n_oxide - 1;

        // Silicon: geometrically graded from interface into bulk.
        // Generate normalized positions [0, 1] then scale.
        if (cfg.si_grading - 1.0).abs() < 1e-6 {
            // Uniform spacing
            let dx_si = cfg.si_depth / (cfg.n_silicon - 1) as f64;
            for i in 1..cfg.n_silicon {
                x.push(cfg.t_ox + i as f64 * dx_si);
                region.push(Region::Silicon);
            }
        } else {
            // Geometric grading: first interval is smallest, grows by ratio.
            // Cumulative sum of geometric series: sum = a * (r^n - 1) / (r - 1)
            // where a = first spacing, r = grading ratio per step.
            //
            // We use a power-law distribution for simplicity:
            //   xi = (i / (n-1))^alpha  where alpha > 1 concentrates near 0
            let alpha = cfg.si_grading;
            let n = cfg.n_silicon;
            for i in 1..n {
                let t = (i as f64 / (n - 1) as f64).powf(alpha);
                x.push(cfg.t_ox + t * cfg.si_depth);
                region.push(Region::Silicon);
            }
        }

        // Mark the interface point as silicon (it's at the boundary)
        region[interface_idx] = Region::Silicon;

        Mesh1D {
            x,
            region,
            interface_idx,
            t_ox: cfg.t_ox,
            si_depth: cfg.si_depth,
        }
    }

    /// Total number of mesh points.
    pub fn len(&self) -> usize {
        self.x.len()
    }

    /// Whether the mesh is empty.
    pub fn is_empty(&self) -> bool {
        self.x.is_empty()
    }

    /// Spacing between point i and i+1 (m).
    pub fn dx(&self, i: usize) -> f64 {
        assert!(i + 1 < self.x.len());
        self.x[i + 1] - self.x[i]
    }

    /// Average spacing around point i (m). Used for finite-difference stencils.
    pub fn dx_avg(&self, i: usize) -> f64 {
        if i == 0 {
            self.dx(0)
        } else if i >= self.x.len() - 1 {
            self.dx(self.x.len() - 2)
        } else {
            0.5 * (self.x[i + 1] - self.x[i - 1])
        }
    }

    /// Permittivity at mesh point i (F/m).
    pub fn epsilon(&self, i: usize) -> f64 {
        use crate::process::silicon::{EPSILON_OX, EPSILON_SI};
        match self.region[i] {
            Region::Gate | Region::Oxide => EPSILON_OX,
            Region::Silicon => EPSILON_SI,
        }
    }

    /// Whether point i is in the silicon region.
    pub fn is_silicon(&self, i: usize) -> bool {
        self.region[i] == Region::Silicon
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_generates_mesh() {
        let cfg = MeshConfig::default();
        let mesh = Mesh1D::generate(&cfg);
        assert_eq!(mesh.len(), cfg.n_oxide + cfg.n_silicon - 1);
        assert!(!mesh.is_empty());
    }

    #[test]
    fn mesh_monotonically_increasing() {
        let mesh = Mesh1D::generate(&MeshConfig::default());
        for i in 1..mesh.len() {
            assert!(
                mesh.x[i] > mesh.x[i - 1],
                "x[{}] = {:.3e} <= x[{}] = {:.3e}",
                i,
                mesh.x[i],
                i - 1,
                mesh.x[i - 1]
            );
        }
    }

    #[test]
    fn interface_at_tox() {
        let cfg = MeshConfig::default();
        let mesh = Mesh1D::generate(&cfg);
        let x_if = mesh.x[mesh.interface_idx];
        assert!(
            (x_if - cfg.t_ox).abs() < 1e-15,
            "interface at {:.3e}, expected {:.3e}",
            x_if,
            cfg.t_ox
        );
    }

    #[test]
    fn silicon_grading_finer_near_interface() {
        let cfg = MeshConfig {
            si_grading: 3.0,
            ..MeshConfig::default()
        };
        let mesh = Mesh1D::generate(&cfg);
        let idx = mesh.interface_idx;

        // First silicon spacing should be smaller than last silicon spacing
        let first_dx = mesh.dx(idx);
        let last_dx = mesh.dx(mesh.len() - 2);
        assert!(
            first_dx < last_dx,
            "first Si dx = {:.3e}, last = {:.3e}, expected first < last",
            first_dx,
            last_dx
        );
    }

    #[test]
    fn uniform_oxide_spacing() {
        let cfg = MeshConfig {
            n_oxide: 10,
            ..MeshConfig::default()
        };
        let mesh = Mesh1D::generate(&cfg);
        let expected_dx = cfg.t_ox / 9.0;

        for i in 0..8 {
            let dx = mesh.dx(i);
            assert!(
                (dx - expected_dx).abs() / expected_dx < 1e-10,
                "oxide dx[{}] = {:.3e}, expected {:.3e}",
                i,
                dx,
                expected_dx
            );
        }
    }

    #[test]
    fn mesh_spans_full_depth() {
        let cfg = MeshConfig::default();
        let mesh = Mesh1D::generate(&cfg);
        let x_last = mesh.x[mesh.len() - 1];
        let expected = cfg.t_ox + cfg.si_depth;
        assert!(
            (x_last - expected).abs() / expected < 1e-10,
            "last point at {:.3e}, expected {:.3e}",
            x_last,
            expected
        );
    }

    #[test]
    fn dx_avg_interior_point() {
        let mesh = Mesh1D::generate(&MeshConfig::default());
        let i = mesh.len() / 2;
        let avg = mesh.dx_avg(i);
        let manual = 0.5 * (mesh.x[i + 1] - mesh.x[i - 1]);
        assert!((avg - manual).abs() < 1e-20);
    }

    #[test]
    fn permittivity_correct_per_region() {
        use crate::process::silicon::{EPSILON_OX, EPSILON_SI};
        let mesh = Mesh1D::generate(&MeshConfig::default());

        // First point is oxide
        assert!((mesh.epsilon(0) - EPSILON_OX).abs() < 1e-20);

        // Last point is silicon
        assert!((mesh.epsilon(mesh.len() - 1) - EPSILON_SI).abs() < 1e-20);
    }

    #[test]
    fn uniform_si_grading() {
        let cfg = MeshConfig {
            si_grading: 1.0,
            n_silicon: 50,
            ..MeshConfig::default()
        };
        let mesh = Mesh1D::generate(&cfg);
        let idx = mesh.interface_idx;

        // All silicon spacings should be equal
        let expected_dx = cfg.si_depth / (cfg.n_silicon - 1) as f64;
        for i in idx..(mesh.len() - 1) {
            let dx = mesh.dx(i);
            assert!(
                (dx - expected_dx).abs() / expected_dx < 1e-6,
                "Si dx[{}] = {:.3e}, expected {:.3e}",
                i,
                dx,
                expected_dx
            );
        }
    }
}
