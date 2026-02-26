# Plan: Algebraic Geometry, Statistical Pipeline, Physics, and Housekeeping

## Context

This plan synthesizes a large ChatGPT analysis of Cayley-Dickson algebras, finite
projective geometry, ultrametric structures, and astrophysical physics into actionable
work items for the open_gororoba repository. The analysis identified several theoretical
structures (PG(n-2,2) mapping, bit-predicates for motif classes, sign-twist cancellation)
that are **observed** in the existing motif census but not yet extracted into code. It
also identified performance and statistical methodology gaps in the GPU ultrametric
pipeline, missing GR physics functions, ~30 bibliography entries, and superseded Python
scripts.

Three exploration agents and three design agents confirmed the current state:
- 1539 tests passing, 0 clippy warnings, 13 Rust workspace crates
- boxkites.rs (1877 lines), zd_graphs.rs (1124 lines), cayley_dickson.rs (1374 lines)
- Ultrametric pipeline: NO adaptive permutation, NO squared-distance opt, NO subset search
- gr_core: NO plasma frequency, NO ScatteringRegime dispatcher
- Swarm HAPI already exists (no action needed)
- Relativistic sound speed already exists in cosmology_core/eos.rs
- PGO/BOLT not justified for research codebase

---

## Workstream A: Algebraic Infrastructure (algebra_core)

### A1. Create `projective_geometry.rs` -- PG(m,2) finite projective space
- **File**: `crates/algebra_core/src/projective_geometry.rs` (NEW)
- Points = non-zero vectors of GF(2)^{m+1}, represented as usize bitmasks
- Lines = unordered triples {a, b, a XOR b} (all non-zero)
- Structs: `PGPoint = usize`, `PGLine { points: [PGPoint; 3] }`,
  `ProjectiveGeometry { m, points, lines, point_lines }`
- Functions: `pg(m)`, `pg_from_cd_dim(dim)`, `incidence_matrix()`
- Tests: PG(2,2)=Fano (7 pts, 7 lines), PG(3,2) (15 pts, 35 lines),
  PG(4,2) (31 pts, 155 lines), general formula verification for m=2..7
- Cross-validate PG(2,2) against hardcoded `O_TRIPS` in boxkites.rs
- ~200 lines impl + ~100 lines tests

### A2. PG-to-motif-component bijection mapping
- **File**: `crates/algebra_core/src/projective_geometry.rs`
- `map_components_to_pg(dim, components, pg) -> Option<Vec<PGPoint>>`
- Extract XOR-key structure from each component to derive GF(2)^(n-1) labels
- Verify bijection at dim=16 (7 comp <-> 7 PG(2,2) pts),
  dim=32 (15 <-> PG(3,2)), dim=64 (31 <-> PG(4,2))
- Verify PG-line structure matches algebraic triples
- ~100 lines impl + ~60 lines tests

### A3. Graph invariants on MotifComponent
- **File**: `crates/algebra_core/src/boxkites.rs` (MODIFY)
- Add `use nalgebra::{DMatrix, SymmetricEigen};` (nalgebra 0.33 already in workspace)
- New methods on `MotifComponent`:
  - `adjacency_matrix() -> DMatrix<f64>` (map nodes to indices, fill 0/1)
  - `spectrum() -> Vec<f64>` (eigenvalues via SymmetricEigen, sorted descending)
  - `triangle_count() -> usize` (trace(A^3) / 6)
  - `diameter() -> usize` (BFS all-pairs shortest paths)
  - `girth() -> usize` (shortest cycle via BFS)
- Tests: octahedron spectrum [-2,-2,0,0,2,4], 8 triangles per box-kite,
  same-class components have identical spectra, different classes have distinct spectra
- ~150 lines impl + ~80 lines tests

