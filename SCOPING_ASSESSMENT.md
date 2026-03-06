# MCS-4 / MCS-40 Ecosystem: Complete Circuit-Level Replica Scoping Assessment

**Date**: 2026-03-05
**Type**: Read-only assessment (no code changes)
**Purpose**: Full scope of what exists, what is missing, and how far the project is from
a complete circuit-level replica in software and FPGA of the entire MCS-4, MCS-40, and
extended chip ecosystem.

---

## PROJECT SNAPSHOT

| Metric | Value |
|--------|-------|
| Workspace | 8 Rust crates under `mcs4-emu/crates/` |
| Source files | 138 .rs files, 42,551 lines |
| Tests | 968 passing, 0 failures |
| Evidence artifacts | 7.2 GB across 48 evidence subdirectories |
| Scripts | 86 Python + 13 shell = 99 analysis/extraction scripts |
| Documentation | 40+ markdown files, 11 PDFs (including 78 MB MOD 40 schematics) |
| Phases 0-4 | COMPLETE (100%) |
| Phase 5 | 75% (peripherals/Intellec-4/Verilog done; hardware validation deferred) |
| Overall | ~92% of original project roadmap |

---

## A. CHIP IMPLEMENTATIONS -- BEHAVIORAL / CYCLE-ACCURATE

### A1. COMPLETE -- 18 Intel First-Party Chips (100%)

All chips have behavioral Rust implementations with full test coverage.

| Chip | Function | File | Tests |
|------|----------|------|-------|
| **4004** | 4-bit CPU, 46 instructions, ALU, 3-level stack | `mcs4-chips/src/i4004/` | 22 (ALU 3, decoder 9, registers 4, bridge 6) |
| **4040** | Enhanced CPU, 60 instructions, interrupts, 7-level stack | `mcs4-chips/src/i4040/` | 20 (mod 6, interrupt 3, decoder 1, registers 9, stack 1) |
| **4001** | 256x8 ROM + 4-bit I/O port | `mcs4-chips/src/i4001.rs` | 3 |
| **4002** | 320-bit RAM + 4-bit output port (bank_id covers 4002-1/4002-2) | `mcs4-chips/src/i4002.rs` | 5 |
| **4003** | 10-bit shift register | `mcs4-chips/src/i4003.rs` | 15 |
| **4008** | 12-bit address latch + CM-ROM decode | `mcs4-chips/src/i4008.rs` | 10 |
| **4009** | Standard I/O expander | `mcs4-chips/src/i4009.rs` | 8 |
| **3216** | 4-bit bus driver (non-inverting) | `mcs4-chips/src/i3216.rs` | 8 |
| **3226** | 4-bit bus driver (inverting) | `mcs4-chips/src/i3226.rs` | 8 |
| **4101** | 256x4 static RAM | `mcs4-chips/src/i4101.rs` | 17 |
| **4201** | Clock generator (crystal, non-overlap, reset) | `mcs4-chips/src/i4201.rs` | 13 |
| **4207** | Single-phase crystal clock | `mcs4-chips/src/i4207.rs` | 6 |
| **4209** | 1-to-2-phase converter | `mcs4-chips/src/i4209.rs` | 5 |
| **4211** | RC oscillator + 2-phase clock | `mcs4-chips/src/i4211.rs` | 6 |
| **4265** | Programmable I/O (4x4 bits) | `mcs4-chips/src/i4265.rs` | 9 |
| **4289** | Standard memory interface | `mcs4-chips/src/i4289.rs` | 14 |
| **4308** | 1Kx8 ROM + I/O | `mcs4-chips/src/i4308.rs` | 13 + 9 integration |
| **4316** | LCD segment driver | `mcs4-chips/src/i4316.rs` | 7 |
| **4702** | 256x8 UV-erasable PROM | `mcs4-chips/src/i4702.rs` | 8 |

**Total chip tests**: 211 (mcs4-chips unit) + 1 fuzz + 11 proptest = 223

### A2. NOT STARTED -- Missing Intel First-Party Chips (0%)

