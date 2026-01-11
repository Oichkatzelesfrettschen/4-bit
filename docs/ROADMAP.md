# Roadmap

Guiding rules:
- Warnings are errors for builds, tests, and linting.
- Sync with `mcs4-emu/STATUS.md` after each milestone.
- Update docs and install requirements with each major change (`requirements.md`, `mcs4-emu/INSTALLATION.md`).

## Reality Check (Current State)

### Implemented / working now
- Cycle-accurate MCS-4 core system with 4004/4001/4002 integrated and fixtures runnable via `mcs4-emu --mode fixture`.
- Phase-accurate I/O control: `io_op` asserted only during transfer phases (X2 writes, X3 reads; `SRC` spans X2+X3) with
  bus-stability assertions and strict fixture checks.
- Evidence tooling:
  - OCR sidecars for PDFs (`docs/evidence/ocr/` + `docs/evidence/ocr_manifest.yaml` + `docs/evidence/ocr_results.md`).
  - Schematic OCR for i400x bitmaps (`docs/evidence/ocr_schematics/`).
  - Coordinate label OCR verification for `signals.txt` (`docs/evidence/ocr_signal_labels/`).
  - Transistor *candidate* extraction via poly∩diffusion intersections (`docs/evidence/transistors/`), cross-checked against
    analyzer-reported 4004 transistor counts (Δ currently -3).

### Not implemented yet (key blockers)
- No full analyzer-grade netlist extraction + reconciliation:
  - `netlist_v0` layout stitching exists (`docs/evidence/netlists_v0/`), but it does not yet extract a schematic netlist nor match schematic↔layout nets.
- No transistor-/switch-level solver consuming extracted devices; `mcs4-core/src/transistor.rs` remains a stub model.
- 4040 CPU remains a stub; MCS-40 support chips are incomplete (4101/4201/4289/4308 protocols, etc.).

## What Must Be Obtained (Primary Sources / Artifacts)

Track these in `docs/evidence/PRIMARY_SOURCES_BACKLOG.md` and record provenance/licensing in
`docs/evidence/photomicrograph_permissions.md` before importing:
- 4040 die shot / mask-layer imagery (enables the same kind of transistor evidence we have for 4004).
- Primary confirmation for 4004/4040 transistor counts (current sources are secondary + analyzer forensic counts).
- Primary, explicit 4040 max-clock figure (current value is derived from clock period; need an explicit spec quote).
- MCS-40 support chip silicon/layer artifacts (4101/4201/4289/4308) and higher-resolution board-level schematics.

## What Must Be Converted To Text (To Move Forward)

The project makes progress fastest when evidence becomes searchable + diffable. Conversion targets:
- **Primary PDFs** → OCR sidecar text:
  - Use `ocrmypdf --sidecar` and record outputs in `docs/evidence/ocr_manifest.yaml` + `docs/evidence/ocr_results.md`.
  - Extract and quote exact lines for claims in `docs/AUDIT.md`; keep derived/pending items in `docs/evidence/audit_claims_backlog.md`.
- **Schematic labels** → deterministic OCR verification:
  - Use `scripts/ocr_signal_labels.py` to verify `docs/emulators/i400{1,2,3,4}-signals.txt` coordinates against printed labels.
  - Curate alias mappings in `scripts/ocr_signal_aliases.json` for cases where the schematic prints pin numbers (e.g. `CLK1` vs `01`).
- **Analyzer readme / netlist notes** → structured extracts:
  - Keep analyzer-derived component counts and quirks referenced in `docs/evidence/ocr_results.md` and `docs/NETLIST_WORKFLOW.md`.
- **Future (netlist extraction)** → machine-readable netlist JSON:
  - Build on `netlist_v0` by adding schematic-netlist extraction + schematic↔layout matching + switch-level solving.

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
- Keep OCR pipelines bounded and deterministic (timeouts, caching) and preserve canonical evidence outputs under `docs/evidence/`.

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
- (Done) Gate RAM/ROM side-effects on decoded I/O ops (avoid “always-on” behavior), implement realistic `SRC` bus nibbles, and add end-to-end `.hex` fixtures (+ CLI fixture runner mode); refine to phase-accurate control assertion (I/O op asserted only during X2/X3 transfer phases).
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
- Prerequisite: build a deterministic multi-layer connectivity graph from `docs/emulators/i400x-*.bmp` (metal/poly/diffusion + vias/contacts),
  then emit a machine-readable netlist and use it to validate control-line timing and selected subcircuits against emulator traces.

## Cross-cutting
- Documentation audit + source validation (claims and specs).
- Security/license policy via `cargo-deny` and dependency audits.
- Expand ARCHITECTURE.md with canonical workspace layout and module map.
- Keep `docs/ROADMAP.md` aligned with `mcs4-emu/STATUS.md` and TODO scans (`scripts/todo_scan.sh` → `docs/TODO.md`).
