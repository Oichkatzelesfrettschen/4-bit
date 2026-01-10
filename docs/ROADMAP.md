# Roadmap

Guiding rules:
- Warnings are errors for builds, tests, and linting.
- Sync with `mcs4-emu/STATUS.md` after each milestone.
- Update docs and install requirements with each major change (`requirements.md`, `mcs4-emu/INSTALLATION.md`).

## Phase 0 - Repo hygiene and reproducibility (Complete)
- Canonical workspace root is the repo root; configs reconciled and `docs/meta/registry.yaml` mirrors every new doc.
- Normalized `Cargo.toml`, `rustfmt.toml`, `deny.toml`, and `.cargo` settings plus artifact destinations (`target/coverage`, etc.).
- Expanded `.gitignore` to absorb core dumps, coverage artifacts, profiling, and other temporary outputs.
- Updated `mcs4-emu/INSTALLATION.md` with per-crate requirements and the new tooling audit checklist.
- Added reproducible tooling inventory (`docs/TOOLING_AUDIT.md`) and audit script (`scripts/tooling_audit.sh`).
- Recorded evidence tooling steps in `docs/evidence/README.md`, `docs/evidence/ocr_manifest.yaml`, and the new `docs/evidence/PROVENANCE_CHECKLIST.md`.
- Chunked OCR for the 1975 Intel Data Catalog and captured datasheet timing claims from reliable sources.
- Cataloged schematics/mask layers/die shots, documented license gaps, and introduced `scripts/generate_layer_overlays.py` for reproducible overlays.
- Rust toolchain locked to nightly-2026-01-06 with edition/workspace defaults targeting Rust **2021** + MSRV 1.92.0; warnings-as-errors enforced via clippy/deny.
- Cataloged transistor counts from the 4004 analyzer, documented missing 4040 die shots, and recorded provenance for each imported image.

## Phase 0.5 - Evidence & photomicrograph audit (In Progress)
- Track outstanding primary sources (Intel users manuals, catalogs, mask shots) and capture supplemental OCR/transcription notes.
- Continue hunting vetted 4040/MCS-40 die shots or mask-layer imagery; add provenance notes before importing.
- Maintain the photomicrograph registry (`docs/photomicrographs/`, `docs/evidence/photomicrograph_permissions.md`) with SHA256 and license notes.
- Expand the netlist extraction workflow with analyzer notes and annotate which layer intersections remain undocumented.
- Align `docs/AUDIT.md`, `ARCHITECTURE.md`, and `docs/CHIP_ARTIFACTS.md` with the evidence inventory; highlight gaps remaining for 4040/4289/4308.
- Keep `docs/ROADMAP.md` and `mcs4-emu/STATUS.md` synchronized after each milestone; log toolchain and documentation pivots explicitly.

## Phase 1 - CPU correctness and instruction coverage
- Complete 4040 CPU: register banks, 7-level stack, interrupts, and new opcodes.
- Harden 4004/4040 decode paths with golden vectors and fuzz regression tests.
- Disassembler correctness (4004 + 4040) with unit tests and ROM fixtures.
- (Done) Implement 4040 TEST pin behavior for `JCN` condition bit 0.
- Resolve 4040 chip-select/test-pin TODOs in `crates/mcs4-chips/src/i4040/`.

## Phase 2 - Support chips and system integration
- Implement bus-accurate 4003, 4101, 4201, 4289, 4308 protocols.
- Finish MCS-4/MCS-40 system integration in `mcs4-system`.
- (Done) Cluster I/O wiring: ROM/RAM port reads in `Cluster`.
- (Done) Gate RAM/ROM side-effects on decoded I/O ops (avoid “always-on” behavior) and add ROM `.hex` fixtures.
- 4308 bus protocol and timing detail coverage.
- Close TODOs in `mcs4-system/src/cluster.rs` and `mcs4-chips/src/i4308.rs`.

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
- Resolve `transistor.rs` TODOs in `mcs4-core` and ARCHITECTURE stubs.
- Replace placeholder netlist output in `mcs4-fpga/src/verilog.rs` with gate-level export.

## Cross-cutting
- Documentation audit + source validation (claims and specs).
- Security/license policy via `cargo-deny` and dependency audits.
- Expand ARCHITECTURE.md with canonical workspace layout and module map.
- Keep `docs/ROADMAP.md` aligned with `mcs4-emu/STATUS.md` and TODO scans (`scripts/todo_scan.sh` → `docs/TODO.md`).
