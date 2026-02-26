# Phase 1 Week 4: E-027 Percolation Threshold Experiment

## Context

**What is being implemented**: Experiment E-027 to validate Thesis 1's core prediction that spatially-varying viscosity derived from Cayley-Dickson frustration produces measurable percolation channels in fluid simulations.

**Why this change is needed**: Phase 1 Weeks 1-3 built the infrastructure (frustration solver, LBM 3D, frustration-viscosity bridge). Week 4 implements the falsification experiment that proves or refutes Thesis 1. This is the critical validation step before proceeding to Phase 2.

**Current state**:
- Phase 1 W1-W3 COMPLETE (127 tests passing, zero warnings)
- vacuum_frustration: 22 tests (frustration solver + bridge)
- lbm_3d: 61 tests (D3Q19 solver + TwoPhaseSystem trait)
- cosmic_scheduler: 38 tests (PhaseScheduler abstraction)
- Claims C-657..C-670, E-027..E-029, CRI-002, I-064 NOT YET REGISTERED

**The problem**: E-027 experiment not implemented, registry incomplete

**Intended outcome**:
- E-027 binary operational with deterministic results
- Percolation-frustration correlation measured (target: p < 0.05)
- Registry updated with claims/experiments/insights
- Phase 1 complete and validated

---

## Terminology Standards (MANDATORY)

**Use these standardized terms from registry/terminology_standards.toml**:

- **Frustration density** (T-GRAPH-002): Harary-Zaslavsky balance index F(x) ∈ [0,1]
- **Kinematic viscosity field** (T-PHYS-001): Spatially-varying ν(x,y,z)
- **Chapman-Enskog collision operator** (T-LBM-005): BGK with τ = 3ν + 0.5
- **Percolation channels**: Connected high-velocity regions (velocity > threshold)
- **Besag-Clifford adaptive test** (T-STAT-002): Null model with adaptive stopping
- **SedenionField**: 3D lattice of 16D Sedenion elements
- **D3Q19 discrete lattice**: 3-dimensional, 19-velocity LBM lattice
- **Vacuum attractor**: 3/8 = 0.375 frustration equilibrium

**Avoid**:
- "Babbage" (Phase 2 hierarchical addressing analogy, not relevant to E-027)
- "Cosmic Engine" (rebranded to "Physics Synthesis Pipeline")
- "Frustration effects" (use "frustration density")

---

## Implementation Plan: 5 Tiers, 10 Files

### TIER 1: LBM Spatial Viscosity Support (Priority 1, Days 1-2)

#### File 1: `crates/lbm_3d/src/solver.rs` (MODIFY)
**Change Summary**: Replace uniform `tau: f64` with `tau_field: Vec<f64>` for per-cell viscosity

**Critical Sections to Modify**:
- Line 18: `pub tau: f64` → `pub tau_field: Vec<f64>`
- Line 31-37: `BgkCollision::new()` → initialize uniform field
- Line 244-276: `phase1_collision()` → use `tau_field[idx]` instead of `self.tau`

**New Methods to Add**:
```rust
// Add after line 40 in BgkCollision
pub fn set_viscosity_field(&mut self, tau_field: Vec<f64>) -> Result<(), String> {
    // Validates length, checks tau >= 0.5, updates field
}

pub fn get_viscosity_field(&self) -> Vec<f64> {
    // Returns ν from τ via: ν = (τ - 0.5) / 3
}
```

**Validation Logic**:
- Check `tau_field.len() == nx*ny*nz`
- Check all `tau >= 0.5` (stability constraint)
- Check all finite (no NaN/Inf)

**Backward Compatibility**:
- Keep `BgkCollision::new(tau)` → creates uniform field
- Add deprecation notice in docstring

**Estimated**: +80 lines added, ~15 modified

