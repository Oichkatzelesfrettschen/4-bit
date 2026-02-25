//! Modified Nodal Analysis (MNA) matrix assembly and solution.
//!
//! Builds the conductance matrix G and current vector I for the system
//! G*V = I, then solves via LU decomposition (nalgebra).
//!
//! The MNA matrix includes:
//! - Conductance stamps from transistor linearizations (gm, gds)
//! - Gmin shunts for convergence
//! - Fixed voltage source constraints (power rails)

use nalgebra::{DMatrix, DVector};

/// MNA linear system: G*x = rhs.
///
/// Only free (non-fixed) nodes are included in the matrix.
/// The `node_to_row` mapping converts graph node indices to matrix rows.
pub struct MnaSystem {
    /// Conductance matrix (n x n) where n = number of free nodes.
    pub g: DMatrix<f64>,
    /// Right-hand side current vector.
    pub rhs: DVector<f64>,
    /// Map from graph node index to matrix row. None for fixed nodes.
    pub node_to_row: Vec<Option<usize>>,
    /// Number of free nodes (matrix dimension).
    pub dim: usize,
}

impl MnaSystem {
    /// Create a new MNA system for the given number of free nodes.
    ///
    /// # Arguments
    /// * `num_free` - number of non-fixed nodes
    /// * `num_total` - total number of nodes in the circuit graph
    /// * `free_indices` - graph indices of free nodes (sorted)
    pub fn new(num_free: usize, num_total: usize, free_indices: &[usize]) -> Self {
        let mut node_to_row = vec![None; num_total];
        for (row, &node_idx) in free_indices.iter().enumerate() {
            node_to_row[node_idx] = Some(row);
        }

        Self {
            g: DMatrix::zeros(num_free, num_free),
            rhs: DVector::zeros(num_free),
            node_to_row,
            dim: num_free,
        }
    }

    /// Reset the matrix and RHS to zero for a new iteration.
    pub fn clear(&mut self) {
        self.g.fill(0.0);
        self.rhs.fill(0.0);
    }

    /// Stamp a conductance between two nodes.
    ///
    /// Adds g to the diagonal entries and subtracts from off-diagonals,
    /// implementing I = g * (V_a - V_b).
    ///
    /// If either node is fixed (power rail), the contribution goes to RHS.
    pub fn stamp_conductance(&mut self, node_a: usize, node_b: usize, g_val: f64, fixed_voltages: &[Option<f64>]) {
        let row_a = self.node_to_row[node_a];
        let row_b = self.node_to_row[node_b];
        let v_a_fixed = fixed_voltages[node_a];
        let v_b_fixed = fixed_voltages[node_b];

        match (row_a, row_b) {
            (Some(ra), Some(rb)) => {
                // Both free: stamp into matrix
                self.g[(ra, ra)] += g_val;
                self.g[(rb, rb)] += g_val;
                self.g[(ra, rb)] -= g_val;
                self.g[(rb, ra)] -= g_val;
            }
            (Some(ra), None) => {
                // B is fixed: stamp diagonal and move fixed voltage to RHS
                self.g[(ra, ra)] += g_val;
                if let Some(vb) = v_b_fixed {
                    self.rhs[ra] += g_val * vb;
                }
            }
            (None, Some(rb)) => {
                // A is fixed: stamp diagonal and move fixed voltage to RHS
                self.g[(rb, rb)] += g_val;
                if let Some(va) = v_a_fixed {
                    self.rhs[rb] += g_val * va;
                }
            }
            (None, None) => {
                // Both fixed: no unknowns, nothing to stamp
            }
        }
    }

    /// Stamp a current source into a node.
    ///
    /// Positive current flows INTO the node (convention).
    pub fn stamp_current(&mut self, node: usize, current: f64) {
        if let Some(row) = self.node_to_row[node] {
            self.rhs[row] += current;
        }
    }

