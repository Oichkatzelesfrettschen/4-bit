# Roadmap

Guiding rules:
- Warnings are errors for builds, tests, and linting.
- Sync with `mcs4-emu/STATUS.md` after each milestone.
- Update docs and install requirements with each major change (`requirements.md`, `mcs4-emu/INSTALLATION.md`).
- Keep fidelity goals explicit: see `docs/ACCURACY_PROGRAM.md` and `docs/CLAIMS_TO_TESTS.md`.

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
- Schematic connectivity extraction is still missing (component recognition on `i400x-schematic.bmp`; `i400x-schematic.png` is the preview copy), but we now have:
  - Schematic net-name artifacts (`docs/evidence/schematic_net_names_v0/`) joined with OCR evidence.
  - A schematic↔layout matching scaffold (`docs/evidence/schematic_layout_match_v0/`) driven by manual anchors + layout node stats.
- Pad anchoring is now partially tractable:
  - `netlist_v0` includes per-node bboxes/areas and pad-like node ranking (`docs/evidence/layout_pad_candidates_v0/`).
  - Geometry-based node suggestions for pad label boxes exist under `docs/evidence/layout_pad_labels_v0/`.
- Transistor/switch-level solvers exist in `mcs4-core/src/transistor_solver.rs` and `mcs4-core/src/nodal_solver.rs` (477 tests in mcs4-core); solver-to-chip bridge connects behavioral models to circuit simulation via `ChipSolverBridge` trait.
- 4040 CPU is COMPLETE (60 instructions, 43 tests); all MCS-40 support chips COMPLETE (4201/4289/4308 + 10 additional chips).
- SIMD cluster COMPLETE: 16-lane parallel 4004 execution with full 46-instruction ISA, differential fuzzing, and benchmarking (45 tests in mcs4-system).

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
- Maintain OCR micro-benchmarks for tiny label crops (`docs/evidence/ocr_benchmarks_v0/`) and a larger labeled-crops manifest (`docs/evidence/ocr_manifests_v0/`) so changes can be evaluated for both accuracy and throughput.
- Keep edge-label OCR fast by default (`scripts/detect_layout_edge_labels_v0.py`; `--deep` only for debugging, `--cuda-preproc` optional, `--backend auto` for ONNX experiments).
- Maintain a minimal schematic↔layout bridge artifact (`docs/evidence/netlists_v1/`) so anchor signals can be traced end-to-end through extraction outputs.
- Keep schematic connectivity tracing explicitly heuristic until component-aware parsing exists (`docs/evidence/schematic_connectivity_v0/`).
- Extract bounded transistor subcircuits around anchor nodes (`docs/evidence/subcircuits_v0/`) and track how many anchors currently reach device candidates; treat “0-transistor anchors” as a first-class extraction lacuna.
- Anchor-node reality check (resolved for 4004 anchors that have layout nodes):
  - Historical baseline: `docs/evidence/anchor_incidence_v0_strict/4004/` showed **10/11** 4004 anchors had **0 incident transistors**.
  - Remapped anchors: `docs/evidence/schematic_layout_anchors_v1.json` + `docs/evidence/anchor_incidence_v1_canonical/4004/` now show **0/11** anchors with zero incident transistors.
  - Remaining lacunae: validate pad↔pin naming around external reset (`POC_PAD` vs primary-source `RESET`) and audit any anchors that land on terminal-only incidence for control lines.
- Prioritize a small set of “anchor” subcircuits to validate end-to-end across evidence → netlist → simulation:
  - Clock + SYNC generation (`CLK1`, `CLK2`, `SYNC`) and their pad drivers.
  - Program/data bus pads (`D0..D3`) including input protectors and bus buffers.
  - Memory control outputs (`~CM-ROM`, `~CM-RAM`, `~SYNC`, `~WR`, `~RD`) and their gating logic.
  - Reset / test-related logic (noting the analyzer readme’s TEST-pin revision mismatch).

### Roadmap snapshot (discovered / active / next)
1. **What we have discovered & completed:**
   - Bounding boxes and anchors for C/T/S pads now synchronized across 4002/4003, and their overlays committed (`docs/evidence/anchor_remap_overlays_v1/`).
   - Manual pad-reading infrastructure (`docs/evidence/layout_pad_labels_v0/*/manual_readings_v0.md`) now captures OCR/angle data, and `pad_pin_template_v0.md` drafts exist for each chip.
   - OCR tooling plans with CUDA/ONNX/pytesseract fallbacks documented and ready for incremental training, including micro-benchmarks under `docs/evidence/ocr_benchmarks_v0/`.
   - Anchor status dashboards (`docs/evidence/LACUNAE_STATUS.md`, `docs/evidence/ANCHOR_COVERAGE_V0.md`) refreshed to record zero-incidence gaps now resolved for 4004 anchors.
   - **2026-01-14**: Power rail anchors (VSS/VDD) for 4001/4002/4003 upgraded from low to **medium confidence** via pad geometry corroboration against PRIMARY_SOURCE_PINOUTS.md.
   - **2026-01-14**: Remap -> incidence -> subcircuit extraction pipeline now complete for ALL chips (4001/4002/4003/4004).
   - **2026-01-14**: Subcircuit metrics generated: 4001 (11 subcircuits, max 117 transistors), 4002 (6 subcircuits, max 42 transistors), 4003 (5 subcircuits, max 9 transistors).
   - **2026-01-14**: CI schematic pipeline passes all checks (anchor audit, pad consistency, incidence, uniqueness).