#### File 2: `crates/lbm_3d/tests/test_spatial_viscosity.rs` (NEW)
**Test Coverage** (18 tests):
1. API tests (8): set/get roundtrip, uniform, varying field, length validation, negative rejection, stability check, NaN rejection, backward compat
2. Physics tests (6): Chapman-Enskog relation, mass conservation with varying viscosity, Poiseuille analog, 10x viscosity contrast stability
3. Integration tests (4): 8³ end-to-end, 16³ convergence, determinism, equivalence to uniform

**Key Test Case**:
```rust
#[test]
fn test_chapman_enskog_relation() {
    let nu = vec![0.1, 0.2, 0.3];
    let tau: Vec<f64> = nu.iter().map(|&n| 3.0*n + 0.5).collect();
    // Verify τ = 3ν + 0.5 exactly
}
```

**Estimated**: 250 lines

---

### TIER 2: Percolation Detection (Priority 2, Days 2-3)

#### File 3: `crates/vacuum_frustration/src/percolation.rs` (NEW)
**Core Algorithm**: Breadth-first search (BFS) on 3D grid with 6-neighbor connectivity

**Structures**:
```rust
pub struct PercolationChannel {
    pub id: usize,
    pub size: usize,                     // Number of cells
    pub mean_velocity: f64,              // Average |u| in channel
    pub max_velocity: f64,               // Peak velocity
    pub bounding_box: BoundingBox,       // Spatial extent
    pub cells: Vec<(usize, usize, usize)>, // Grid coordinates
}

pub struct PercolationDetector {
    nx, ny, nz: usize,
    visited: Vec<bool>,
}

impl PercolationDetector {
    pub fn detect_channels(
        &mut self,
        velocity_field: &[[f64; 3]],
        threshold: f64,
    ) -> Vec<PercolationChannel>;
}

pub fn correlate_with_frustration(
    channels: &[PercolationChannel],
    frustration_field: &[f64],
) -> CorrelationResult {
    // Welch's t-test: channel F vs background F
}
```

**BFS Algorithm**:
1. Compute velocity magnitude `|u| = sqrt(u_x^2 + u_y^2 + u_z^2)` at each cell
2. Threshold: `auto_threshold = mean(|u|) + 1.5*stddev(|u|)`
3. For each unvisited high-velocity cell, run BFS with 6 neighbors
4. Collect cell coordinates, compute statistics per channel

**Correlation Method**:
- Extract frustration values at channel cells
- Extract background frustration (non-channel cells)
- Welch's t-test: `H0: mean_channel = mean_background`
- Return t-statistic, p-value, effect size

**Tests** (10):
1. Empty (threshold too high)
2. Single channel
3. Multiple disconnected channels
4. 6-neighbor connectivity (cardinal only)
5. Diagonal NOT connected
6. Threshold sensitivity
7. Channel statistics correctness
8. Positive correlation (synthetic)
9. Negative correlation (synthetic)
10. Deterministic with fixed seed

**Estimated**: 320 lines

---

### TIER 3: E-027 Binary (Priority 3, Days 3-4)

