# MCS-4/MCS-40 Digital Twin: Crystal Lattice to System Level

## Context

This document rescopes the entire 4-bit project -- what has been built, the
path taken to get here, and the forward roadmap toward materials-science-depth
simulation where the physical silicon grid, circuit topology, and CPU
architecture are aligned into a single coherent digital twin of the Intel 4004
die.

The project simulates the Intel MCS-4/MCS-40 chip family (4001/4002/4003/4004,
4040/4101/4201/4289) at multiple fidelity levels, from instruction-level
behavioral emulation down through SPICE-class analog circuit simulation. The
goal is to descend further -- through semiconductor device physics, carrier
transport, and ultimately crystallographic properties -- while keeping every
layer connected to the layers above and below it.

---

## Part 1: What Has Been Built (Inventory)

### Evidence and Research (Layer 0)
- 9 OCR'd primary source documents (940 KB total, SHA-256 verified)
- BibTeX bibliography (20 entries), source manifest, citation guide
- Process parameters document (v1) with OCR cross-references
- 90+ Python scripts: OCR, netlist extraction, layout analysis, clustering
- Chip photomicrographs with coordinate transforms and anchor incidence data
- Extracted netlists: 4003 (37 transistors), 4002 (639), 4004 (1030), 4001 (1999)

### Process Parameters (Layer 1) -- `mcs4-core/src/process/`
- `ProcessParams` struct: VDD, VSS, tox, Vth, mu0, theta, Nsub, Nsd, Lmin, Xj, lambda, temp
- `silicon.rs`: Physical constants (q, kB, eps_si, eps_ox, ni, Eg), thermal voltage, Fermi potential
- `oxide.rs`: Cox, gate capacitance, breakdown field
- `mobility.rs`: Lombardi effective mobility, temperature dependence, beta
- `junction.rs`: Abrupt-junction Cj(Vr), Cj0 from doping, sidewall capacitance
- `thermal.rs`: Vth(T), mu_eff(T), ni(T)
- `interconnect.rs`: Sheet resistance (poly/diffusion/metal), wire R/C, Elmore delay

### Device Models (Layer 2) -- `mcs4-core/src/device/`
- `pmos_level1.rs`: Shichman-Hodges Level 1 (cutoff/triode/saturation, gm, gds, CLM)
- `depletion_load.rs`: Gate-tied-to-source always-on load
- `parasitic.rs`: Cgs, Cgd (overlap), Cdb, Csb (junction), Cgb (channel)

### Circuit Representation (Layer 3) -- `mcs4-core/src/circuit/`
- `graph.rs`: CircuitGraph with AnalogNode + AnalogTransistor
- `netlist_bridge.rs`: NetlistV1 JSON to circuit (all 4 chip netlists)
- `power_rail_id.rs`: Automatic VDD/VSS identification
- `bbox_to_geometry.rs`: Pixel bounding boxes to physical W/L (meters)

### SPICE-Class Solver (Layer 4) -- `mcs4-core/src/solver/`
- `dc_op.rs`: Newton-Raphson DC operating point (converges on all 4 chips)
- `matrix.rs`: Dense MNA (nalgebra), `sparse_matrix.rs`: Sparse MNA (faer)
- `convergence.rs`: Voltage limiting, gmin stepping, source stepping, damping
- `transient.rs`: Backward Euler companion models (framework only, not integrated)
- `event_scheduler.rs`: Hybrid event-driven scheduler

### Switch-Level / Nodal (Layer 5)
- `transistor_solver.rs`: Iterative combinational (14 tests)
- `nodal_solver.rs`: Simplified Gaussian elimination (14 tests)

### Gate-Level (Layer 6)
- `gate.rs`, `signal.rs`, `simulator.rs`, `wire.rs`, `timing.rs`, `trace_buffer.rs`

### Instruction-Level (Layer 7) -- `mcs4-chips/`
- 4004 CPU: 46 instructions, ALU, 3-level stack, 16 registers
- 4040 CPU: 60 instructions, interrupt controller, 7-level stack, bank switching
- 8-phase machine cycle (A1-A3, M1-M2, X1-X3)