    /// Stamp a transistor linearization (Norton equivalent).
    ///
    /// At the operating point, the transistor is linearized as:
    /// - Conductance gds between drain and source
    /// - Transconductance: I_d += gm * (Vgs - Vgs_op) = gm * dVg - gm * dVs
    ///   This adds gm to the matrix coupling gate to drain-source path
    /// - Current source Ieq = Ids_op - gds*Vds_op - gm*Vgs_op
    ///
    /// For pMOS with absolute terminal voltages (gate, source, drain as
    /// graph node indices), using companion model:
    ///   Id = Ids_op + gm*(Vgs - Vgs_op) + gds*(Vds - Vds_op)
    #[allow(clippy::too_many_arguments)]
    pub fn stamp_transistor(
        &mut self,
        gate: usize,
        source: usize,
        drain: usize,
        ids_op: f64,
        gm: f64,
        gds: f64,
        vgs_op: f64,
        vds_op: f64,
        fixed_voltages: &[Option<f64>],
    ) {
        // Conductance gds between drain and source
        self.stamp_conductance(drain, source, gds, fixed_voltages);

        // Transconductance: the gate voltage controls drain current.
        // Linear model: dId = gm * dVgs + gds * dVds
        //
        // Stamp gm contributions:
        // Current into drain: +gm * Vg, -gm * Vs
        // Current out of source: -gm * Vg, +gm * Vs
        let row_d = self.node_to_row[drain];
        let row_s = self.node_to_row[source];
        let row_g = self.node_to_row[gate];

        // gm coupling: gate -> drain current
        match (row_d, row_g) {
            (Some(rd), Some(rg)) => {
                self.g[(rd, rg)] += gm;
            }
            (Some(rd), None) => {
                // Gate is fixed
                if let Some(vg) = fixed_voltages[gate] {
                    self.rhs[rd] += gm * vg;
                }
            }
            _ => {}
        }

        // gm coupling: gate -> source current (negative)
        match (row_s, row_g) {
            (Some(rs), Some(rg)) => {
                self.g[(rs, rg)] -= gm;
            }
            (Some(rs), None) => {
                if let Some(vg) = fixed_voltages[gate] {
                    self.rhs[rs] -= gm * vg;
                }
            }
            _ => {}
        }

        // gm coupling: source -> drain (negative)
        match (row_d, row_s) {
            (Some(rd), Some(rs)) => {
                self.g[(rd, rs)] -= gm;
            }
            (Some(rd), None) => {
                if let Some(vs) = fixed_voltages[source] {
                    self.rhs[rd] += gm * vs;
                }
            }
            _ => {}
        }

        // gm coupling: source -> source (positive)
        if let Some(rs) = row_s {
            self.g[(rs, rs)] += gm;
        }

        // Equivalent current source (companion model):
        // Ieq = Ids_op - gm * Vgs_op - gds * Vds_op
        let ieq = ids_op - gm * vgs_op - gds * vds_op;

        // Ieq flows into drain, out of source
        if let Some(rd) = row_d {
            self.rhs[rd] += ieq;
        }
        if let Some(rs) = row_s {
            self.rhs[rs] -= ieq;
        }
    }

    /// Add gmin shunts from each free node to ground (node 0 convention).
    ///
    /// This improves convergence by ensuring the matrix is not singular.
    pub fn add_gmin(&mut self, gmin: f64) {
        for i in 0..self.dim {
            self.g[(i, i)] += gmin;
        }
    }