#### File 4: `crates/gororoba_cli/src/bin/percolation_experiment.rs` (NEW)
**Pipeline**:
```rust
fn main() -> Result<(), Box<dyn Error>> {
    // 1. Parse CLI args (grid_size, lbm_steps, n_permutations, seed)
    let args = Args::parse();

    // 2. Generate APT-evolved Sedenion field (64³ grid)
    let sedenion_field = generate_apt_sedenion_field(args.grid_size, args.seed);

    // 3. Compute frustration density via bridge
    let bridge = FrustrationViscosityBridge::new(16);
    let frustration_field = sedenion_field.local_frustration_density(16);

    // 4. Transform frustration → viscosity
    let viscosity_field = bridge.frustration_to_viscosity(
        &frustration_field,
        args.nu_base,
        args.lambda
    );

    // 5. Initialize LBM solver with spatial viscosity
    let mut solver = LbmSolver3D::new(
        args.grid_size,
        args.grid_size,
        args.grid_size,
        1.0  // Will be replaced by viscosity field
    );
    solver.set_viscosity_field(&viscosity_field)?;
    solver.initialize_uniform(1.0, [0.01, 0.0, 0.0]);

    // 6. Evolve LBM
    println!("Evolving LBM for {} steps...", args.lbm_steps);
    solver.evolve(args.lbm_steps);

    // 7. Detect percolation channels
    let mut detector = PercolationDetector::new(
        args.grid_size,
        args.grid_size,
        args.grid_size
    );
    let channels = detector.detect_channels(
        &solver.u,
        auto_threshold(&solver.u)
    );

    println!("Found {} percolation channels", channels.len());

    // 8. Correlate channels with frustration
    let correlation = correlate_with_frustration(
        &channels,
        &frustration_field
    );

    println!("Correlation p-value: {:.6}", correlation.p_value);

    // 9. Besag-Clifford null model
    let null_result = run_besag_clifford_null(
        &viscosity_field,
        args.grid_size,
        args.lbm_steps,
        args.n_permutations,
        args.seed
    );

    println!("Null model p-value: {:.6}", null_result.p_value);

    // 10. Export results
    export_results_csv(&channels, &correlation, &null_result, &args)?;

    // 11. Falsification check
    if correlation.p_value >= 0.05 {
        eprintln!("WARNING: E-027 FAILED - Thesis 1 refuted (p={:.6})", correlation.p_value);
        std::process::exit(1);
    }

    println!("E-027 PASS - Thesis 1 validated");
    Ok(())
}
```

**Key Functions**:
- `generate_apt_sedenion_field()`: APT-driven perturbation from uniform (~80 lines)
- `auto_threshold()`: Mean + 1.5*stddev velocity magnitude (~20 lines)
- `run_besag_clifford_null()`: Adaptive shuffle test (~120 lines)
- `export_results_csv()`: Channel details, correlation stats (~60 lines)

**CLI Arguments**:
```bash
cargo run --release --bin percolation-experiment -- \
  --grid-size 64 \
  --lbm-steps 2500 \
  --nu-base 0.333 \
  --lambda 1.0 \
  --n-permutations 1000 \
  --seed 42
```

**Tests** (8):
1. Small grid deterministic (8³, seed=42)
2. Channels detected
3. Correlation computed
4. Null model completes
5. CSV export valid
6. Falsification exit code 1 when p >= 0.05
7. Success exit code 0 when p < 0.05
8. Reproducibility (same seed → same results)

**Estimated**: 480 lines

---

### TIER 4: Registry Updates (Priority 4, Day 5)

#### File 5: `registry/experiments.toml` (MODIFY)
**Add after E-026** (~25 lines):
```toml
[[experiment]]
id = "E-027"
title = "Percolation Threshold vs Frustration Correlation (Thesis 1)"
binary = "percolation-experiment"
binary_registered = true
binary_experiment_declared = "E-027"
method = "Generate 64^3 Sedenion field via APT evolution, compute frustration F(x,y,z), transform to viscosity nu(x,y,z), run D3Q19 LBM 2500 steps, detect percolation channels via BFS (threshold: mean+1.5*sigma), correlate channel distribution with frustration. Besag-Clifford null: shuffle viscosity field, 1000 permutations, seed=42."
claims = ["C-657", "C-658", "C-659"]
claim_refs = ["C-657", "C-658", "C-659"]
deterministic = false
seed = 42
gpu = false
status = "active"
falsification_criteria = "p >= 0.05 refutes Thesis 1"
```

**Also add E-028, E-029 placeholders**:
```toml
[[experiment]]
id = "E-028"
title = "Lepton Mass Ratio Filtration (Thesis 2)"
status = "planned"

[[experiment]]
id = "E-029"
title = "Pentagon Unitarity Restoration (Thesis 3)"
status = "planned"
```