| Chip | Description | Difficulty | Documentation |
|------|-------------|------------|---------------|
| **4269** | Programmable Keyboard/Display Interface | HIGH | Datasheet in 1975 Intel Data Catalog (6.4 MB PDF in repo) |
| **4238** | Program Memory Interface (ROM/EPROM) | MEDIUM | Limited public docs |
| **4243** | Program Memory Interface for EPROM/RAM | MEDIUM | Very limited public docs |
| **3205** | 1-of-8 binary decoder | LOW | Standard TTL, well-documented |
| **3404** | 6-bit D-type latch / dual NAND buffer | LOW | Standard TTL, well-documented |

### A3. NOT STARTED -- External Memory Chips (0%)

Used with MCS-4/40 systems via 4289 memory interface:

| Chip | Description | Difficulty |
|------|-------------|------------|
| **2101** | 256x4 SRAM | LOW |
| **2102** | 1024x1 SRAM | LOW |
| **1302/1602/1702** | EPROM variants (1702 = original UV-EPROM) | MEDIUM |
| **2316** | 16K mask ROM | LOW |

### A4. NOT APPLICABLE -- Second-Source / Clone Chips

| Manufacturer | Chips | Assessment |
|-------------|-------|------------|
| National Semiconductor | INS4004, INS4001/4002/4003 | Pin/function identical to Intel |
| NEC | uPD4004, uPD4040 | Pin/function identical to Intel |
| Mostek | MK4004 (if produced) | Pin/function identical |

**Verdict**: Second-sources are functionally identical. No separate behavioral
implementation needed. The only value would be documenting timing/electrical differences
at the circuit level, which requires die shots that do not exist publicly.

### A5. LOW PRIORITY -- 74-Series TTL Glue Logic

Standard 7442/74154 decoders, 7407/74125 buffers, 7475/74175 latches, 7490/7493 counters,
7447/7448 display drivers. These are board-level glue, not part of the MCS-4/40 chip
family. Only needed for specific application systems (Busicom, Intellec MOD 40 board-level).

---

## B. CIRCUIT-LEVEL SIMULATION INFRASTRUCTURE

### B1. COMPLETE -- Solver Stack (100%)

All solvers reside in `mcs4-core/src/solver/` and related files. Total: 473 tests in
mcs4-core + 12 error path + 18 integration + 1 nodal_4003 = 504 core tests.
Combined solver infrastructure: 5,886 lines across 11 solver files.

| Solver | File | Lines | Tests | Status |
|--------|------|-------|-------|--------|
| DC Operating Point (MNA + Newton-Raphson) | `solver/dc_op.rs` | 716 | 12 | COMPLETE |
| AC Small-Signal | `solver/ac.rs` | 713 | 9 | COMPLETE |
| Transient (BE / Trapezoidal / TRBDF2) | `solver/transient.rs` | 1,247 | 18 | COMPLETE |
| Convergence aids (source stepping, damping, gmin) | `solver/convergence.rs` | 216 | 13 | COMPLETE |
| Dense Matrix (nalgebra) | `solver/matrix.rs` | 327 | 6 | COMPLETE |
| Sparse Matrix (faer COO->CSC->LU, auto >100 nodes) | `solver/sparse_matrix.rs` | 544 | 13 | COMPLETE |
| Stimulus sources (Pulse, DC, PWL) | `solver/stimulus.rs` | 378 | 11 | COMPLETE |
| Event Scheduler | `solver/event_scheduler.rs` | 205 | 7 | COMPLETE |
| Noise Analysis (thermal, flicker) | `solver/noise.rs` | 421 | 14 | COMPLETE |
| Sensitivity Analysis | `solver/sensitivity.rs` | 679 | 14 | COMPLETE |
| Temperature Sweep | `solver/temp_sweep.rs` | 344 | 5 | COMPLETE |
| Switch-Level Transistor Solver | `transistor_solver.rs` | 757 | 14 | COMPLETE |
| Nodal Analysis Solver (full MNA) | `nodal_solver.rs` | 602 | 3 | COMPLETE |

### B2. COMPLETE -- Process Models: Intel 10um pMOS (100%)

Process models: 2,118 lines / 74 tests in `mcs4-core/src/process/`.
Device models: 2,004 lines / 64 tests in `mcs4-core/src/device/`.