### Chip Implementations (Layer 8) -- `mcs4-chips/`
- 4001 ROM, 4002 output port, 4003 shift register
- 4101 RAM, 4201 clock, 4289 memory interface, 4308 test ROM
- Disassembler with symbol tables and auto-labeling

### System Integration (Layer 9) -- `mcs4-system/`
- MCS-4 system (4004 + peripherals), MCS-40 system (4040 + peripherals)
- 16-lane SIMD parallel execution cluster with performance metrics
- Test fixture runner

### Bus Protocol (Layer 9.5) -- `mcs4-bus/`
- Two-phase clock, 4-bit bidirectional data bus, control signals
- 8-phase bus cycle state machine

### Visualization & Output (Layer 10) -- `mcs4-gui/`, `mcs4-fpga/`
- GUI: waveform viewer, disassembly panel (register/memory panels planned)
- Verilog exporter for FPGA synthesis

### Infrastructure
- 357 tests passing, 0 failures, clippy clean
- Nightly Rust 1.92.0, faer 0.24 (sparse), nalgebra 0.34 (dense), rayon (parallel)
- CI: format, clippy, test, coverage, mdBook build, rustdoc deploy
- mdBook documentation with chip guides, evidence methodology, dev guides

---

## Part 2: Simulation Hierarchy Map

```
Layer 10: System Integration                          [COMPLETE]
  MCS-4 system, MCS-40 system, SIMD cluster
  |
Layer 9: Instruction-Level Simulation                 [COMPLETE]
  4004 (46 insns), 4040 (60 insns), 8-phase cycle
  |
Layer 8: Chip Implementations                         [90%]
  All 9 chips implemented (GUI panels pending)
  |
Layer 7: Gate-Level Simulation                        [COMPLETE]
  Event-driven engine, logic primitives, trace buffer
  |
Layer 6: Switch-Level Simulation                      [COMPLETE]
  Transistor solver (14 tests), Nodal solver (14 tests)
  |
Layer 5: SPICE-Class Circuit Solver                   [75%]
  DC operating point: DONE (all 4 chips)
  Transient: FRAMEWORK ONLY (not integrated)
  AC analysis: NOT STARTED
  Noise analysis: NOT STARTED
  |
Layer 4: Circuit Graph / Netlist                      [COMPLETE]
  NetlistV1 bridge, power rails, 4 chip netlists
  |
Layer 3: Device Models                                [35%]
  Level 1 (Shichman-Hodges): DONE
  Body effect: NOT STARTED
  Subthreshold / DIBL / Vsat: NOT STARTED
  |
Layer 2: Process Parameters                           [65%]
  Constants, Cox, mobility, junction, thermal: DONE
  Interface states, oxide charges, body params: NOT STARTED
  |
Layer 1: Materials / Band Structure                   [5%]
  Eg, ni stored as constants
  Effective masses, density of states, transport: NOT STARTED
  |
Layer 0: Crystallography / Physical Grid              [0%]
  <111> orientation noted as metadata only
  Spatial die mapping, defect models: NOT STARTED
```

---

## Part 3: The Physics Gap

Current model (Level 1 Shichman-Hodges) captures:
- Square-law Ids in 3 regions, first-order CLM, Lombardi mobility, abrupt junction Cj

What is missing for materials-science fidelity:

**Category A -- Incremental device corrections** (extend existing PmosLevel1):
1. Body effect: Vth(Vsb) = Vth0 + gamma * (sqrt(2*phi_f + Vsb) - sqrt(2*phi_f))
2. Subthreshold conduction: Ids_sub = Ids0 * exp((Vgs - Vth) / (n * Vt))
3. DIBL: Vth_eff = Vth(Vsb) - eta * Vds
4. Velocity saturation: v_sat ~ 8e6 cm/s for holes, limits Ids_sat
5. Region-dependent Cgs/Cgd/Cgb (Meyer model with Ward-Dutton partition)

**Category B -- New solver capabilities** (extend solver/ module):
1. Transient integration (wire backward Euler + capacitor companions into NR loop)
2. AC small-signal: (G + jw*C) * V = I, frequency sweep
3. Noise: thermal (4kT*2/3*gm), flicker (Kf/f*Cox*W*L), propagation via H(f)
4. Parameter sensitivity: dV/dp via adjoint method