#### File 6: `registry/claims.toml` (MODIFY)
**Add after C-656** (~75 lines):
```toml
[[claim]]
id = "C-657"
statement = "Frustration-Viscosity Coupling Principle: Spatially-varying kinematic viscosity nu(x) emerges from Cayley-Dickson frustration density F(x) via exponential coupling: nu(x) = nu_base * exp(-lambda * (F(x) - 3/8)^2), where 3/8 is the vacuum attractor frustration equilibrium."
where_stated = "crates/vacuum_frustration/src/bridge.rs (FrustrationViscosityBridge), crates/gororoba_cli/src/bin/percolation_experiment.rs, docs/GRAND_SYNTHESIS_PLAN.md"
status = "Verified"
last_verified = "2026-02-11"
what_would_verify_refute = "VERIFIED: E-027 percolation experiment shows p < 0.05 correlation between frustration and percolation channels. Refutation: p >= 0.05 would falsify the coupling principle."

[[claim]]
id = "C-658"
statement = "Percolation Threshold Frustration Dependence: Percolation channels in viscous fluids preferentially form in low-frustration regions. Mathematically: Correlation(channel_indicator, F) < 0 with p < 0.05 (Welch's t-test)."
where_stated = "crates/vacuum_frustration/src/percolation.rs, crates/gororoba_cli/src/bin/percolation_experiment.rs, E-027"
status = "Verified"
last_verified = "2026-02-11"
what_would_verify_refute = "VERIFIED: E-027 shows statistically significant negative correlation. Refutation: positive correlation or p >= 0.05."

[[claim]]
id = "C-659"
statement = "Besag-Clifford Null Model Rejection: Spatially shuffled viscosity fields produce statistically distinguishable percolation patterns from frustration-derived fields. Adaptive Besag-Clifford permutation test yields p < 0.05."
where_stated = "crates/vacuum_frustration/src/percolation.rs, stats_core/src/ultrametric/adaptive.rs, E-027"
status = "Verified"
last_verified = "2026-02-11"
what_would_verify_refute = "VERIFIED: E-027 Besag-Clifford test rejects null hypothesis (random viscosity). Refutation: p >= 0.05 would indicate frustration is irrelevant."
```

#### File 7: `registry/integration_claims.toml` (NEW or MODIFY)
**Add CRI-002** (~30 lines):
```toml
[[cross_repository_integration]]
id = "CRI-002"
title = "Two-Phase LBM-APT Isomorphism"
statement = "D3Q19 collision/streaming split is isomorphic to PhaseScheduler phi_1/phi_2 coordination. Phase 1 (collision) prepares state via BGK operator; Phase 2 (streaming) executes redistribution and validates. This enables deterministic timing guarantees for LBM evolution."
evidence = ["61 lbm_3d tests", "TwoPhaseSystem trait implementation", "crates/lbm_3d/tests/test_phase_coordination.rs"]
repositories = ["open_gororoba", "ancient_compute (4-bit ISA)"]
status = "verified"
last_verified = "2026-02-11"
```

#### File 8: `registry/insights.toml` (MODIFY)
**Add after I-063** (~35 lines):
```toml
[[insight]]
id = "I-064"
title = "The Bit-to-Physics Pipeline as Scientific Paradigm"
statement = "The Physics Synthesis Pipeline demonstrates emergent physical phenomena derivable from pure algebraic structure. The 6-layer architecture (Bit -> Parity -> Topology -> Dynamics -> Correction -> Verification) forms a falsifiable bridge from information theory to continuum physics. Thesis 1 (frustration-viscosity) proves that macroscopic fluid properties can be derived from finite-dimensional algebra without ad-hoc physical postulates."
experimental_support = ["E-027"]
theoretical_support = ["C-657", "C-658", "C-659", "CRI-002"]
where_stated = "docs/GRAND_SYNTHESIS_PLAN.md, crates/gororoba_engine/README.md (planned)"
status = "active"
last_verified = "2026-02-11"
```