### A4. GF(2)-linear bit-predicate for motif class assignment
- **File**: `crates/algebra_core/src/projective_geometry.rs`
- `find_linear_class_predicate(labels, classes, n_bits) -> Option<PGPoint>`
- Brute-force over 2^(n-1)-1 non-zero weight vectors w in GF(2)^(n-1)
- Fallback: `find_affine_class_predicate` (adds constant term)
- Fallback: `find_quadratic_class_predicate` (adds cross-terms)
- Key test: dim=32 8/7 split -> find w that separates heptacross (8) from mixed (7)
- Verify hyperplane structure: class-A points form a PG hyperplane
- dim=64: 4 classes from 2 independent GF(2)-linear predicates
- ~120 lines impl + ~80 lines tests

### A5. Sign-twist cancellation predicate formalization
- **File**: `crates/algebra_core/src/projective_geometry.rs`
- `sign_twist_signature(dim, a: CrossPair, b: CrossPair) -> u8`
  (4-bit encoding of sign(i,k)*sign(j,l), sign(i,l)*sign(j,k), etc.)
- Cross-validate with existing `diagonal_zero_products_exact()` in boxkites.rs
- Verify: signature fully determines zero-product solution count
- Verify: 168 actual / 315 XOR-passing = 0.533 ratio at dim=16
- ~80 lines impl + ~50 lines tests

### A6. Register module and re-exports
- **File**: `crates/algebra_core/src/lib.rs` (MODIFY)
- Add `pub mod projective_geometry;` and appropriate `pub use` statements

---

## Workstream B: Statistical Pipeline (stats_core/ultrametric)

### B1. Squared-distance optimization (CPU)
- **Files**: `crates/stats_core/src/ultrametric/baire.rs`, `mod.rs`, `local.rs`
- Remove `.sqrt()` calls in `matrix_free_fraction` (baire.rs lines 446-448)
- Pre-compute `epsilon_sq = 1.0 - (1.0 - epsilon).powi(2)` (exact, not approximate)
- Same for `compute_ultrametric_fraction` in mod.rs (1D scalar case: use `.powi(2)`)
- Same for `euclidean_3d` in local.rs -> `euclidean_3d_sq`
- Leave matrix-based functions unchanged (sqrt already paid in matrix construction)
- Test: `test_squared_distance_equivalence` -- both paths match to 1e-12

### B2. Squared-distance optimization (GPU kernel)
- **File**: `crates/stats_core/src/ultrametric/gpu.rs`
- Remove 3 `sqrtf()` calls in CUDA kernel (lines 79-81)
- Host pre-computes `epsilons_sq[i] = 1.0 - (1.0 - eps[i]).powi(2)` and uploads
- Sorting via fmaxf/fminf works identically on squared distances
- Test: `test_gpu_squared_matches_cpu` if CUDA available

### B3. Null model abstraction
- **File**: `crates/stats_core/src/ultrametric/null_models.rs` (NEW)
- Enum `NullModel { ColumnIndependent, RowPermutation, ToroidalShift, RandomRotation }`
- `apply_null_column_major(data, n, d, null_model, rng)` dispatcher
- ColumnIndependent = current behavior (backward compat default)
- RowPermutation = shuffle row indices, reorder all columns
- ToroidalShift = random circular shift per column
- RandomRotation = Mezzadri Haar SO(d) matrix applied to d-dim data
- Add `null_model: NullModel` parameter to `matrix_free_ultrametric_test` and GPU tests
- Wrapper functions for backward compat (existing CLI binaries unchanged)
- Register in mod.rs: `pub mod null_models;`
- Tests: legacy match, row preservation, marginal preservation, distance preservation

### B4. Adaptive/sequential permutation testing (Besag-Clifford 1991)
- **File**: `crates/stats_core/src/ultrametric/adaptive.rs` (NEW)
- `AdaptiveConfig { batch_size, max_permutations, alpha, confidence, min_permutations }`
- `AdaptiveResult { p_value, n_permutations_used, stopped_early, stop_reason, p_trajectory }`
- `should_stop(r, k, alpha, confidence)` -- binomial CI via Normal inverse CDF (statrs)
- `adaptive_permutation_test(config, run_batch_closure) -> AdaptiveResult`
- CPU batch_size=20, GPU batch_size=50 (configurable)
- Register in mod.rs: `pub mod adaptive;`
- Tests: early stop on random data, max on marginal, match fixed-perm result, type-I rate

