# Roadmap

Guiding rules:
- Warnings are errors for builds, tests, and linting.
- Sync with `mcs4-emu/STATUS.md` after each milestone.
- Update docs and install requirements with each major change.

## Phase 0 - Repo hygiene and reproducibility (Now)
- Canonical workspace root is the repo root; configs reconciled.
- Normalize `Cargo.toml`, `rustfmt.toml`, `deny.toml`, and `.cargo` settings.
- Define artifact destinations (target/coverage) and add clean scripts.
- Expand `.gitignore` for core dumps, coverage, profiling, and analysis output.
- Update `mcs4-emu/INSTALLATION.md` with per-crate requirements.
- Normalize docs registry/INDEX and CI doc checks.
- Track workspace lint inheritance and MSRV across all crates.
- Add primary-source citations to `docs/AUDIT.md` for CPU specs.

## Phase 1 - CPU correctness and instruction coverage
- Complete 4040 CPU: register banks, 7-level stack, interrupts, and new opcodes.
- Harden 4004/4040 decode paths with golden vectors and fuzz regression tests.
- Disassembler correctness (4004 + 4040) with unit tests and ROM fixtures.
- Implement 4040 test pin behavior and JCN/ISZ edge cases.

## Phase 2 - Support chips and system integration
- Implement bus-accurate 4003, 4101, 4201, 4289, 4308 protocols.
- Finish MCS-4/MCS-40 system integration in `mcs4-system`.
- Cluster I/O wiring: implement ROM/RAM port reads/writes in `Cluster`.
- 4308 bus protocol and timing detail coverage.

## Phase 3 - Debugger and UI consolidation
- GUI: waveform viewer refinement and disassembly synchronization.
- Add CLI and TUI entrypoints; keep shared debugger controller.
- ROM loading, stepping, breakpoints, and trace export.

## Phase 4 - Performance and clustering
- SIMD cluster execution plan (std::simd) with deterministic scheduling.
- rkyv snapshots for time-travel and reproducible benchmarking.
- Benchmark suite with CI thresholds.

## Phase 5 - FPGA + transistor-level fidelity
- Verilog export with gate-level netlist generation.
- Transistor-level simulation model (event-driven or nodal analysis).
- Populate inverter/NAND transistor models and simulation parameters.

## Cross-cutting
- Documentation audit + source validation (claims and specs).
- Security/license policy via `cargo-deny` and dependency audits.
- Expand ARCHITECTURE.md with canonical workspace layout and module map.