### Phase 4 - Clustering and Performance (100% Complete - 13/13 tasks)
Status: COMPLETE (2026-02-25)
Key Milestones:
   - **2026-01-29**: Hierarchical clustering system complete - 3-level hierarchy (individual subcircuits, electrical connectivity clusters, functional blocks).
   - **2026-01-29**: Clustering outputs generated for all 4 chips: 4001 (11->1->1), 4002 (6->1->1), 4003 (5->1->1), 4004 (19->6->3).
   - **2026-01-29**: SIMD cluster design complete - 16-lane vectorization strategy with masked execution for control flow.
   - **2026-01-29**: Benchmark suite implemented with CI integration and 20% regression threshold.
   - **2026-01-29**: Performance infrastructure established with baseline comparison and fixture benchmarking.
   - **2026-02-25**: SIMD cluster full ISA implementation - all 46 4004 instructions in 16-lane parallel execution.
   - **2026-02-25**: Two-byte instruction infrastructure - per-lane two-phase fetch for JUN/JMS/JCN/ISZ/FIM.
   - **2026-02-25**: Differential fuzzing - scalar reference executor + proptest harness proving SIMD == scalar.
   - **2026-02-25**: Solver-to-chip bridge - SimulationFidelity enum, ChipSolverBridge trait, I4004 clock buffer PoC.
   - **2026-02-25**: Process model expansion - I/O driver, power, ESD, ROM cell, SRAM cell (22 new tests).

Deferred to Phase 5+:
   - rkyv snapshots and time-travel debugging
   - Advanced clustering optimizations (spatial and adaptive strategies)

### Phase 5 - FPGA and Advanced Features (75% Implemented)
Status: IMPLEMENTED (peripherals, Intellec-4, Verilog chips complete; hardware validation deferred)
Key Milestones:
   - **2026-01-29**: Verilog export architecture complete - gate-level netlist to synthesizable HDL.
   - **2026-01-29**: Verilog generator implemented - 8 files generated (4 modules + 4 testbenches) for all chips.
   - **2026-01-29**: FPGA synthesis workflow documented - Lattice iCE40 (open-source) and Xilinx Spartan-7 (proprietary) targets.
   - **2026-01-29**: Peripheral interface design complete - 7-segment, Nixie tubes, matrix keyboard, serial UART.
   - **2026-01-29**: OCR training pipeline designed - Conv+LSTM+CTC architecture targeting >98% accuracy.
   - **2026-01-29**: Complete design documentation - ready for hardware validation and peripheral implementation.

Deferred Work:
   - Hardware validation with physical FPGA boards (requires hardware purchase)
   - Peripheral driver implementation (7-segment, keyboard, UART)
   - Custom ONNX CTC model training (requires dataset collection)

2. **What must be done immediately:**
   - (DONE) Map the remaining SYNC / POC_PAD / TEST_PAD anchors from the bounding-box sync outputs and commit them into `docs/evidence/schematic_layout_anchors_v1.json` so the remap/incidence pipeline can consume them.
   - (DONE) Finish documenting pad readings for 4001/4002/4003, flagging low-confidence rails/pads, and finalize the pad-pin templates (rationalized naming + offsets).
   - (DONE) Run remap -> incidence -> subcircuit extraction for 4001/4002/4003, validate netlists against schematic layouts, and record any mismatches in `docs/evidence/anchor_incidence_v0/` plus `subcircuits_v0/` outputs.
   - (IN PROGRESS) Update the living docs (`docs/ROADMAP.md`, `docs/CHIP_EXTRACTION_STATUS.md`, `docs/evidence/LACUNAE_STATUS.md`) after each batch of anchor/pad work so the roadmap reflects actual dependencies and next focus areas.
3. **What must be built next:**
   - Deploy the enhanced OCR toolchain (CUDA + ONNX + pytesseract fallback) and train it on the collected pad label crops so future anchor detection is ensemble-backed and self-validating.
   - Translate the remapped anchors/subcircuits into a transistor-aware netlist that can feed the transistor/switch solver in `mcs4-core/src/transistor.rs` and the future FPGA exporter.
   - Expand the plan's 50-step cycle with tooling milestones (self-training OCR, anchor propagation automation, fixture validation) and keep the status snapshots (Roadmap, CHIP_EXTRACTION_STATUS, LACUNAE_STATUS) up to date.
   - (DONE) Phase 1 (4040 CPU): register banks, 7-level stack, interrupts, and 14 new opcodes complete.