| Model | File | Lines | Tests | Status |
|-------|------|-------|-------|--------|
| ProcessParams + temperature scaling | `process/mod.rs` | 333 | 9 | COMPLETE |
| Silicon substrate (ni, mobility, bandgap) | `process/silicon.rs` | 202 | 10 | COMPLETE |
| Gate oxide (tox=80nm, Cox, breakdown) | `process/oxide.rs` | 105 | 5 | COMPLETE |
| Field-dependent mobility | `process/mobility.rs` | 133 | 6 | COMPLETE |
| PN junction (built-in potential, depletion) | `process/junction.rs` | 121 | 5 | COMPLETE |
| Al interconnect R/C | `process/interconnect.rs` | 168 | 8 | COMPLETE |
| Thermal resistance / self-heating | `process/thermal.rs` | 134 | 5 | COMPLETE |
| I/O driver (Ron, slew rate, output impedance) | `process/io_driver.rs` | 158 | 4 | COMPLETE |
| Power model (static leakage + dynamic CV^2f) | `process/power.rs` | 200 | 6 | COMPLETE |
| ESD protection diode | `process/esd.rs` | 165 | 5 | COMPLETE |
| ROM cell model (wordline RC, bitline charge sharing) | `process/rom_cell.rs` | 173 | 4 | COMPLETE |
| SRAM cell model (6T SNM, read/write timing) | `process/sram_cell.rs` | 226 | 7 | COMPLETE |
| Level 1 pMOS (Shichman-Hodges + subthreshold + body + DIBL + vsat) | `device/pmos_level1.rs` | 818 | 28 | COMPLETE |
| Level 3 pMOS (short-channel effects) | `device/pmos_level3.rs` | 385 | 12 | COMPLETE |
| Meyer capacitance model | `device/cap_model.rs` | 387 | 10 | COMPLETE |
| Depletion-load device model | `device/depletion_load.rs` | 199 | 8 | COMPLETE |
| Layout parasitic extraction | `device/parasitic.rs` | 149 | 6 | COMPLETE |

### B3. COMPLETE -- TCAD Physics (100%)

TCAD: 2,165 lines / 51 tests in `mcs4-core/src/tcad/`.

| Module | File | Lines | Tests | Status |
|--------|------|-------|-------|--------|
| Poisson-Boltzmann electrostatic solver | `tcad/poisson.rs` | 568 | 14 | COMPLETE |
| Drift-diffusion carrier transport | `tcad/drift_diffusion.rs` | 546 | 10 | COMPLETE |
| Fermi-Dirac carrier statistics | `tcad/carrier.rs` | 324 | 11 | COMPLETE |
| 1D non-uniform spatial mesh | `tcad/mesh.rs` | 309 | 9 | COMPLETE |
| Channel discretization + inversion charge | `tcad/channel.rs` | 200 | 5 | COMPLETE |
| TCAD-to-circuit bridge (Pao-Sah integral) | `tcad/bridge.rs` | 192 | 2 | COMPLETE |

### B4. COMPLETE -- Circuit Representation (100%)

Circuit representation: 3,431 lines / 84 tests in `mcs4-core/src/circuit/`.

| Module | File | Lines | Tests | Status |
|--------|------|-------|-------|--------|
| Clock distribution network | `circuit/clock_tree.rs` | 566 | 11 | COMPLETE |
| Static timing analysis / critical path | `circuit/timing_analysis.rs` | 550 | 12 | COMPLETE |
| Metal layer parasitic R/C extraction | `circuit/parasitic_extract.rs` | 482 | 14 | COMPLETE |
| Architecture block mapping | `circuit/arch_map.rs` | 474 | 9 | COMPLETE |
| Die coordinate system / spatial queries | `circuit/spatial.rs` | 456 | 16 | COMPLETE |
| Netlist-to-graph conversion | `circuit/netlist_bridge.rs` | 311 | 6 | COMPLETE |
| CircuitGraph data structure | `circuit/graph.rs` | 293 | 5 | COMPLETE |
| Power rail identification (VDD/VSS) | `circuit/power_rail_id.rs` | 143 | 5 | COMPLETE |
| Bounding box to geometry | `circuit/bbox_to_geometry.rs` | 131 | 6 | COMPLETE |

### B5. COMPLETE -- Multi-Fidelity Bridge (100%)

Fidelity/bridge: 463 lines / 15 tests across 3 files in `mcs4-core/src/`.