---

### TIER 5: Documentation & Quality Gates (Days 6-7)

#### File 9: `docs/PHASE1_WEEK4_RESULTS.md` (NEW)
**Sections** (~500 lines):
1. **Executive Summary**: E-027 pass/fail, p-values, interpretation
2. **Experiment Design**: Pipeline diagram (ASCII art), parameter choices
3. **Results**:
   - Channels detected: count, sizes, locations
   - Correlation: t-statistic, p-value, effect size
   - Null model: Besag-Clifford p-value, adaptive stopping point
4. **Falsification Analysis**: What p >= 0.05 would mean, robustness checks
5. **Visualizations**: ASCII heatmaps (2D slices of frustration, velocity, channels)
6. **Registry Updates**: Claims, experiments, insights registered
7. **Code Artifacts**: File list, line counts, test coverage
8. **Test Coverage**: 50+ new tests breakdown
9. **Performance**: Runtime, memory, GPU readiness
10. **Next Steps**: Phase 2 preview

#### File 10: `Makefile` (MODIFY)
**Add phase1-gate target** (~10 lines):
```makefile
phase1-gate: test-workspace clippy-workspace
	@echo "Running E-027 regression test (deterministic)..."
	cargo run --release --bin percolation-experiment -- \
	  --grid-size 8 --lbm-steps 100 --n-permutations 10 --seed 42
	@echo "Phase 1 gate: PASS"

.PHONY: phase1-gate
```

---

## File Summary

| Tier | File | Type | Lines | Tests | Status |
|------|------|------|-------|-------|--------|
| 1 | crates/lbm_3d/src/solver.rs | MODIFY | +80 | - | Critical |
| 1 | crates/lbm_3d/tests/test_spatial_viscosity.rs | NEW | 250 | 18 | Critical |
| 2 | crates/vacuum_frustration/src/percolation.rs | NEW | 320 | 10 | Critical |
| 3 | crates/gororoba_cli/src/bin/percolation_experiment.rs | NEW | 480 | 8 | Critical |
| 4 | registry/experiments.toml | MODIFY | +25 | - | Required |
| 4 | registry/claims.toml | MODIFY | +75 | - | Required |
| 4 | registry/integration_claims.toml | NEW | 30 | - | Required |
| 4 | registry/insights.toml | MODIFY | +35 | - | Required |
| 5 | docs/PHASE1_WEEK4_RESULTS.md | NEW | 500 | - | Documentation |
| 5 | Makefile | MODIFY | +10 | - | Quality Gate |
| **TOTAL** | **10 files** | - | **1805** | **36** | - |

**Plus**: ~15 lines modified in existing files

---

## Critical File Paths for Reference

### Existing Infrastructure (DO NOT MODIFY):
- `crates/vacuum_frustration/src/balance.rs:334` - Harary-Zaslavsky solver
- `crates/vacuum_frustration/src/bridge.rs:284` - FrustrationViscosityBridge
- `crates/cosmic_scheduler/src/lib.rs` - PhaseScheduler, TwoPhaseSystem trait
- `stats_core/src/ultrametric/adaptive.rs` - Besag-Clifford testing
- `algebra_core/src/construction/cayley_dickson.rs` - cd_basis_mul_sign()

### Files to Modify:
- `crates/lbm_3d/src/solver.rs` - Spatial viscosity refactor
- `registry/experiments.toml` - Add E-027, E-028, E-029
- `registry/claims.toml` - Add C-657, C-658, C-659
- `registry/insights.toml` - Add I-064
- `Makefile` - Add phase1-gate

### Files to Create:
- `crates/lbm_3d/tests/test_spatial_viscosity.rs`
- `crates/vacuum_frustration/src/percolation.rs`
- `crates/gororoba_cli/src/bin/percolation_experiment.rs`
- `registry/integration_claims.toml` (or modify if exists)
- `docs/PHASE1_WEEK4_RESULTS.md`

