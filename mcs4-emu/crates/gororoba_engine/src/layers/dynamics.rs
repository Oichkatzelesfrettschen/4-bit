//! Lattice Boltzmann Method (LBM) D3Q19 Implementation
//!
//! Evaluates the physical dynamics operating over the specified topology.
//! Specifically, implements a 3D Lattice Boltzmann solver to map the
//! discrete temporal evolution of the 4004 clock phases to a macroscopic
//! fluid or state continuous field.

use nalgebra::Vector3;

/// LBM D3Q19 specific constants
pub const Q: usize = 19;
pub const W0: f64 = 1.0 / 3.0;
pub const WS: f64 = 1.0 / 18.0;
pub const WL: f64 = 1.0 / 36.0;

/// Discrete velocity directions for D3Q19
pub const C: [Vector3<i32>; Q] = [
    Vector3::new(0, 0, 0),    // 0: Center
    Vector3::new(1, 0, 0),    // 1: +X
    Vector3::new(-1, 0, 0),   // 2: -X
    Vector3::new(0, 1, 0),    // 3: +Y
    Vector3::new(0, -1, 0),   // 4: -Y
    Vector3::new(0, 0, 1),    // 5: +Z
    Vector3::new(0, 0, -1),   // 6: -Z
    Vector3::new(1, 1, 0),    // 7: +X+Y
    Vector3::new(-1, -1, 0),  // 8: -X-Y
    Vector3::new(1, -1, 0),   // 9: +X-Y
    Vector3::new(-1, 1, 0),   // 10: -X+Y
    Vector3::new(1, 0, 1),    // 11: +X+Z
    Vector3::new(-1, 0, -1),  // 12: -X-Z
    Vector3::new(1, 0, -1),   // 13: +X-Z
    Vector3::new(-1, 0, 1),   // 14: -X+Z
    Vector3::new(0, 1, 1),    // 15: +Y+Z
    Vector3::new(0, -1, -1),  // 16: -Y-Z
    Vector3::new(0, 1, -1),   // 17: +Y-Z
    Vector3::new(0, -1, 1),   // 18: -Y+Z
];

/// Weights for each discrete velocity
pub const WEIGHTS: [f64; Q] = [
    W0, WS, WS, WS, WS, WS, WS, WL, WL, WL, WL, WL, WL, WL, WL, WL, WL, WL, WL,
];

/// A single cell in the D3Q19 lattice
#[derive(Clone, Debug, PartialEq)]
pub struct LbmCell {
    /// Distribution functions for the 19 discrete velocities
    pub f: [f64; Q],
    /// Kinematic viscosity (spatially varying due to Topological Frustration)
    pub nu: f64,
}

impl Default for LbmCell {
    fn default() -> Self {
        let mut f = [0.0; Q];
        for i in 0..Q {
            f[i] = WEIGHTS[i]; // Initialize to equilibrium at rest, density=1.0
        }
        Self { f, nu: 0.1 } // Default viscosity
    }
}

impl LbmCell {
    /// Compute macroscopic density (rho)
    pub fn density(&self) -> f64 {
        self.f.iter().sum()
    }

    /// Compute macroscopic velocity (u)
    pub fn velocity(&self, rho: f64) -> Vector3<f64> {
        let mut u = Vector3::zeros();
        for i in 0..Q {
            let c_f = Vector3::new(C[i].x as f64, C[i].y as f64, C[i].z as f64);
            u += c_f * self.f[i];
        }
        u / rho
    }

    /// Compute equilibrium distribution function (f_eq)
    pub fn equilibrium(&self, rho: f64, u: Vector3<f64>) -> [f64; Q] {
        let mut f_eq = [0.0; Q];
        let u_sq = u.dot(&u);

        for i in 0..Q {
            let c_f = Vector3::new(C[i].x as f64, C[i].y as f64, C[i].z as f64);
            let cu = c_f.dot(&u);
            f_eq[i] = WEIGHTS[i] * rho * (1.0 + 3.0 * cu + 4.5 * cu * cu - 1.5 * u_sq);
        }
        f_eq
    }