| Component | File | Lines | Tests | Status |
|-----------|------|-------|-------|--------|
| SimulationFidelity enum (5 levels) | `fidelity.rs` | 111 | 6 | COMPLETE |
| ChipSolverBridge trait + PinMapping | `bridge.rs` | 195 | 8 | COMPLETE |
| FidelityManager orchestrator | `fidelity_manager.rs` | 157 | 1 | COMPLETE |
| I4004 bridge implementation | `mcs4-chips/src/i4004/solver_bridge.rs` | -- | 6 | COMPLETE |
| I4040 bridge implementation | `mcs4-chips/src/i4040/solver_bridge.rs` | -- | 6 | COMPLETE |

**Proof-of-concept**: I4004 clock buffer (3-inverter chain, 6 transistors) validated
through DC operating point + transient simulation using ProcessParams-based device models.

**Fidelity levels** (ordered, each includes capabilities of lower):
- `Behavioral` -- pure Rust state machine (fastest)
- `PhaseAccurate` -- bus-cycle-accurate timing
- `SwitchLevel` -- transistor switch-level simulation
- `NodalLevel` -- analog nodal analysis with RC networks
- `TCADLevel` -- full device physics via Pao-Sah integral

**Section B totals** (mcs4-core simulation infrastructure):

| Category | Files | Lines | Tests |
|----------|-------|-------|-------|
| Solver stack | 11 | 5,886 | 122 |
| Circuit representation | 10 | 3,431 | 84 |
| Process models | 12 | 2,118 | 74 |
| Device models | 6 | 2,004 | 64 |
| TCAD physics | 7 | 2,165 | 51 |
| Fidelity/bridge | 3 | 463 | 15 |
| Other (gate, signal, wire, timing, etc.) | 16 | ~5,030 | ~94 |
| **Total mcs4-core** | **65** | **21,097** | **504** |

---

## C. TRANSISTOR-LEVEL NETLIST DATA

### C1. COMPLETE -- Core MCS-4 Chips (100%)

Verified from `docs/evidence/netlists_v0/metrics.json` and `netlists_v1/` files:

| Chip | V0 Nodes | V0 Transistors | V1 Kept | V1 Signals | V1 JSON Lines |
|------|----------|----------------|---------|------------|---------------|
| 4001 | 5,744 | 2,000 | 1,999 | 91 | 62,358 |
| 4002 | 3,280 | 640 | 639 | 418 | 29,255 |
| 4003 | 490 | 38 | 37 | 16 | 2,437 |
| 4004 | 3,448 | 1,031 | 1,030 | 19 | 41,826 |
| **Total** | **12,962** | **3,709** | **3,705** | **544** | **135,876** |

**Subcircuit extraction** (from `docs/evidence/subcircuits_v0/`):

| Chip | Files | Nonzero-T Subcircuits | Max Transistors | Notes |
|------|-------|-----------------------|-----------------|-------|
| 4001 | 15 | 11 | 117 | CL, CLK1/2, CM, custom, D0-D3, IO0-3, RESET |
| 4002 | 15 | 6 | 42 | CLK1/2, CM, CS, custom, D0-D3, etc. |
| 4003 | 15 | 5 | 9 | CLOCK, custom, DATA, EN, OUT, Q0-Q3 |
| 4004 | 19 | 11 | 437 | CLK1/2, CMRAM0-3, CMROM, D0-D3, etc. |
| **Total** | **64** | **33** | -- | |

**Supporting data**:
- Gate-level netlists (`gates_v0/`): 4 JSON files, **currently empty** (0 gates extracted)
- Device graphs (`device_graph_v0/`): 4 chips
- Hierarchical clustering (`clusters_v0/`): 4 chips (4004: 19->6->3 hierarchy)
- Anchor mappings (`schematic_layout_anchors_v1.json`): 544 total anchors
  (4001: 91, 4002: 418, 4003: 16, 4004: 19)
- Power rail evidence: medium-high confidence (VDD/VSS corroborated from pad geometry)

### C2. BLOCKED -- Non-Core Chips (0%)

**No die shots or mask data exist publicly for ANY chip beyond 4001/4002/4003/4004.**