## Phase 1 - CPU correctness and instruction coverage (COMPLETE)
- [DONE] 4040 CPU: 60 instructions, register banks, 7-level stack, interrupts (43 tests).
- [DONE] 4004 CPU: 46 instructions, full ALU, registers, 3-level stack (115+ tests).
- Harden 4004/4040 decode paths with golden vectors and fuzz regression tests.
- [DONE] Disassembler correctness (4004 + 4040) with unit tests and ROM fixtures.
- [DONE] Implement 4040 TEST pin behavior for `JCN` condition bit 0.
- Resolve 4040 chip-select/test-pin TODOs in `crates/mcs4-chips/src/i4040/`.

### Chip Implementation Priority

- **P0 (Done):** 4004, 4001, 4002, 4003
- **P1 (Done):** 4040, 4101, 4201, 4289, 4308 (bus protocol + proptest)
- **P2 (Done):** 4008, 4009, 3216, 3226, 4207, 4209, 4211, 4265, 4316, 4702
- **P3+ (Deferred):** 4002-1/2, 4269, 3205, 3404, 2101, 2102, 1302, TTL glue, peripherals

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

## Phase 4 - Performance and clustering (100% COMPLETE)
COMPLETED (2026-01-29 through 2026-02-25):
- [DONE] Hierarchical clustering strategy (electrical + functional grouping).
- [DONE] Cluster extraction for all 4 chips with 3-level hierarchy.
- [DONE] SIMD cluster design document with 16-lane vectorization architecture.
- [DONE] Benchmark suite implementation with baseline comparison.
- [DONE] CI integration for performance regression detection (20% threshold).
- [DONE] Cluster validation (100% coverage, no overlaps).
- [DONE] Performance metrics documentation and reporting.
- [DONE] SIMD cluster full 4004 ISA (46 instructions, 87 tests with feature gate).
- [DONE] Two-byte fetch infrastructure (per-lane two-phase fetch).
- [DONE] Differential fuzzing (scalar reference executor + proptest, SIMD == scalar).
- [DONE] Solver-to-chip bridge (SimulationFidelity, ChipSolverBridge, I4004 clock buffer PoC).
- [DONE] Process models: I/O driver, power, ESD, ROM cell, SRAM cell (22 new tests).

DEFERRED (Future Work):
- rkyv snapshots for time-travel and reproducible benchmarking.
- Spatial clustering and adaptive strategies.
- Hardware-in-loop testing and co-simulation.
- Advanced performance optimizations.

## Phase 5 - FPGA + transistor-level fidelity (75% IMPLEMENTED)
Design COMPLETE (2026-01-29), implementation (peripherals/Intellec-4/Verilog) COMPLETE (2026-02-25):
- [DONE] Verilog export architecture document (450 lines) - complete design specification.
- [DONE] Gate-to-Verilog converter implementation (280 lines Python) - functional generator.
- [DONE] Verilog module generation for all 4 chips (8 files: 4 modules + 4 testbenches).
- [DONE] Inline primitive library (INV, NAND2/3, NOR2/3, TGATE) in generated Verilog.
- [DONE] FPGA synthesis workflow documentation (520 lines) - complete toolchain guide.
- [DONE] Target platform specifications: Lattice iCE40HX4K and Xilinx Spartan-7.
- [DONE] Open-source toolchain path: Yosys + nextpnr-ice40 + icestorm.
- [DONE] Proprietary toolchain path: Xilinx Vivado with TCL automation.
- [DONE] Resource utilization estimates and timing analysis for all chips.
- [DONE] Peripheral interface design (130 lines) - 7-segment, Nixie, keyboard, UART.
- [DONE] OCR training pipeline design (90 lines) - Conv+LSTM+CTC architecture.
- [DONE] Complete design phase documentation with clear roadmaps for implementation.

DEFERRED (Future Work - Requires Hardware):
- Hardware validation with physical FPGA boards (iCE40 or Spartan-7 purchase required).
- FPGA bitstream synthesis and programming on actual hardware.
- Hardware-in-loop testing and differential validation vs software emulator.
- Peripheral driver implementation (7-segment display, matrix keyboard, serial UART).
- Custom ONNX CTC model training (requires 200+ labeled training crops).
- Multi-chip FPGA system integration (4004 + ROM + RAM on single FPGA).
- Transistor-level simulation model implementation (event-driven or nodal solver).
- Transistor model parameters (BSIM4 or simple switch models).
- Resolve `transistor.rs` TODOs in `mcs4-core`.
- Via connectivity modeling and parasitic extraction for timing accuracy.

## Cross-cutting
- Documentation audit + source validation (claims and specs).
- Security/license policy via `cargo-deny` and dependency audits.
- Expand ARCHITECTURE.md with canonical workspace layout and module map.
- Keep `docs/ROADMAP.md` aligned with `mcs4-emu/STATUS.md` and TODO scans (`scripts/todo_scan.sh` → `docs/TODO.md`).