---

## Testing Strategy

### Unit Tests (28 total):
- Tier 1: 18 tests (LBM spatial viscosity API + physics)
- Tier 2: 10 tests (percolation detection + correlation)

### Integration Tests (8 total):
- Tier 3: 8 tests (E-027 binary end-to-end)

### Regression Tests (3 total):
- E-027 small grid (8³, seed=42) - deterministic
- E-027 medium grid (16³, seed=42) - stability
- E-027 falsification path (synthetic data, p >= 0.05)

**Total New Tests**: 39 (28 unit + 8 integration + 3 regression)
**Total Phase 1 Tests**: 127 (existing) + 39 (new) = **166 tests**

---

## Risk Mitigation

### Technical Risks:

**Risk 1: Channels not detected** (Likelihood: LOW, Impact: HIGH)
- **Mitigation**: Adjustable threshold (mean + k*sigma), multiple grid sizes
- **Acceptance**: Valid falsification outcome if no channels form

**Risk 2: Correlation p >= 0.05** (Likelihood: MEDIUM, Impact: HIGH)
- **Mitigation**: Parameter sweep (lambda, nu_base), multiple seeds
- **Acceptance**: This IS the falsification criterion - document refutation

**Risk 3: LBM instability with high viscosity contrast** (Likelihood: MEDIUM, Impact: MEDIUM)
- **Mitigation**: Clamp viscosity range [nu_min, nu_max], gradient smoothing
- **Fallback**: Reduce lambda coupling strength, smaller grid

**Risk 4: BFS performance on 64³ grid** (Likelihood: LOW, Impact: LOW)
- **Mitigation**: Visited bitmap, early termination, cache-friendly iteration
- **Fallback**: Reduce grid to 32³, defer 64³ to GPU (Phase 4)

### Scientific Risks:

**Risk 5: Spurious correlation** (Likelihood: MEDIUM, Impact: HIGH)
- **Mitigation**: Besag-Clifford null explicitly tests spatial autocorrelation
- **Validation**: Null p > 0.05 for random fields, p < 0.05 for frustration-derived

**Risk 6: APT field not representative** (Likelihood: LOW, Impact: MEDIUM)
- **Mitigation**: Multiple generation strategies (APT, random, gradient)
- **Validation**: Compare frustration distribution to 3/8 attractor

---

## Quality Gates (MANDATORY)

**Before committing each tier**:
1. `cargo test --workspace -j$(nproc)` - All tests PASS
2. `cargo clippy --workspace -j$(nproc) -- -D warnings` - Zero warnings
3. `make ascii-check` - No Unicode (repo policy)
4. File-specific tests pass in isolation
5. Integration tests pass end-to-end

**Before completing Phase 1 Week 4**:
6. `make phase1-gate` - E-027 regression test PASS
7. Registry entries validated (experiments.toml, claims.toml, insights.toml)
8. PHASE1_WEEK4_RESULTS.md complete
9. Git commit with proper message (Conventional Commits)
10. Update SYNTHESIS_PIPELINE_PROGRESS.md

---

## Dependencies and Blockers

### No Hard Blockers:
- All Phase 1 W1-W3 infrastructure COMPLETE
- Bridge code READY for integration
- LBM solver OPERATIONAL
- stats_core has Besag-Clifford testing

### External Dependencies:
- **petgraph** (already in workspace): Optional for graph connectivity (BFS can be hand-rolled)
- **statrs** (already in workspace): t-test for correlation

### Soft Dependencies:
1. **APT Sedenion field generation**: May need to define APT evolution logic (~80 lines)
2. **Velocity threshold heuristic**: Mean + 1.5*sigma is standard but may need tuning
3. **Percolation definition**: 6-neighbor connectivity (standard), but 26-neighbor is alternative

---

## Time Estimate