| Chip | Die Shot | Mask Data | Circuit-Level Feasibility |
|------|----------|-----------|---------------------------|
| 4040 | NONE | NONE | BLOCKED -- #1 priority missing artifact |
| 4101 | NONE | NONE | BLOCKED |
| 4201 | NONE | NONE | BLOCKED |
| 4289 | NONE | NONE | BLOCKED |
| 4308 | NONE | NONE | BLOCKED |
| All others | NONE | NONE | BLOCKED |

**This is the single largest barrier to a complete circuit-level replica of the MCS-40
system.**

---

## D. CIRCUIT-LEVEL SIMULATION GAPS

### D1. INFRASTRUCTURE EXISTS, Wiring Incomplete (~30%)

The full path from netlist JSON -> CircuitGraph -> solver -> waveforms is proven and
tested for small subcircuits. What is missing is wiring all extracted subcircuits through
that path:

| Gap | Current State | What's Needed |
|-----|---------------|---------------|
| Subcircuit enumeration | Only `clock_buffer` (6 of 1,030 transistors) exposed for 4004 | Wire all 11 nonzero subcircuits from `subcircuits_v0/4004/` into I4004::subcircuit() |
| ChipSolverBridge for 4001/4002/4003 | Not implemented | Implement bridge for each chip, expose subcircuits |
| Full-chip DC operating point | Only 4003 (37T) tested end-to-end | Run 4004 full netlist (1,030T) through DC solver |
| Full-chip transient simulation | Only 6-transistor clock buffer tested | Validate convergence with full 4004/4001 netlists |
| Behavioral-vs-circuit cross-validation | FidelityManager designed but no end-to-end test | Complete machine cycle: behavioral tick -> circuit solve -> compare |
| Parasitic RC from real geometry | Infrastructure exists, not populated from mask data | Extract wire segments from bitmap coordinates |
| Gate-level netlist extraction | `gates_v0` JSON files exist but contain **0 gates** | Run `extract_gates_v0.py` with correct parameters |

### D2. DESIGNED BUT NOT IMPLEMENTED (~15%)

| Feature | Design Status | Code Status |
|---------|---------------|-------------|
| Hierarchical subcircuit composition | `CLUSTERING_STRATEGY_V0.md` (3-level hierarchy) | Clustering data exists (4004: 19->6->3), no composition code |
| Mixed-mode simulation (behavioral + analog) | FidelityManager designed | No multi-chip integration test |
| BSIM4 device model | Mentioned as deferred | No code |
| Via connectivity modeling | In ROADMAP.md Phase 5 | `extract_via_connectivity_v0.py` script exists, no integration |
| rkyv snapshots / time-travel debugging | Deferred to Phase 5+ | rkyv is a workspace dependency, no snapshot code |

---

## E. VERILOG / HDL STATUS

### E1. Behavioral Verilog -- 4 Core Chips (100%)

The `mcs4-fpga` crate (`mcs4-emu/crates/mcs4-fpga/`, 495 lines, 12 tests) generates
synthesizable behavioral Verilog for the 4 core MCS-4 chips:

| Chip | Module | Features |
|------|--------|----------|
| 4004 | `i4004_behavioral` | 8-phase state machine, PC, accumulator, registers, stack |
| 4001 | `i4001_behavioral` | Address latch, chip select, ROM output |
| 4002 | `i4002_behavioral` | Address, status registers, output port |
| 4003 | `i4003_behavioral` | 10-bit shift, enable, cascade |

### E2. Gate-Level Verilog -- Empty Shells (~5%)

`docs/evidence/verilog_v0/` contains 8 files (4 modules + 4 testbenches), 424 lines total:

- Module bodies are **empty** -- no wire declarations or gate instances
- Includes a standard gate library (inv, nand2, nand3, nor2, nor3, and2, or2)
- Root cause: `gates_v0` JSON data contains **0 extracted gates**, so `gate_to_verilog_v0.py`
  has no gate data to populate the shells

### E3. MCS-40 Chips Verilog -- None (0%)

No Verilog exists for any of the 14 MCS-40 chips (4040, 4101, 4201, 4289, 4308, etc.).
The behavioral Rust implementations could be mechanically translated using existing
`mcs4-fpga` infrastructure (`VerilogExporter`, `Module`, `Port`, `ChipSpec`).

---

## F. FPGA DEPLOYMENT STATUS

### F1. Design Complete, No Implementation (~20%)