### B5. Attribute subset search library function
- **File**: `crates/stats_core/src/ultrametric/subset_search.rs` (NEW)
- Extract `attribute_subsets`, `generate_combinations`, `project_data` from
  `gororoba_cli/src/bin/multi_dataset_ultrametric.rs` into library
- `SubsetTestResult { attribute_indices, names, fraction, null_mean, effect_size, raw_p, ... }`
- `SubsetSearchResult { subsets, adjusted_p_values, significant, n_subsets, fdr_level }`
- `subset_search(data, specs, min_k, n_triples, adaptive_config, null_model, fdr, seed)`
- GPU variant: `subset_search_gpu(engine, ...)`
- Update CLI binary to call library function (thin wrapper)
- Register in mod.rs: `pub mod subset_search;`
- Tests: combinatorial count, random data false positive rate, planted signal detection

---

## Workstream C: GR Physics (gr_core)

### C1. Plasma frequency function
- **File**: `crates/gr_core/src/absorption.rs` (MODIFY)
- `pub fn plasma_frequency(n_e: f64) -> f64`
  = `(n_e * E_CHARGE^2 / (PI * M_ELECTRON))^0.5` [CGS, Hz]
- Tests: solar corona (n_e~1e8 -> nu_p~90 MHz), ISM (n_e~0.03 -> nu_p~1.6 kHz),
  scaling (nu_p ~ sqrt(n_e)), consistency with DM dispersion delay

### C2. Mie scattering regime dispatcher
- **File**: `crates/gr_core/src/scattering.rs` (MODIFY)
- `pub enum ScatteringRegime { Rayleigh, Transition, Geometric }`
- `pub fn classify_scattering_regime(x: f64) -> ScatteringRegime`
  (x < 0.05 -> Rayleigh, x < 1.0 -> Transition, x >= 1.0 -> Geometric)
- Tests: boundary values, continuity with existing `mie_efficiency()`

### C3. Standalone relativistic sound speed in gr_core
- **File**: `crates/gr_core/src/constants.rs` (MODIFY)
- `pub fn relativistic_sound_speed_sq(pressure, energy_density, gamma) -> f64`
  = `gamma * P / (epsilon + P)`
- Complements `cosmology_core::eos::Polytrope::sound_speed_sq()` (already exists)
- Provides gr_core-local function without cross-crate dependency
- Tests: ideal gas limits, ultrarelativistic (gamma=4/3, P=epsilon/3 -> cs^2=1/3)

---

## Workstream D: Documentation

### D1. BIBLIOGRAPHY.md -- add ~30 new references
- **File**: `docs/BIBLIOGRAPHY.md` (MODIFY)
- Cayley-Dickson: Schafer (1966), Eakin-Sathaye (1990), Smith (1995), Bales (2023)
- Finite projective geometry: Saniga-Holweck-Pracna (2015), Polster (1998), Hirschfeld (1998)
- Zero-divisor graphs: Anderson-Livingston (1999), Mulay (2002), DeMeyer+ (2002)
- Ultrametric: Rammal-Toulouse-Virasoro (1986), Murtagh (2004), Bradley (2008, 2010)
- Permutation testing: Besag-Clifford (1991), Phipson-Smyth (2010), North+ (2002)
- GR/astro: Page-Thorne (1974), Novikov-Thorne (1973), Boyer-Lindquist
- Dust: Bohren-Huffman (1983), Draine (2003), MRN (1977)
- NANOGrav/EHT: Agazie+ (2023), IPTA DR2, EHT Papers I-VIII
- Run `make ascii-check` after (diacritics in author names are common pitfall)

### D2. CLAIMS_EVIDENCE_MATRIX.md -- new claims
- **File**: `docs/CLAIMS_EVIDENCE_MATRIX.md` (MODIFY)
- C-443: "CD motif components correspond bijectively to PG(n-2,2) points"
- C-444: "Motif class assignment is determined by GF(2)-linear predicate"
- C-445: "Sign-twist signature fully determines zero-product solution count"
- C-446: "Adaptive permutation testing preserves type-I error rate"
- Each with WHERE_STATED, WHAT_WOULD_REFUTE, initial status