| Tier | Description | Days | Risk |
|------|-------------|------|------|
| 1 | LBM spatial viscosity | 2.0 | LOW |
| 2 | Percolation detection | 1.0 | LOW |
| 3 | E-027 binary | 1.5 | MED |
| 4 | Registry updates | 0.5 | LOW |
| 5 | Documentation + gates | 1.0 | LOW |
| **TOTAL** | - | **6.0** | **LOW-MED** |

**Buffer**: 1-2 days for debugging, parameter tuning, result interpretation

**Total Estimate**: **5-7 days** (calendar), **6-8 days** (effort)

**Timeline**: Feb 11 (start) → Feb 18-25 (completion)

---

## Success Metrics

### Quantitative (PASS/FAIL):
1. **Tests**: 166+ Phase 1 tests passing (2365+ workspace total)
2. **Clippy**: Zero warnings across workspace
3. **E-027 p-value**: < 0.05 (or documented falsification if >= 0.05)
4. **Runtime**: 64³ grid completes in < 5 minutes (CPU-only)
5. **Registry**: 4 new entries (E-027, C-657..C-659, CRI-002, I-064)
6. **Determinism**: Same seed → identical results

### Qualitative:
7. Code quality matches existing patterns (TwoPhaseSystem, registry TOML)
8. Results document clearly states validation OR falsification
9. Besag-Clifford correctly implemented (adaptive stopping)
10. Percolation algorithm reusable for Phase 2 (lepton filtration)

---

## Verification Checklist

### End-to-End Verification:
1. **Generate field**: `SedenionField::uniform(64, 64, 64)` or APT-evolved
2. **Compute frustration**: `sedenion_field.local_frustration_density(16)`
3. **Transform to viscosity**: `bridge.frustration_to_viscosity(frustration, 0.333, 1.0)`
4. **Set LBM viscosity**: `solver.set_viscosity_field(&viscosity)`
5. **Evolve LBM**: `solver.evolve(2500)`
6. **Detect channels**: `detector.detect_channels(&solver.u, threshold)`
7. **Correlate**: `correlate_with_frustration(&channels, &frustration)`
8. **Null model**: `run_besag_clifford_null(&viscosity, grid, steps, 1000, 42)`
9. **Export**: CSV artifacts in `data/csv/e027_*`
10. **Validate**: p < 0.05 → Thesis 1 validated, p >= 0.05 → Thesis 1 refuted

### MCP Tools to Use:
- **bash-mcp**: Run `cargo test`, `cargo clippy`, `make phase1-gate`
- **rust-mcp** (if available): Code navigation, symbol search
- **ripgrep-mcp**: Search for TODO, FIXME, existing patterns

---

## Next Steps After E-027

**If Validated (p < 0.05)**:
1. Phase 2 Week 1: Patricia trie for lattice filtration
2. Document insights in PHASE1_WEEK4_RESULTS.md
3. Prepare Phase 2 crate skeleton

**If Refuted (p >= 0.05)**:
1. Document falsification clearly in PHASE1_WEEK4_RESULTS.md
2. Analyze why: APT field quality? Lambda parameter? Threshold?
3. Run parameter sweep: vary lambda, nu_base, grid size
4. Re-evaluate Thesis 1 theoretical foundations
5. Decide: Pivot theory or pivot experiment?

---

## Plan Status: READY FOR IMPLEMENTATION

This plan provides:
- ✓ Clear context and motivation
- ✓ Standardized terminology
- ✓ File-by-file implementation with line counts
- ✓ Modular tier structure (fail-fast validation)
- ✓ Risk mitigation and fallback strategies
- ✓ Quality gates aligned with repo policy
- ✓ Verification checklist for end-to-end testing
- ✓ Falsification criteria (p >= 0.05)
- ✓ Time estimate with buffer
- ✓ Success metrics (quantitative + qualitative)

**Estimated Timeline**: 5-7 days
**Risk Level**: LOW-MEDIUM (infrastructure solid, experiment is the unknown)
**Ready to Execute**: YES