| Artifact | Design Doc | Implementation |
|----------|-----------|----------------|
| Pin constraints (.pcf for iCE40) | Template in `FPGA_SYNTHESIS_WORKFLOW_V0.md` (443 lines) | No files exist |
| Pin constraints (.xdc for Spartan-7) | Template in same doc | No files exist |
| Synthesis Makefile | Full Makefile in design doc | No Makefile exists |
| Resource estimates | 4001:~200 LUT, 4002:~100, 4003:~50, 4004:~2000 | Not verified |
| Timing constraints | 500ns period / 2 MHz documented | Not verified |

### F2. Hardware Validation -- Not Started (0%)

Requires: FPGA board purchase (~$10-20), Yosys+nextpnr+icestorm toolchain (documented),
bitstream synthesis, LED blink test, logic analyzer capture, differential validation vs
software emulator, multi-chip MCS-4 system on single FPGA.

---

## G. SYSTEM-LEVEL EMULATION

### G1. COMPLETE -- Working Systems (100%)

| System | Crate | Description | Tests |
|--------|-------|-------------|-------|
| MCS-4 | mcs4-system | 4004 + 4001 + 4002 + 4003, bus protocol | ~45 |
| MCS-40 | mcs4-system | 4040 + 4308 + 4101 + support chips | 9 integration |
| Intellec-4 | mcs4-intellec | Front panel, monitor ROM, PROM programmer | 44 + 6 integration |
| SIMD Cluster | mcs4-system | 16-lane parallel 4004 execution, full ISA | 87 |
| Peripherals | mcs4-periph | 7-segment, matrix keyboard, UART | 30 |
| GUI Debugger | mcs4-gui | Disasm, registers, memory, stack, breakpoints, waveform | 78 |

### G2. NOT STARTED -- Extended Systems (0%)

| System | Difficulty | Blockers |
|--------|------------|----------|
| Intellec-4/MOD 40 | MEDIUM | Board schematics exist (78 MB PDF in repo); reuse Intellec-4 crate + 4040 |
| SIM4-01 evaluation board | LOW | Limited docs; minimal MCS-4 system |
| Busicom 141-PF calculator | HIGH | Requires ROM dump + printer/keyboard hardware reverse-engineering |

---

## H. OVERALL COMPLETION MATRIX

| Domain | Scope | Complete | Notes |
|--------|-------|----------|-------|
| Behavioral chips (first-party) | 18/18 | **100%** | All MCS-4 + MCS-40 chips identified in datasheets |
| Missing Intel chips | 0/5 | **0%** | 4269, 4238, 4243, 3205, 3404 |
| External memory chips | 0/4 families | **0%** | 2101, 2102, 1302, 2316 |
| Circuit solver stack | 13 solvers | **100%** | SPICE-class DC/AC/Transient/Switch/Nodal |
| Process models | 16 models | **100%** | Intel 10um pMOS, Level 1+3, parasitics |
| TCAD physics | 6 modules | **100%** | Poisson + Drift-Diffusion + mesh |
| Transistor netlists (core) | 4/4 chips | **100%** | 3,705 transistors, 135K lines JSON |
| Transistor netlists (other) | 0/14 chips | **0%** | BLOCKED on die shots |
| Gate-level netlists | 0/4 chips | **~5%** | JSON shells exist, 0 gates extracted |
| Circuit-level simulation infra | Full stack | **100%** | End-to-end path proven |
| Circuit-level actual chip sim | 6T + 37T tested | **~10%** | Only clock_buffer and 4003 end-to-end |
| Behavioral Verilog | 4/18 chips | **22%** | Core MCS-4 chips only |
| Gate-level Verilog | 0/4 populated | **~5%** | Empty shells + gate library; no gate data |
| FPGA synthesis | Design only | **~20%** | No constraint files, no actual synthesis |
| FPGA hardware | None | **0%** | Requires board purchase |
| Working systems | 5/5 planned | **100%** | MCS-4, MCS-40, Intellec-4, SIMD, peripherals |
| Extended systems | 0/3 | **0%** | MOD 40, SIM4-01, Busicom |
| Documentation | Core + evidence | **85%** | 4040 die shots = critical gap |

---

## I. DISTANCE TO FULL CIRCUIT-LEVEL REPLICA