### D3. INSIGHTS.md -- new insights
- **File**: `docs/INSIGHTS.md` (MODIFY)
- I-013: PG(n-2,2) finite geometry explains motif component counts and structure
- I-014: Squared-distance optimization gives ~30% speedup on ultrametric tests

### D4. ROADMAP.md -- add new workstream items
- **File**: `docs/ROADMAP.md` (MODIFY)
- Section 6.x: Finite projective geometry integration
- Section 6.x: Adaptive permutation testing
- Section 6.x: Attribute subset search (library extraction)

---

## Workstream E: Python Migration (Superseded Script Deletion)

### E1. Delete sedenion_nilpotency.py
- **File**: `src/sedenion_nilpotency.py` (DELETE, 121 lines)
- Verify: CD multiplication + ZD search fully in algebra_core/cayley_dickson.rs
- Regenerate any CSV outputs from Rust if needed

### E2. Delete e6_refinement.py
- **File**: `src/e6_refinement.py` (DELETE, 108 lines)
- Verify: E8/E6 root generation covered by atlas-embeddings + algebra_core/e8_lattice.rs

### E3. Delete correlate_physics.py
- **File**: `src/correlate_physics.py` (DELETE, 48 lines)
- Verify: superseded by Rust analysis binaries in gororoba_cli

### E4. Delete correlate_real_physics.py
- **File**: `src/correlate_real_physics.py` (DELETE, 59 lines)
- Same verification as E3

### E5. Delete correlate_real_physics_gwtc.py
- **File**: `src/correlate_real_physics_gwtc.py` (DELETE, 72 lines)
- Verify: GWTC handling in data_core/catalogs/gwtc.rs covers this

### E6. Delete surreal_matmul_bench.py
- **File**: `src/surreal_matmul_bench.py` (DELETE, 144 lines)
- Verify: Rust criterion benchmarks in data_core::benchmarks are superior

### E7. Delete sedenion_mass_ladder.py
- **File**: `src/sedenion_mass_ladder.py` (DELETE, 66 lines)
- Verify: mass predictions from associators covered by Rust algebra_core

---

## Workstream F: Build Optimization

### F1. Evaluate mold linker for faster incremental builds
- **File**: `.cargo/config.toml` (MODIFY)
- Add: `linker = "clang"`, `rustflags += ["-C", "link-arg=-fuse-ld=mold"]`
- Benchmark: measure incremental rebuild time before/after
- Skip PGO/BOLT (not justified for research codebase per analysis)

---

## Execution Order (Dependency-Aware)

Phase 1 -- Quick wins (no code dependencies):
  D1 (bibliography), D2 (claims), D3 (insights), D4 (roadmap)

Phase 2 -- Independent implementations (parallel):
  A1 (PG core)  |  B1+B2 (squared-distance CPU+GPU)  |  C1 (plasma freq)
  A6 (register)  |  B3 (null models)                   |  C2 (Mie dispatcher)

Phase 3 -- Dependent on Phase 2:
  A2 (PG-motif mapping, needs A1)
  A3 (graph invariants, independent but logically after A1)
  B4 (adaptive testing, needs B3)
  C3 (standalone sound speed)

Phase 4 -- Dependent on Phase 3:
  A4 (bit-predicate, needs A2)
  A5 (sign-twist, needs A2)
  B5 (subset search, needs B3+B4)

Phase 5 -- Cleanup:
  E1-E7 (Python deletions, one at a time, verified)
  F1 (mold linker evaluation)

---

## Verification

After each workstream:
- `cargo clippy --workspace -j$(nproc) -- -D warnings` (zero warnings)
- `cargo test --workspace -j$(nproc)` (all tests pass)
- `make ascii-check` (no Unicode in source files)

After all workstreams:
- Full test count should increase from 1539 to ~1600+ (new algebraic + statistical tests)
- `cargo test -p algebra_core` verifies PG, invariants, predicates
- `cargo test -p stats_core` verifies squared-distance, adaptive, subset search
- `cargo test -p gr_core` verifies plasma frequency, Mie dispatcher, sound speed
- Regenerate any CSV outputs affected by Python deletions