    /// Update the kinematic viscosity based on topological frustration.
    ///
    /// Implements Task 1.2: Frustration-Viscosity Coupling
    /// nu(x) = nu_base * exp(-lambda * (F(x) - 3/8)^2)
    pub fn update_viscosity(&mut self, frustration_index: f64) {
        let nu_base = 0.1;
        let lambda = 10.0; // Decay factor
        let offset = frustration_index - (3.0 / 8.0);
        self.nu = nu_base * (-lambda * offset * offset).exp();
    }

    /// Perform BGK collision step (in-place)
    pub fn collide_bgk(&mut self) {
        let rho = self.density();
        let u = self.velocity(rho);
        let f_eq = self.equilibrium(rho, u);
        
        // Relaxation time tau related to kinematic viscosity nu
        // nu = (tau - 0.5) / 3
        let tau = 3.0 * self.nu + 0.5;
        let omega = 1.0 / tau;

        for i in 0..Q {
            self.f[i] += omega * (f_eq[i] - self.f[i]);
        }
    }
}

/// 3D Lattice for the LBM simulation
pub struct Lattice3D {
    pub width: usize,
    pub height: usize,
    pub depth: usize,
    pub cells: Vec<LbmCell>,
    pub next_cells: Vec<LbmCell>,
}

impl Lattice3D {
    pub fn new(w: usize, h: usize, d: usize) -> Self {
        let size = w * h * d;
        Self {
            width: w,
            height: h,
            depth: d,
            cells: vec![LbmCell::default(); size],
            next_cells: vec![LbmCell::default(); size],
        }
    }

    fn idx(&self, x: usize, y: usize, z: usize) -> usize {
        (z * self.height + y) * self.width + x
    }

    /// Apply spatial streaming step
    pub fn stream(&mut self) {
        for z in 0..self.depth {
            for y in 0..self.height {
                for x in 0..self.width {
                    let curr_idx = self.idx(x, y, z);
                    
                    for i in 0..Q {
                        let cx = C[i].x;
                        let cy = C[i].y;
                        let cz = C[i].z;

                        // Periodic boundary conditions for streaming
                        let nx = (x as i32 + cx).rem_euclid(self.width as i32) as usize;
                        let ny = (y as i32 + cy).rem_euclid(self.height as i32) as usize;
                        let nz = (z as i32 + cz).rem_euclid(self.depth as i32) as usize;

                        let next_idx = self.idx(nx, ny, nz);
                        self.next_cells[next_idx].f[i] = self.cells[curr_idx].f[i];
                        self.next_cells[next_idx].nu = self.cells[next_idx].nu; // Preserve viscosity
                    }
                }
            }
        }
        // Swap buffers
        std::mem::swap(&mut self.cells, &mut self.next_cells);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lbm_cell_rest_state() {
        let cell = LbmCell::default();
        let rho = cell.density();
        let u = cell.velocity(rho);

        // Density should be 1.0 (sum of weights)
        assert!((rho - 1.0).abs() < 1e-6);

        // Velocity should be 0.0 at rest
        assert!(u.norm() < 1e-6);

        // Equilibrium should match current state at rest
        let f_eq = cell.equilibrium(rho, u);
        for i in 0..Q {
            assert!((f_eq[i] - cell.f[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn test_lbm_collision_conserves_mass() {
        let mut cell = LbmCell::default();
        // Perturb the cell slightly
        cell.f[1] += 0.01;
        cell.f[2] -= 0.01;

        let initial_density = cell.density();
        cell.collide_bgk();
        let final_density = cell.density();

        // Collision step must strictly conserve mass
        assert!((initial_density - final_density).abs() < 1e-12);
    }
}