    /// Solve the linear system G*x = rhs.
    ///
    /// Returns the solution vector (voltages at free nodes),
    /// or None if the system is singular.
    pub fn solve(&self) -> Option<DVector<f64>> {
        self.g.clone().lu().solve(&self.rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resistor_divider_2_node() {
        // VDD(5V) -- R -- N0 -- R -- GND(0V)
        // Node 0: VDD (fixed, 5V)
        // Node 1: N0 (free)
        // Node 2: GND (fixed, 0V)
        let free_indices = vec![1_usize];
        let mut mna = MnaSystem::new(1, 3, &free_indices);

        let fixed = vec![Some(5.0), None, Some(0.0)];
        let g = 0.001; // 1k ohm

        mna.stamp_conductance(0, 1, g, &fixed); // VDD to N0
        mna.stamp_conductance(1, 2, g, &fixed); // N0 to GND

        let sol = mna.solve().expect("should solve");
        assert!(
            (sol[0] - 2.5).abs() < 1e-10,
            "Divider mid = {:.6}, expected 2.5",
            sol[0]
        );
    }

    #[test]
    fn resistor_divider_3_node() {
        // VDD(5V) -- R -- N0 -- R -- N1 -- R -- GND(0V)
        // Free nodes: 1 (N0), 2 (N1)
        // Fixed: 0 (VDD=5), 3 (GND=0)
        let free_indices = vec![1_usize, 2];
        let mut mna = MnaSystem::new(2, 4, &free_indices);

        let fixed = vec![Some(5.0), None, None, Some(0.0)];
        let g = 0.001;

        mna.stamp_conductance(0, 1, g, &fixed);
        mna.stamp_conductance(1, 2, g, &fixed);
        mna.stamp_conductance(2, 3, g, &fixed);

        let sol = mna.solve().expect("should solve");
        // Expected: N0 ~ 3.33V, N1 ~ 1.67V
        assert!((sol[0] - 10.0 / 3.0).abs() < 1e-6, "N0 = {:.6}, expected 3.333", sol[0]);
        assert!((sol[1] - 5.0 / 3.0).abs() < 1e-6, "N1 = {:.6}, expected 1.667", sol[1]);
    }

    #[test]
    fn gmin_prevents_singular() {
        // Single node with no connections: matrix would be zero
        let free_indices = vec![0_usize];
        let mut mna = MnaSystem::new(1, 1, &free_indices);
        mna.add_gmin(1e-12);

        let sol = mna.solve();
        assert!(sol.is_some(), "gmin should prevent singularity");
    }

    #[test]
    fn stamp_current_source() {
        // Node with current source and conductance to ground
        // I = 1mA, G = 1mS -> V = I/G = 1V
        let free_indices = vec![0_usize];
        let mut mna = MnaSystem::new(1, 2, &free_indices);

        let fixed = vec![None, Some(0.0)];
        mna.stamp_conductance(0, 1, 0.001, &fixed);
        mna.stamp_current(0, 0.001); // 1mA into node 0

        let sol = mna.solve().expect("should solve");
        assert!((sol[0] - 1.0).abs() < 1e-10, "V = {:.6}, expected 1.0", sol[0]);
    }

    #[test]
    fn pmos_voltage_divider() {
        // Test MNA with a pMOS-like inverter:
        // VDD(-15V) node 0 (fixed), OUT node 1 (free), VSS(0V) node 2 (fixed)
        // Depletion load: gds=1e-5 between VDD and OUT
        // Enhancement: gds=5e-5 between OUT and VSS (gate driven to VDD)
        let free_indices = vec![1_usize];
        let mut mna = MnaSystem::new(1, 3, &free_indices);

        let fixed = vec![Some(-15.0), None, Some(0.0)];

        // Depletion load: conductance from VDD to OUT
        mna.stamp_conductance(0, 1, 1e-5, &fixed);
        // Enhancement driver: conductance from OUT to VSS
        mna.stamp_conductance(1, 2, 5e-5, &fixed);

        let sol = mna.solve().expect("should solve");
        // V_out = (g_load * VDD + g_driver * VSS) / (g_load + g_driver)
        // = (1e-5 * -15 + 5e-5 * 0) / (1e-5 + 5e-5) = -15e-5 / 6e-5 = -2.5V
        assert!((sol[0] - (-2.5)).abs() < 1e-6, "V_out = {:.6}, expected -2.5", sol[0]);
    }

    #[test]
    fn clear_resets() {
        let free_indices = vec![0_usize];
        let mut mna = MnaSystem::new(1, 2, &free_indices);

        let fixed = vec![None, Some(0.0)];
        mna.stamp_conductance(0, 1, 0.001, &fixed);
        mna.stamp_current(0, 0.001);

        mna.clear();

        // After clear, matrix should be zero -> singular without gmin
        assert!((mna.g[(0, 0)] - 0.0).abs() < 1e-20);
        assert!((mna.rhs[0] - 0.0).abs() < 1e-20);
    }
}
