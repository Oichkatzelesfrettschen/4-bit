//! Sparse Modified Nodal Analysis (MNA) matrix assembly and solution.
//!
//! Equivalent to [`super::matrix::MnaSystem`] but uses faer's sparse
//! COO->CSC->LU pipeline. This allows scaling from the 4003 (93 nodes)
//! to the 4001 (2159 nodes) without O(n^2) memory or O(n^3) time.
//!
//! The key optimization: circuit topology is fixed, so the symbolic
//! factorization (fill-reducing permutations, elimination tree) is
//! computed once and reused across Newton-Raphson iterations. Only the
//! numeric factorization (actual values) changes each iteration.

use faer::sparse::SparseColMat;
use faer::sparse::Triplet;
use faer::sparse::linalg::solvers::{Lu, SymbolicLu};
use faer::linalg::solvers::SolveCore;
use faer::{Conj, Mat};

/// Sparse MNA linear system: G*x = rhs.
///
/// Same interface as [`super::matrix::MnaSystem`] but stores stamps as
/// COO triplets and solves via sparse LU factorization.
pub struct SparseMnaSystem {
    /// Map from graph node index to matrix row. None for fixed nodes.
    pub node_to_row: Vec<Option<usize>>,
    /// Number of free nodes (matrix dimension).
    pub dim: usize,
    /// COO triplets accumulated during stamping.
    triplets: Vec<Triplet<usize, usize, f64>>,
    /// Right-hand side current vector.
    rhs: Vec<f64>,
    /// Cached symbolic factorization (computed once per topology).
    symbolic: Option<SymbolicLu<usize>>,
}

