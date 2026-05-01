# mcs4-core -- Requirements

> Simulation kernel: gate primitives, switch-level transistor solver, nodal
> analysis solver, full SPICE-class solver stack (DC, AC, transient,
> sensitivity, noise, temperature sweep), Intel 10 um pMOS process and device
> models, TCAD physics (Poisson + drift-diffusion), circuit graph, parasitic
> extraction, and the multi-fidelity bridge. 504 tests across the crate plus
> integration suites.

## Build

```sh
cargo build -p mcs4-core
cargo test  -p mcs4-core
cargo clippy -p mcs4-core --all-targets -- -D warnings
cargo +nightly miri test -p mcs4-core
```

## Toolchain and lints

- Nightly pin: `nightly-2026-04-05` (workspace).
- MSRV stable: 1.92.0.
- Miri-clean (already covered in CI for this crate).

## Features

None today. Future phases may introduce `pathological_circuits` (D6 plan) and
`spice_raw_export` (D10 plan) gates.

## Rust dependencies (workspace-pinned)

- `nalgebra` -- dense matrix backend for small MNA stamps.
- `faer` -- sparse LU factorization (auto-selected above 100 free nodes).
- `bumpalo` -- per-step arena allocation for solver temporaries.
- `serde`, `serde_json`, `rkyv`, `bytemuck`, `zerocopy` -- snapshot and netlist
  parsing.
- `smallvec`, `tracing` -- diagnostics and small-buffer optimization.
- `proptest` (dev) -- planned solver property tests.

## System packages

None. Solvers are pure Rust.

## Known gotchas

- `transistor::CircuitBuilder` is `#[deprecated]` in favour of
  `transistor_solver::TransistorSimulator`. The 8 `#[allow(deprecated)]`
  annotations inside `src/transistor.rs` are intentional: they keep tests
  for the legacy builder API alive for backward-compat coverage. No external
  crate depends on `CircuitBuilder`. Full removal is tracked under debt
  phase D1.3 in `~/.claude/plans/elucidate-and-build-out-merry-gadget.md`.
- `TransientSolver` supports all three integrators (`BackwardEuler`,
  `Trapezoidal`, `TRBDF2`). The dispatch lives at
  `src/solver/transient.rs:276-279` and TRBDF2 composes BE + Trap stages
  at `:429-435`. LTE-driven adaptive stepping is wired at `:556-563`.
  An earlier audit incorrectly claimed Trap/TRBDF2 were unwired; that
  finding was retracted on 2026-04-30.
- `SolverConfig` field is `max_nr_iterations` (not `max_iterations`).
- `DcSolver::new(config, process).solve(&mut graph)` returns `DcOpResult`
  directly (not `Result`); inspect `result.converged`.
- Nodal solver: `gmin` (default `1e-9`) shifts voltage divider results by
  ~`1e-5`; tests use a `1e-4` tolerance.
- For `cargo +nightly udeps`, run from workspace root (deps are inherited).