**Category C -- TCAD-class physics** (new tcad/ module):
1. 1D Poisson-Boltzmann: electrostatic potential through gate/oxide/depletion/bulk
2. 1D drift-diffusion: Scharfetter-Gummel from source to drain
3. 2D MOS cross-section: finite volume for junction profiles, parasitic coupling
4. Quantum corrections: Schrodinger-Poisson for inversion layer quantization

**Category D -- Reliability and variation**:
1. Monte Carlo process variation (sigma_vth, sigma_tox, sigma_mu)
2. Hot carrier injection (lucky electron model)
3. NBTI (reaction-diffusion), TDDB (percolation)
4. Electromigration (Black's equation on layout-extracted wires)

---

## Part 4: Phased Workstreams

### Stream Alpha: Device Model Enhancement

Upgrade from Level 1 to Level 3-equivalent. Same `DeviceModel` trait; all 357
existing tests continue to pass.

**Alpha-1: Body Effect and Subthreshold**
- Add `gamma`, `phi_b` to ProcessParams (derived from existing n_sub, eps_si, cox)
- Extend PmosLevel1 with Vsb-dependent Vth, subthreshold exponential regime
- Extend AnalogTransistor to track bulk node (currently implicit)
- Files: `device/pmos_level1.rs`, `process/mod.rs`, `circuit/graph.rs`
- Tests: Vth shift vs Vsb; subthreshold slope on semilog plot; regression green

**Alpha-2: DIBL and Velocity Saturation**
- Add eta_dibl to ProcessParams (small for 10um but structurally correct)
- Vth(Vds) correction, velocity saturation via field-dependent Vdsat
- Add v_sat_holes to `process/silicon.rs`
- Files: `device/pmos_level1.rs`, `process/silicon.rs`
- Tests: DIBL magnitude; Ids with v_sat; comparison with/without

**Alpha-3: Improved Capacitance Model**
- Region-dependent Cgs/Cgd/Cgb (Meyer model)
- Ward-Dutton charge partition for transient accuracy
- Files: `device/parasitic.rs`, new `device/cap_model.rs`
- Tests: capacitance transitions at region boundaries; charge conservation

**Alpha-4: Unified Level 3 Model**
- New `PmosLevel3` struct packaging Alpha-1 through Alpha-3
- Keep PmosLevel1 for backward compatibility and fast simulation
- Model selection in SolverConfig
- Files: new `device/pmos_level3.rs`, `solver/dc_op.rs`
- Tests: Level 3 = Level 1 at body=0, no-DIBL, above-threshold; all 4 chips converge

### Stream Beta: Solver Completion

**Beta-1: Transient Solver Integration** (highest priority)
- Wire TransientSolver into DcSolver: DC op as initial condition, then
  stamp capacitor companions into MNA at each time step
- Adaptive dt via local truncation error
- Clock stimulus: phi1/phi2 waveforms
- Record to TraceBuffer for GUI waveform display
- Files: `solver/transient.rs`, `solver/dc_op.rs`, new `solver/stimulus.rs`
- Tests: RC charging (analytical); inverter tpd; ring oscillator frequency
- Validation: one 4003 clock cycle, compare switching with gate-level

**Beta-2: AC Small-Signal Analysis**
- Complex MNA: (G + jw*C) * V = I
- Logarithmic frequency sweep 1 Hz to 100 MHz
- Bode plot data output (magnitude, phase)
- Files: new `solver/ac.rs`
- Tests: single-pole RC rolloff; inverter gain-bandwidth

**Beta-3: Noise Analysis**
- Per-transistor noise: thermal, flicker
- Propagate via AC transfer function
- Files: new `solver/noise.rs`
- Tests: thermal noise floor; input-referred noise vs W/L

**Beta-4: Parameter Sensitivity**
- Adjoint method: dV/dp for all nodes vs selected process params
- Files: new `solver/sensitivity.rs`
- Tests: inverter output sensitivity to Vth near switching point

### Stream Gamma: TCAD Physics Engine

New `mcs4-core/src/tcad/` module. Solves semiconductor equations on spatial
meshes rather than using compact models.

**Gamma-1: 1D Poisson-Boltzmann Solver**
- Mesh from gate through oxide through depletion into bulk
- Self-consistent solution with Boltzmann carrier statistics
- Compute: inversion charge, depletion width, Vth from first principles
- Compare Vth against ProcessParams textbook formula
- Files: new `tcad/mod.rs`, `tcad/mesh.rs`, `tcad/poisson.rs`, `tcad/carrier.rs`
- Tests: depletion width vs Vg; Cg-Vg curve; Vth onset

**Gamma-2: 1D Drift-Diffusion Channel Solver**
- Coupled Poisson + hole continuity, source to drain
- Scharfetter-Gummel exponential fitting
- Compute Ids-Vds from first principles; compare against Level 1/3
- Velocity saturation and CLM emerge naturally from field solution
- Files: new `tcad/drift_diffusion.rs`, `tcad/channel.rs`
- Tests: Ids-Vds at multiple Vgs; agreement with compact model within 10%

**Gamma-3: 2D MOS Cross-Section** (stretch)
- 2D finite-volume mesh: source/channel/drain cross-section
- Poisson in 2D for junction profiles, depletion regions, parasitic coupling
- Files: new `tcad/mesh2d.rs`, `tcad/poisson2d.rs`, `tcad/fvm.rs`
- Tests: depletion region shape; 2D potential under bias

**Gamma-4: Quantum Corrections** (stretch)
- 1D Schrodinger perpendicular to interface (triangular well)
- Subband energies, carrier centroid shift
- Self-consistent Schrodinger-Poisson loop
- Files: new `tcad/schrodinger.rs`
- Tests: centroid shift vs Vg; quantum capacitance correction

### Stream Delta: Grid Alignment (Layout to Architecture)

Connects the physical silicon die geometry to the circuit topology and CPU
architecture. This is the "digital twin" integration layer.

**Delta-1: Spatial Netlist with Coordinates**
- Extend AnalogTransistor with `x_center, y_center` (um, die-referenced)
- Extend AnalogNode with wire geometry (segments, layers, widths)
- Import from existing bounding box data + coordinate transforms
- Calibrate pixel-to-um (die: ~3.2 x 3.8 mm for 4004)
- Files: `circuit/graph.rs`, new `circuit/spatial.rs`, `circuit/wire_geometry.rs`
- Tests: all transistors within die bounds; wire lengths positive

**Delta-2: Layout-Aware Parasitic Extraction**
- Compute wire R/C from physical geometry (layer, width, length)
- Coupling capacitance between adjacent wires
- RC tree per signal net (Elmore delay from layout)
- Replace default 50 fF with computed values
- Files: new `circuit/parasitic_extract.rs`
- Tests: clock cap matches datasheet 14-20 pF; data bus 7-10 pF

**Delta-3: Functional Block Mapping**
- Annotate transistors with architectural role: ALU, RegisterFile, Decoder,
  Stack, ClockTree, IoBuffer
- Map from 4004.com reverse-engineering annotations
- Per-block statistics: count, area, power, critical path
- Files: new `circuit/arch_map.rs`
- Tests: block transistor counts sum to total; blocks match known 4004 arch

**Delta-4: Clock Distribution Analysis**
- Trace phi1/phi2 from pads through die
- Clock skew from layout-aware RC (Elmore at each endpoint)
- Compare against setup/hold timing specs
- Files: new `circuit/clock_tree.rs`
- Tests: max skew within spec; all sequentials receive both clocks

**Delta-5: Critical Path Extraction**
- Static timing on transistor netlist: longest RC-delay path
- Per-instruction critical path identification
- Compare computed delay with 1.35-2.0 us clock period
- Files: new `circuit/timing_analysis.rs`
- Tests: critical path < clock period; matches known bottlenecks

### Stream Epsilon: Reliability and Variation

**Epsilon-1: Process Variation Monte Carlo**
- Statistical params: sigma_vth, sigma_tox, sigma_mu, correlations
- Generate N parameter sets, DC solve each, compute yield
- Files: new `process/variation.rs`, `solver/monte_carlo.rs`
- Tests: mean = nominal; sigma scales as 1/sqrt(W*L)

**Epsilon-2: Hot Carrier Injection** (requires Gamma-2)
- Lateral field from drift-diffusion, lucky electron model
- Vth shift rate and time-to-failure
- Files: new `tcad/hot_carrier.rs`

**Epsilon-3: NBTI and TDDB**
- Reaction-diffusion Vth shift (depletion loads always stressed)
- Percolation oxide breakdown probability
- Files: new `tcad/nbti.rs`, `tcad/tddb.rs`

**Epsilon-4: Electromigration** (requires Delta-2)
- Current density from DC op + layout wire dimensions
- Black's equation, flag exceeding segments
- Files: new `circuit/electromigration.rs`

---

## Part 5: The Grid Alignment Concept

Three grids, one die:

### Grid 1: Crystal Lattice (Materials)
- Diamond cubic, a = 5.431 angstrom, <111> surface orientation
- Affects: surface mobility (lower on (111) for holes, but preferred for early
  pMOS), interface state density, oxidation rate, etch anisotropy
- TCAD modules (Gamma) solve on meshes within this crystal structure

### Grid 2: Layout Grid (Physical Die)
- Design rules: 10um minimum feature, ~2um alignment
- Every transistor and wire has (x, y) coordinates on the die
- Layout grid is where process physics meets circuit topology:
  a transistor at (x, y) has its own local tox, Vth, and parasitics

### Grid 3: Architecture Grid (CPU Structure)
- Register file rows/columns, ALU bit slices, decoder ROM columns
- Signal flow (data bus vertical, address horizontal through decoder)
- Timing constraints (carry chain across 4 bits within one phase)
- Power distribution (VDD/VSS buses to highest-activity regions)

### How the Grids Align
1. **Crystal -> Layout**: Oxide thickness variation across die (from
   orientation-dependent oxidation rate) creates systematic Vth variation
2. **Layout -> Architecture**: Physical ALU placement determines carry chain
   delay; register-to-ALU distance determines data transfer time; clock
   tree shape determines inter-block skew
3. **Architecture -> Crystal**: CPU power dissipation pattern creates
   time-varying thermal map; local hotspots shift device parameters
4. **Full loop**: Monte Carlo tox variation at (x,y) shifts Vth for local
   transistors, changes gate propagation delay, affects architectural block
   timing, determines whether CPU operates at 740 kHz or must derate

The `CircuitGraph` is the hub connecting all three grids:
- AnalogNode gets spatial coordinates (Delta-1) and wire geometry (Delta-2)
- AnalogTransistor gets block annotations (Delta-3) and per-instance
  ProcessParams overrides (Epsilon-1 Monte Carlo)
- TCAD solver (Gamma) generates/validates compact model parameters per device
- Transient solver (Beta-1) uses layout-extracted RC (Delta-2)
- Timing analysis (Delta-5) connects physical delay to architecture constraints

---

## Part 6: Execution Sequencing

```
Phase I (Foundation):
  Alpha-1 (Body effect / subthreshold)
  Alpha-2 (DIBL / velocity saturation)
  Beta-1  (Transient solver integration)
  Delta-1 (Spatial netlist coordinates)

Phase II (Analysis):
  Alpha-3 (Region-dependent capacitance)
  Alpha-4 (Unified Level 3 model)
  Beta-2  (AC small-signal)
  Delta-2 (Layout-aware parasitic extraction)
  Delta-3 (Functional block mapping)

Phase III (TCAD Core):
  Gamma-1 (1D Poisson-Boltzmann)
  Gamma-2 (1D Drift-Diffusion)
  Beta-3  (Noise analysis)
  Beta-4  (Parameter sensitivity)
  Delta-4 (Clock distribution)
  Delta-5 (Critical path extraction)

Phase IV (Advanced):
  Gamma-3 (2D MOS cross-section)
  Gamma-4 (Quantum corrections)
  Epsilon-1 (Process variation Monte Carlo)
  Epsilon-2 (Hot carrier injection)
  Epsilon-3 (NBTI / TDDB)
  Epsilon-4 (Electromigration)
```

Dependencies:
- Beta-1 depends on Alpha-3 (capacitance model for transient)
- Gamma-2 depends on Gamma-1 (Poisson needed for drift-diffusion)
- Epsilon-2 depends on Gamma-2 (lateral field from DD solution)
- Epsilon-4 depends on Delta-2 (wire dimensions from layout extraction)
- Delta-5 depends on Delta-2 + Delta-4 (RC extraction + clock tree)

---

## Part 7: What Stays Intact

All 357 existing tests remain as regression anchors. Every new phase adds tests
that verify new capabilities without breaking old ones. Specifically:

- `DeviceModel` trait is extended with optional methods, not replaced
- `CircuitGraph` structs gain new fields with defaults, not restructured
- DC solver continues working with Level 1 models (Level 3 is opt-in)
- New analysis modes (transient, AC, noise, TCAD) are additive modules
- Existing PmosLevel1 remains available for fast simulation modes

---

## Part 8: Critical Files

### Files to Extend
| File | Stream | Change |
|------|--------|--------|
| `device/pmos_level1.rs` | Alpha | Body effect, subthreshold, DIBL, vsat |
| `process/mod.rs` | Alpha | gamma, phi_b, eta_dibl, v_sat, sigma params |
| `process/silicon.rs` | Alpha | v_sat_holes, effective masses |
| `circuit/graph.rs` | Delta | Spatial coords, bulk node, block annotation |
| `solver/transient.rs` | Beta | Wire into DC solver, capacitor companions |
| `solver/dc_op.rs` | Alpha+Beta | Level 3 dispatch, transient loop |
| `device/parasitic.rs` | Alpha | Region-dependent Meyer model |

### Files to Create
| File | Stream | Purpose |
|------|--------|---------|
| `device/pmos_level3.rs` | Alpha-4 | Unified Level 3 compact model |
| `device/cap_model.rs` | Alpha-3 | Meyer capacitance with charge partition |
| `solver/ac.rs` | Beta-2 | Complex MNA frequency sweep |
| `solver/noise.rs` | Beta-3 | Per-device noise, spectral density |
| `solver/sensitivity.rs` | Beta-4 | Adjoint parameter sensitivity |
| `solver/stimulus.rs` | Beta-1 | Clock and input waveform generation |
| `solver/monte_carlo.rs` | Epsilon-1 | Statistical variation engine |
| `tcad/mod.rs` | Gamma | TCAD module root |
| `tcad/mesh.rs` | Gamma-1 | 1D spatial mesh |
| `tcad/poisson.rs` | Gamma-1 | Poisson-Boltzmann solver |
| `tcad/carrier.rs` | Gamma-1 | Carrier statistics |
| `tcad/drift_diffusion.rs` | Gamma-2 | Channel DD solver |
| `tcad/channel.rs` | Gamma-2 | Source-drain channel mesh |
| `tcad/mesh2d.rs` | Gamma-3 | 2D finite-volume mesh |
| `tcad/fvm.rs` | Gamma-3 | 2D Poisson FVM |
| `tcad/schrodinger.rs` | Gamma-4 | 1D quantum corrections |
| `tcad/hot_carrier.rs` | Epsilon-2 | Lucky electron model |
| `tcad/nbti.rs` | Epsilon-3 | Reaction-diffusion Vth shift |
| `tcad/tddb.rs` | Epsilon-3 | Oxide breakdown probability |
| `circuit/spatial.rs` | Delta-1 | Coordinate system and transforms |
| `circuit/wire_geometry.rs` | Delta-1 | Wire segment representation |
| `circuit/parasitic_extract.rs` | Delta-2 | Layout-aware RC extraction |
| `circuit/arch_map.rs` | Delta-3 | Transistor-to-block mapping |
| `circuit/clock_tree.rs` | Delta-4 | Clock distribution analysis |
| `circuit/timing_analysis.rs` | Delta-5 | Static timing / critical path |
| `circuit/electromigration.rs` | Epsilon-4 | Current density vs EM limits |
| `process/variation.rs` | Epsilon-1 | Statistical process parameters |

---

## Verification

Each phase verifies:
1. `cargo test --workspace` -- all existing + new tests pass
2. `cargo clippy --all-targets -- -D warnings` -- clean
3. Phase-specific validation (documented per-phase above)

End-to-end validation target: run the full simulation stack on the 4004 --
from TCAD-computed device parameters, through layout-extracted parasitics,
through the DC operating point, through a transient clock cycle, producing
gate-level switching waveforms that match the instruction-level behavioral
model within timing specifications.