impl SparseMnaSystem {
    /// Create a new sparse MNA system for the given number of free nodes.
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
            node_to_row,
            dim: num_free,
            triplets: Vec::new(),
            rhs: vec![0.0; num_free],
            symbolic: None,
        }
    }

    /// Reset the triplets and RHS for a new iteration.
    pub fn clear(&mut self) {
        self.triplets.clear();
        self.rhs.iter_mut().for_each(|v| *v = 0.0);
    }

    /// Add a single triplet to the COO accumulator.
    ///
    /// Duplicate (row, col) entries are summed during CSC assembly.
    fn add_triplet(&mut self, row: usize, col: usize, val: f64) {
        if val != 0.0 {
            self.triplets.push(Triplet::new(row, col, val));
        }
    }

    /// Stamp a conductance between two nodes.
    ///
    /// Same semantics as [`super::matrix::MnaSystem::stamp_conductance`].
    pub fn stamp_conductance(
        &mut self,
        node_a: usize,
        node_b: usize,
        g_val: f64,
        fixed_voltages: &[Option<f64>],
    ) {
        let row_a = self.node_to_row[node_a];
        let row_b = self.node_to_row[node_b];
        let v_a_fixed = fixed_voltages[node_a];
        let v_b_fixed = fixed_voltages[node_b];

        match (row_a, row_b) {
            (Some(ra), Some(rb)) => {
                self.add_triplet(ra, ra, g_val);
                self.add_triplet(rb, rb, g_val);
                self.add_triplet(ra, rb, -g_val);
                self.add_triplet(rb, ra, -g_val);
            }
            (Some(ra), None) => {
                self.add_triplet(ra, ra, g_val);
                if let Some(vb) = v_b_fixed {
                    self.rhs[ra] += g_val * vb;
                }
            }
            (None, Some(rb)) => {
                self.add_triplet(rb, rb, g_val);
                if let Some(va) = v_a_fixed {
                    self.rhs[rb] += g_val * va;
                }
            }
            (None, None) => {}
        }
    }

    /// Stamp a current source into a node.
    pub fn stamp_current(&mut self, node: usize, current: f64) {
        if let Some(row) = self.node_to_row[node] {
            self.rhs[row] += current;
        }
    }

    /// Stamp a transistor linearization (Norton equivalent).
    ///
    /// Same semantics as [`super::matrix::MnaSystem::stamp_transistor`].
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
        // gds between drain and source
        self.stamp_conductance(drain, source, gds, fixed_voltages);

        let row_d = self.node_to_row[drain];
        let row_s = self.node_to_row[source];
        let row_g = self.node_to_row[gate];

        // gm coupling: gate -> drain current
        match (row_d, row_g) {
            (Some(rd), Some(rg)) => {
                self.add_triplet(rd, rg, gm);
            }
            (Some(rd), None) => {
                if let Some(vg) = fixed_voltages[gate] {
                    self.rhs[rd] += gm * vg;
                }
            }
            _ => {}
        }

        // gm coupling: gate -> source current (negative)
        match (row_s, row_g) {
            (Some(rs), Some(rg)) => {
                self.add_triplet(rs, rg, -gm);
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
                self.add_triplet(rd, rs, -gm);
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
            self.add_triplet(rs, rs, gm);
        }

        // Equivalent current source: Ieq = Ids_op - gm * Vgs_op - gds * Vds_op
        let ieq = ids_op - gm * vgs_op - gds * vds_op;

        if let Some(rd) = row_d {
            self.rhs[rd] += ieq;
        }
        if let Some(rs) = row_s {
            self.rhs[rs] -= ieq;
        }
    }

    /// Add gmin shunts from each free node to ground.
    pub fn add_gmin(&mut self, gmin: f64) {
        for i in 0..self.dim {
            self.add_triplet(i, i, gmin);
        }
    }

    /// Add a conductance to the diagonal of a specific matrix row.
    ///
    /// Used by the transient solver to stamp capacitor companion models
    /// directly by row index (bypassing the node_to_row lookup).
    pub fn add_gmin_node(&mut self, row: usize, g_val: f64) {
        self.add_triplet(row, row, g_val);
    }

    /// Add a current source to a specific matrix row.
    ///
    /// Used by the transient solver to stamp capacitor companion models
    /// directly by row index (bypassing the node_to_row lookup).
    pub fn stamp_current_by_row(&mut self, row: usize, current: f64) {
        if row < self.rhs.len() {
            self.rhs[row] += current;
        }
    }

    /// Compute symbolic factorization from the current sparsity pattern.
    ///
    /// Call this once after the first stamping pass to cache the fill-reducing
    /// permutations. Subsequent iterations with the same topology reuse this.
    pub fn compute_symbolic(&mut self) -> bool {
        if self.dim == 0 {
            return false;
        }
        let mat = match SparseColMat::<usize, f64>::try_new_from_triplets(
            self.dim, self.dim, &self.triplets,
        ) {
            Ok(m) => m,
            Err(_) => return false,
        };
        match SymbolicLu::try_new(mat.symbolic()) {
            Ok(s) => {
                self.symbolic = Some(s);
                true
            }
            Err(_) => false,
        }
    }

    /// Assemble the current triplets into a CSC matrix and solve G*x = rhs.
    ///
    /// If a symbolic factorization is cached, reuses it for the numeric
    /// factorization. Otherwise, computes both symbolic and numeric from
    /// scratch.
    ///
    /// Returns the solution vector (voltages at free nodes), or None if
    /// the system is singular or assembly fails.
    pub fn solve(&mut self) -> Option<Vec<f64>> {
        if self.dim == 0 {
            return Some(Vec::new());
        }

        let mat = SparseColMat::<usize, f64>::try_new_from_triplets(
            self.dim, self.dim, &self.triplets,
        ).ok()?;

        let lu = if let Some(ref symbolic) = self.symbolic {
            // Reuse cached symbolic factorization
            Lu::try_new_with_symbolic(symbolic.clone(), mat.as_ref()).ok()?
        } else {
            // No cached symbolic; compute full LU
            let sym = SymbolicLu::try_new(mat.symbolic()).ok()?;
            let lu = Lu::try_new_with_symbolic(sym.clone(), mat.as_ref()).ok()?;
            self.symbolic = Some(sym);
            lu
        };

        // Build RHS as a faer column matrix (n x 1)
        let mut rhs_mat = Mat::<f64>::zeros(self.dim, 1);
        for (i, &v) in self.rhs.iter().enumerate() {
            rhs_mat[(i, 0)] = v;
        }

        // Solve in place
        lu.solve_in_place_with_conj(Conj::No, rhs_mat.as_mut());

        // Extract solution
        let sol: Vec<f64> = (0..self.dim).map(|i| rhs_mat[(i, 0)]).collect();
        Some(sol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resistor_divider_2_node() {
        // VDD(5V) -- R -- N0 -- R -- GND(0V)
        // Node 0: VDD (fixed, 5V), Node 1: N0 (free), Node 2: GND (fixed, 0V)
        let free_indices = vec![1_usize];
        let mut mna = SparseMnaSystem::new(1, 3, &free_indices);

        let fixed = vec![Some(5.0), None, Some(0.0)];
        let g = 0.001; // 1k ohm

        mna.stamp_conductance(0, 1, g, &fixed);
        mna.stamp_conductance(1, 2, g, &fixed);

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
        let free_indices = vec![1_usize, 2];
        let mut mna = SparseMnaSystem::new(2, 4, &free_indices);

        let fixed = vec![Some(5.0), None, None, Some(0.0)];
        let g = 0.001;

        mna.stamp_conductance(0, 1, g, &fixed);
        mna.stamp_conductance(1, 2, g, &fixed);
        mna.stamp_conductance(2, 3, g, &fixed);

        let sol = mna.solve().expect("should solve");
        assert!(
            (sol[0] - 10.0 / 3.0).abs() < 1e-6,
            "N0 = {:.6}, expected 3.333",
            sol[0]
        );
        assert!(
            (sol[1] - 5.0 / 3.0).abs() < 1e-6,
            "N1 = {:.6}, expected 1.667",
            sol[1]
        );
    }

    #[test]
    fn gmin_prevents_singular() {
        // Single node with no connections: matrix would be zero
        let free_indices = vec![0_usize];
        let mut mna = SparseMnaSystem::new(1, 1, &free_indices);
        mna.add_gmin(1e-12);

        let sol = mna.solve();
        assert!(sol.is_some(), "gmin should prevent singularity");
    }

    #[test]
    fn stamp_current_source() {
        // I = 1mA, G = 1mS -> V = I/G = 1V
        let free_indices = vec![0_usize];
        let mut mna = SparseMnaSystem::new(1, 2, &free_indices);

        let fixed = vec![None, Some(0.0)];
        mna.stamp_conductance(0, 1, 0.001, &fixed);
        mna.stamp_current(0, 0.001);

        let sol = mna.solve().expect("should solve");
        assert!(
            (sol[0] - 1.0).abs() < 1e-10,
            "V = {:.6}, expected 1.0",
            sol[0]
        );
    }

    #[test]
    fn pmos_voltage_divider() {
        // VDD(-15V) node 0, OUT node 1, VSS(0V) node 2
        let free_indices = vec![1_usize];
        let mut mna = SparseMnaSystem::new(1, 3, &free_indices);

        let fixed = vec![Some(-15.0), None, Some(0.0)];
        mna.stamp_conductance(0, 1, 1e-5, &fixed);
        mna.stamp_conductance(1, 2, 5e-5, &fixed);

        let sol = mna.solve().expect("should solve");
        assert!(
            (sol[0] - (-2.5)).abs() < 1e-6,
            "V_out = {:.6}, expected -2.5",
            sol[0]
        );
    }

    #[test]
    fn clear_resets() {
        let free_indices = vec![0_usize];
        let mut mna = SparseMnaSystem::new(1, 2, &free_indices);

        let fixed = vec![None, Some(0.0)];
        mna.stamp_conductance(0, 1, 0.001, &fixed);
        mna.stamp_current(0, 0.001);

        mna.clear();

        assert!(mna.triplets.is_empty());
        assert!((mna.rhs[0]).abs() < 1e-20);
    }

    #[test]
    fn sparse_matches_dense_divider() {
        // Compare sparse vs dense on 3-node resistor divider
        use super::super::matrix::MnaSystem;

        let free_indices = vec![1_usize, 2];
        let fixed = vec![Some(5.0), None, None, Some(0.0)];
        let g = 0.001;

        // Dense solve
        let mut dense = MnaSystem::new(2, 4, &free_indices);
        dense.stamp_conductance(0, 1, g, &fixed);
        dense.stamp_conductance(1, 2, g, &fixed);
        dense.stamp_conductance(2, 3, g, &fixed);
        let dense_sol = dense.solve().expect("dense solve");

        // Sparse solve
        let mut sparse = SparseMnaSystem::new(2, 4, &free_indices);
        sparse.stamp_conductance(0, 1, g, &fixed);
        sparse.stamp_conductance(1, 2, g, &fixed);
        sparse.stamp_conductance(2, 3, g, &fixed);
        let sparse_sol = sparse.solve().expect("sparse solve");

        for i in 0..2 {
            assert!(
                (dense_sol[i] - sparse_sol[i]).abs() < 1e-10,
                "Node {}: dense={:.10}, sparse={:.10}",
                i,
                dense_sol[i],
                sparse_sol[i]
            );
        }
    }

    #[test]
    fn symbolic_reuse_across_iterations() {
        // Simulate two Newton-Raphson iterations with different values
        // but the same sparsity pattern. Verify symbolic is computed once.
        let free_indices = vec![1_usize, 2];
        let mut mna = SparseMnaSystem::new(2, 4, &free_indices);

        let fixed = vec![Some(5.0), None, None, Some(0.0)];

        // First iteration
        mna.stamp_conductance(0, 1, 0.001, &fixed);
        mna.stamp_conductance(1, 2, 0.001, &fixed);
        mna.stamp_conductance(2, 3, 0.001, &fixed);

        assert!(mna.symbolic.is_none(), "No symbolic yet");
        let sol1 = mna.solve().expect("first solve");
        assert!(mna.symbolic.is_some(), "Symbolic cached after first solve");

        // Second iteration with different conductance values
        mna.clear();
        mna.stamp_conductance(0, 1, 0.002, &fixed);
        mna.stamp_conductance(1, 2, 0.002, &fixed);
        mna.stamp_conductance(2, 3, 0.002, &fixed);

        let sol2 = mna.solve().expect("second solve");

        // Same circuit topology, same resistor ratios -> same voltages
        for i in 0..2 {
            assert!(
                (sol1[i] - sol2[i]).abs() < 1e-10,
                "Node {}: iter1={:.10}, iter2={:.10}",
                i,
                sol1[i],
                sol2[i]
            );
        }
    }

    #[test]
    fn empty_system_returns_empty() {
        let mut mna = SparseMnaSystem::new(0, 0, &[]);
        let sol = mna.solve().expect("empty solve");
        assert!(sol.is_empty());
    }

    #[test]
    fn singleton_system() {
        // Single free node with gmin only
        let free_indices = vec![0_usize];
        let mut mna = SparseMnaSystem::new(1, 1, &free_indices);
        mna.add_gmin(1e-12);
        mna.stamp_current(0, 1e-12); // small current

        let sol = mna.solve().expect("singleton solve");
        assert!(
            (sol[0] - 1.0).abs() < 1e-6,
            "V = {:.6}, expected 1.0",
            sol[0]
        );
    }

    #[test]
    fn sparse_matches_dense_inverter() {
        // Compare sparse vs dense on pMOS inverter-like circuit
        use super::super::matrix::MnaSystem;

        let free_indices = vec![1_usize];
        let fixed = vec![Some(-15.0), None, Some(0.0)];

        // Dense
        let mut dense = MnaSystem::new(1, 3, &free_indices);
        dense.stamp_conductance(0, 1, 1e-5, &fixed);
        dense.stamp_conductance(1, 2, 5e-5, &fixed);
        let dense_sol = dense.solve().expect("dense");

        // Sparse
        let mut sparse = SparseMnaSystem::new(1, 3, &free_indices);
        sparse.stamp_conductance(0, 1, 1e-5, &fixed);
        sparse.stamp_conductance(1, 2, 5e-5, &fixed);
        let sparse_sol = sparse.solve().expect("sparse");

        assert!(
            (dense_sol[0] - sparse_sol[0]).abs() < 1e-10,
            "dense={:.10}, sparse={:.10}",
            dense_sol[0],
            sparse_sol[0]
        );
    }

    #[test]
    fn larger_mesh_network() {
        // 10-node mesh to exercise sparse structure
        let n = 10;
        let free_indices: Vec<usize> = (1..=n).collect();
        let num_total = n + 2; // node 0 = VDD, node n+1 = GND
        let mut mna = SparseMnaSystem::new(n, num_total, &free_indices);

        let mut fixed = vec![None; num_total];
        fixed[0] = Some(5.0);
        fixed[n + 1] = Some(0.0);

        let g = 0.001;
        // Chain: VDD -- N1 -- N2 -- ... -- Nn -- GND
        mna.stamp_conductance(0, 1, g, &fixed);
        for i in 1..n {
            mna.stamp_conductance(i, i + 1, g, &fixed);
        }
        mna.stamp_conductance(n, n + 1, g, &fixed);

        let sol = mna.solve().expect("mesh solve");

        // Voltage should decrease linearly from 5V to 0V
        for (i, &v) in sol.iter().enumerate().take(n) {
            let expected = 5.0 * (n - i) as f64 / (n + 1) as f64;
            assert!(
                (v - expected).abs() < 1e-6,
                "Node {}: got {:.6}, expected {:.6}",
                i + 1,
                v,
                expected
            );
        }
    }

    #[test]
    fn gmin_stamp_verification() {
        // Verify that gmin adds to diagonal entries only
        let free_indices = vec![0_usize, 1, 2];
        let mut mna = SparseMnaSystem::new(3, 3, &free_indices);
        let gmin = 1e-12;
        mna.add_gmin(gmin);

        // Should have exactly 3 diagonal triplets
        assert_eq!(mna.triplets.len(), 3);
        for t in &mna.triplets {
            assert_eq!(t.row, t.col, "gmin should only be on diagonal");
            assert!((t.val - gmin).abs() < 1e-30);
        }
    }
}