### For Software Simulation

**MCS-4 system (4004 + 4001 + 4002 + 4003): ~35% complete for circuit-level**

- All transistor netlists exist (3,705 transistors total across 4 chips)
- All solvers exist and are proven end-to-end
- Subcircuit extraction complete (33 nonzero-transistor subcircuits, 64 files total)
- Gate-level extraction NOT done (0 gates in gates_v0 data)
- Gap: wire subcircuits through ChipSolverBridge, run full-chip sims, cross-validate
- Gap: gate extraction is a prerequisite for meaningful gate-level Verilog

**MCS-40 system (4040 + support chips): ~5% complete for circuit-level**

- Behavioral models complete for all chips
- Solvers/infrastructure complete and reusable
- Gap: NO transistor netlists -- completely BLOCKED on acquiring die shots/mask data
- The 4040 die shot is the single highest-priority missing artifact

### For FPGA

**MCS-4 core (4001-4004): ~25% complete**

- Behavioral Verilog exists and is synthesizable (12 tests)
- Gate-level Verilog shells exist but are empty
- Gap: populate gate-level Verilog (requires gate extraction first), create constraint
  files, run synthesis, validate on hardware

**MCS-40 chips: ~5% complete**

- No Verilog at all for any MCS-40 chip
- Behavioral Verilog generation is mechanical using existing mcs4-fpga infrastructure
- Gate-level Verilog impossible without die shots

**Full MCS-4 system on FPGA: ~15% complete**

- Individual chip behavioral Verilog exists
- No system-level Verilog (top-level interconnect, bus arbitration)
- No constraint files, no synthesis verification, no hardware

---

## J. CRITICAL PATH PRIORITIES (if work were to proceed)

### Tier 1 -- Complete Circuit-Level for MCS-4 Core (highest ROI)

1. Run `extract_gates_v0.py` to populate gate-level netlists from subcircuit data
2. Wire all 11 nonzero-transistor 4004 subcircuits into I4004::subcircuit()
3. Implement ChipSolverBridge for I4001, I4002, I4003
4. Full-chip 4003 transient simulation (37T -- smallest, validation target)
5. Full-chip 4004 DC operating point (1,030T -- sparse solver stress test)
6. Behavioral-vs-circuit cross-validation for one complete machine cycle
7. Populate gate-level Verilog by running gate_to_verilog_v0.py against populated gates_v0

### Tier 2 -- Extend Behavioral Coverage

8. 3205 decoder (simple, enables address decode)
9. 2101 SRAM (simple, used with 4289)
10. 4269 keyboard/display interface (complex, needed for real applications)
11. Behavioral Verilog for remaining 14 chips

### Tier 3 -- FPGA Deployment

12. Create .pcf/.xdc constraint files from templates in FPGA_SYNTHESIS_WORKFLOW_V0.md
13. Run Yosys synthesis on behavioral Verilog (4001 first -- smallest)
14. Procure iCE40 board, program, validate
15. Multi-chip MCS-4 system on FPGA

### Tier 4 -- External Dependencies (cannot be solved by coding)

16. Locate 4040 die shot (unlocks ALL circuit-level MCS-40 work)
17. Locate MCS-40 support chip die shots (4101/4201/4289/4308)
18. Full-size photomicrographs for 4001-4004 (higher resolution)

---

## K. LANGUAGES AND ARTIFACTS SUMMARY

| Language/Format | Count | Purpose |
|-----------------|-------|---------|
| **Rust** | 138 .rs files, 42,551 lines | All behavioral models, solvers, GUI, system integration |
| **Python** | 86 scripts | Netlist extraction, OCR, gate extraction, Verilog conversion, metrics |
| **Shell** | 13 scripts | CI pipeline, doc validation, benchmarks, cleanup |
| **Verilog** | 8 .v files (424 lines, empty bodies) | Gate-level shells for 4001/4002/4003/4004 |
| **Verilog (generated)** | via mcs4-fpga crate | Behavioral synthesizable HDL for 4 chips |
| **JSON** | ~463K lines (v0+v1 netlists) | Transistor netlists, gate netlists, device graphs, subcircuits |
| **PDF** | 11 documents (incl. 78 MB MOD 40 schematics) | Primary Intel documentation, datasheets, manuals |
| **Markdown** | 40+ documents | Architecture, status, roadmap, evidence, phase reports |
| **YAML** | Manifests + configs | OCR pipeline manifests, CI configuration |
| **BibTeX** | 1 file (24 KB) | Academic bibliography |

**Not used**: SystemVerilog, VHDL, Clash/Chisel/Amaranth, SPICE netlists, PCB/KiCad.

---

## L. WORKSPACE CRATE DETAIL

| Crate | Files | Lines | Tests | Purpose |
|-------|-------|-------|-------|---------|
| mcs4-core | 65 | 21,097 | 504 | Simulation kernel: solvers, process, TCAD, circuit, fidelity |
| mcs4-chips | 35 | 9,364 | 223 | All chip implementations (4004/4040, 16 support chips) |
| mcs4-system | 8 | 4,524 | 54 | System integration, SIMD cluster, fixtures |
| mcs4-gui | 13 | 2,841 | 78 | egui-based debugger panels |
| mcs4-intellec | 6 | 1,748 | 50 | Intellec-4 development system |
| mcs4-bus | 5 | 1,281 | 17 | Bus protocol abstraction (4-bit data, control, clock) |
| mcs4-periph | 4 | 1,201 | 30 | Peripheral devices (7-segment, keyboard, UART) |
| mcs4-fpga | 2 | 495 | 12 | Verilog export and chip module generation |
| **Total** | **138** | **42,551** | **968** | |

---

## M. KEY DEPENDENCIES

| Category | Crates |
|----------|--------|
| Math/Solver | nalgebra, faer, rayon |
| Bitfields | modular-bitfield, bitflags, bitvec, smallvec |
| Serialization | rkyv, serde, serde_json, memmap2, bytemuck, zerocopy |
| GUI | egui, eframe |
| Testing | proptest, criterion, tempfile |
| Tracing | tracing, tracing-subscriber |
| Build | seq-macro, paste, static_assertions |

**Rust edition**: 2021, **MSRV**: 1.92, **Toolchain**: nightly-2026-01-07

---

## N. EVIDENCE PIPELINE SUMMARY

The project maintains a rigorous extraction pipeline from silicon to simulation:

```
Die shots / mask layers (bitmap)
    |
    v
OCR pipeline (86 Python scripts, CUDA/ONNX/pytesseract)
    |
    v
Transistor extraction (poly x diffusion intersections)
    |
    v
Netlist V0 (node bboxes, transistor candidates, via stitching)
    |    12,962 nodes, 3,709 transistors, 327K JSON lines
    v
Anchor mapping (schematic <-> layout alignment, 544 anchors)
    |
    v
Netlist V1 (filtered transistors, signal labeling)
    |    3,705 kept transistors, 135K JSON lines
    v
Subcircuit extraction (bounded neighborhoods around anchor nodes)
    |    64 files, 33 nonzero-transistor subcircuits
    v
Device graph + hierarchical clustering
    |    4004: 19 -> 6 -> 3 hierarchy
    v
Gate extraction (INCOMPLETE: 0 gates in gates_v0)
    |
    v
Circuit simulation (proven for 6T clock buffer, 37T 4003)
    |
    v
Behavioral cross-validation (designed, not yet end-to-end)
```

---

## O. RISK REGISTER

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| 4040 die shots never surface | Blocks ALL MCS-40 circuit-level work | HIGH | Monitor collector communities, museums; behavioral models unaffected |
| Sparse solver convergence at 1000+ transistors | Blocks full-chip simulation | MEDIUM | Tested at 37T; subcircuit decomposition reduces scale per solve |
| Gate extraction fails to produce meaningful netlists | Blocks gate-level Verilog | MEDIUM | Fallback to behavioral Verilog (already working) |
| iCE40 resource limits for 4004 (~2000 LUT) | Requires HX8K upgrade | LOW | 4001/4002/4003 fit HX4K; behavioral 4004 may fit with optimization |
| OCR accuracy insufficient for remaining pad labels | Delays anchor propagation | LOW | Manual readings infrastructure already exists as fallback |

---

*Generated 2026-03-05. This is a point-in-time assessment; verify against `mcs4-emu/CLAUDE.md`
(canonical status) and `cargo test --workspace` for current test counts.*
