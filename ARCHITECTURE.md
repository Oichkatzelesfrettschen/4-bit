# MCS-4/MCS-40 Emulator Architecture

## Project Goals

Build a multi-fidelity emulator for the Intel MCS-4 (4004) and MCS-40 (4040)
microcomputer systems:
- **Cycle-accurate behavioral simulation** of all MCS-4/MCS-40 chips
- **Transistor-level solver stack** (switch-level, nodal, TCAD) for physics validation
- **Bus-protocol-accurate integration** across CPU, ROM, RAM, and peripherals
- **GUI debugger** with disassembly, signal trace, and waveform views
- **FPGA synthesis path** via Verilog export

## Accuracy Levels

```
Level 3: Transistor-level (mcs4-core)
  Switch-level solver: VoltageLevel::High/Low/Z per node
  Nodal analysis solver: analog-resolution RC networks
  TCAD physics: process models, Level 1/3 MOSFET, body effect, velocity saturation
  ~4-10 Hz effective clock

Level 2: Gate-level (mcs4-core)
  NAND/NOR/INV/AND/OR primitives with propagation delays
  Wire delays from fanout estimation
  Event-driven simulation engine
  ~1-100 kHz effective clock

Level 1: Cycle-accurate (mcs4-chips, mcs4-system) [PRIMARY]
  Phase-accurate (phi1/phi2) state machine
  8-phase bus protocol: A1/A2/A3/M1/M2/X1/X2/X3
  Instruction-correct behavior, real-time or faster
```

## System Architecture

Per-chip implementation status lives in the single chip-status table
set in `mcs4-emu/STATUS.md` (Chip Implementation Status); the diagrams
below show bus topology only.

### MCS-4 Family (4004-based)
```
+--------+     4-bit data bus      +--------+
|  4004  |<----------------------->|  4001  | ROM (256x8) + I/O
|  CPU   |<----------------------->|  4002  | RAM (320-bit) + output
|        |<----------------------->|  4003  | Shift register (10-bit)
+--------+                         +--------+
    |
    +-- 4008 Address latch + CM-ROM decode
    +-- 4009 I/O expander (MCS-4 to standard bus)
    +-- 3216/3226 Bus drivers (non-inverting/inverting)
    +-- SYNC, CM-ROM, CM-RAM control signals
    +-- phi1, phi2 two-phase clock
```

### MCS-40 Family (4040-based)
```
+--------+     4-bit data bus      +--------+
|  4040  |<----------------------->|  4001  | ROM (256x8) + I/O
|  CPU   |<----------------------->|  4002  | RAM (320-bit) + output
|        |<----------------------->|  4308  | ROM (1Kx8)
|        |<----------------------->|  4101  | RAM (256x4)
+--------+                         +--------+
    |
    +-- 4289 Standard Memory Interface
    +-- 4201/4207/4209/4211 Clock generators
    +-- 4265 Programmable I/O (4x4 bits)
    +-- 4316 LCD segment driver
    +-- 4702 UV-erasable PROM (256x8)
    +-- Interrupt/halt signals
```

## Trace, replay, and GUI ownership

`mcs4-system` defines a versioned `TraceFrame` contract. A frame records a
post-phase observation, run and sequence identity, input-event identity,
provenance, and canonical signals. Behavioral captures carry a SHA-256 of the
complete ordered external-input transcript, including ROM load, reset, program
counter, TEST, and ROM-port inputs. The provenance also names the transcript
representation. A `ReplaySession` records every input and phase command, then
reconstructs a fresh behavioral system from a checkpoint transcript. It does
not claim an analog-state snapshot.

The GUI has one mutation owner: its simulation worker owns the
`ReplaySession`. UI panels consume immutable frames through a channel. A GUI
startup trace import streams and validates JSONL frames within byte, line, and
frame bounds, then atomically swaps the imported trace and disables live Run
and Step commands until Reset returns control to the behavioral worker. The
waveform excludes its label column from data hit testing and retains frame IDs,
reset boundaries, and eviction counts rather than pretending that dropped
history is continuous. The provenance panel displays backend,
fidelity, evidence status, model identity, and stimulus identity. The die
panel renders no transistor overlay until a coordinate-bearing physical netlist
supplies evidence-backed device state.

Cross-backend comparison requires equal declared stimulus representation and
digest plus at least one mapped signal path. The i4003 Verilator adapter satisfies the JSON schema
but has neither a whole-system stimulus nor shared signal mapping, so
comparison correctly rejects it. The full-system Verilator adapter emits mapped
MCS-4 paths and a scenario-document hash, which supports diagnostic
comparison-surface tests. A behavioral replay transcript and a scenario JSON
are deliberately distinct representations even when a hash is forced equal.
It has not yet replayed the same behavioral ROM and input transcript,
so it does not support a behavioral-to-FPGA equivalence claim.

## FPGA system boundary

`mcs4_system_core.v` is the one shared MCS-4 HDL datapath. It composes generated
4004, 4001, and 4002 models, monitor ROM, RAM, UART bridge, and debug signals.
`mcs4_system_sim_top.v` gives Icarus and Verilator a deterministic external
clock. `mcs4_top.v` is the board wrapper and requires `sys_clk_in`; it does not
invent an oscillator.

Host lint and simulation validate the shared HDL contract. They do not prove
target synthesis, timing closure, a board clock route, or a physical probe.
The `mcs4-virtual-system` Verilator adapter executes this wrapper with staged
monitor ROM input and emits mapped frame records, VCD, and bounded monitor
invariants. The host enforces scenario-byte, action, cumulative-cycle, VCD,
trace-frame, and trace-byte limits and latches bus contention. It supports
comparison-surface diagnosis but does not yet replay a
behavioral ROM transcript with identical input events.
`docs/evidence/fpga-board-clock-and-conformance-blockers.md` defines the
required evidence before programming hardware or promoting a board claim.

## Rust Workspace Structure

```
Cargo.toml                        # Workspace root

mcs4-emu/crates/
  mcs4-core/                      # Simulation kernel (473 tests)
    src/
      lib.rs                      # Re-exports, prelude
      gate.rs                     # Gate primitives: And2, Or2, Nand2/3, Nor2/3, Inv
      signal.rs                   # SignalLevel {High, Low, Z, X}, Signal history
      timing.rs                   # Time type (picoseconds), NANOSECOND/MICROSECOND
      wire.rs                     # Net, Wire, Fanout
      simulator.rs                # Event-driven gate-level simulation engine
      transistor.rs               # PmosFet, DepletionLoad, CircuitBuilder
      transistor_solver.rs        # Switch-level iterative solver (14 tests)
      nodal_solver.rs             # Nodal analysis with RC networks (14 tests)
      trace_buffer.rs             # Signal capture for waveform display
      layout_netlist.rs           # NetlistV1 JSON parser
      netlist_v0.rs               # Legacy netlist parser
      process/                    # Intel 10um pMOS process parameters
        mod.rs                    # ProcessParams, SiliconProperties
        oxide.rs                  # Gate oxide model (tox, Cox, breakdown)
        silicon.rs                # Substrate properties, ni, mobility
        mobility.rs               # Field-dependent mobility models
        junction.rs               # PN junction, built-in potential
        interconnect.rs           # Al metallization R/C
        thermal.rs                # Thermal resistance, self-heating
      device/                     # Transistor device models
        mod.rs                    # DeviceModel trait
        pmos_level1.rs            # Shichman-Hodges Level 1
        pmos_level3.rs            # Level 3 with short-channel effects
        cap_model.rs              # Meyer capacitance model
        depletion_load.rs         # Depletion-load device model
        parasitic.rs              # Layout parasitic extraction
      circuit/                    # Circuit representation
        mod.rs                    # CircuitGraph, Node types
        graph.rs                  # Adjacency-based circuit graph
        arch_map.rs               # Architecture block mapping
        clock_tree.rs             # Clock distribution analysis
        netlist_bridge.rs         # Netlist-to-graph conversion
        parasitic_extract.rs      # R/C extraction from geometry
        power_rail_id.rs          # VDD/VSS identification
        spatial.rs                # Physical coordinate system
        timing_analysis.rs        # Critical path analysis
        bbox_to_geometry.rs       # Bounding box -> geometry conversion
      solver/                     # SPICE-class simulation
        mod.rs                    # Solver orchestration
        dc_op.rs                  # DC operating point
        ac.rs                     # AC small-signal analysis
        transient.rs              # Transient simulation (adaptive timestep)
        convergence.rs            # Newton-Raphson convergence control
        matrix.rs                 # Dense matrix operations
        sparse_matrix.rs          # Sparse matrix (CSR format)
        stimulus.rs               # Voltage/current sources
        event_scheduler.rs        # Event scheduling for mixed-mode
        noise.rs                  # Noise analysis (thermal, flicker)
        sensitivity.rs            # Parameter sensitivity analysis
      tcad/                       # TCAD physics
        mod.rs                    # TCAD module hub
        mesh.rs                   # 2D finite-element mesh
        poisson.rs                # Poisson equation solver
        carrier.rs                # Carrier concentration models
        channel.rs                # Channel formation physics
        drift_diffusion.rs        # Drift-diffusion transport

  mcs4-bus/                       # Bus infrastructure (17 tests)
    src/
      lib.rs                      # Re-exports, prelude
      data_bus.rs                 # 4-bit bidirectional data bus
      control.rs                  # ControlSignals: SYNC, CM-ROM, CM-RAM, IoOp
      clock.rs                    # TwoPhaseClock (phi1/phi2, non-overlapping)

  mcs4-chips/                     # Chip implementations (252 tests)
    src/
      lib.rs                      # Module registry, Chip trait
      disasm.rs                   # Disassembler + DisasmCache (O(1) window)
      simd.rs                     # SIMD parallel execution (nightly, feature-gated)
      i4004/                      # 4004 CPU (46 instructions)
        mod.rs                    # CPU state machine, phase methods
        alu.rs                    # 4-bit ALU with carry/BCD
        registers.rs              # 16 index registers, 3-level stack
        instruction_decode.rs     # Opcode decoder
        timing_io.rs              # I/O timing coordination
      i4040/                      # 4040 CPU (60 instructions)
        mod.rs                    # Extended CPU, interrupt support
        instruction_decode.rs     # 4040-specific opcodes (LCR, RPL, etc.)
        interrupt.rs              # Interrupt controller with vector support
        registers.rs              # 24 registers, register bank switching
        stack.rs                  # 7-level stack
      i4001.rs                    # ROM (256x8) + 4-bit I/O port
      i4002.rs                    # RAM (320-bit) + 4-bit output port
      i4003.rs                    # 10-bit shift register
      i4008.rs                    # 12-bit address latch + CM-ROM decode
      i4009.rs                    # Standard I/O expander
      i3216.rs                    # Bus driver (non-inverting)
      i3226.rs                    # Bus driver (inverting)
      i4101.rs                    # 256x4 static RAM
      i4201.rs                    # Clock generator (crystal, non-overlap, reset, STP)
      i4207.rs                    # Single-phase crystal clock
      i4209.rs                    # Single-to-two-phase converter
      i4211.rs                    # RC oscillator + two-phase clock
      i4265.rs                    # Programmable I/O (4x4 bits)
      i4289.rs                    # Standard memory interface
      i4308.rs                    # 1Kx8 ROM with I/O ports
      i4316.rs                    # LCD segment driver
      i4702.rs                    # 256x8 UV-erasable PROM
    tests/
      fuzz_test.rs                # Fuzz regression (1 test)
      proptest_chips.rs           # Property-based tests (11 tests)

  mcs4-system/                    # System assembly (45 tests)
    src/
      lib.rs                      # Workspace wiring, exports
      mcs4.rs                     # MCS-4 system builder (4004+4001+4002)
      mcs40.rs                    # MCS-40 system builder (4040+4308+4101+4201)
      cluster.rs                  # Multi-CPU cluster wiring
      fixture.rs                  # Test fixtures, hex loader
      simd_cluster.rs             # SIMD parallel cluster (nightly, feature-gated)
    tests/
      mcs40_4308_integration.rs   # End-to-end bus protocol tests (9 tests)

  mcs4-gui/                       # GUI debugger (78 tests)
    src/
      app.rs                      # eframe application shell
      lib.rs
      main.rs
      signal_trace.rs             # SignalTrace: sample capture, overflow eviction
      waveform.rs                 # Legacy waveform scaffolding
      panels/
        mod.rs
        disasm.rs                 # DisasmPanel with cached O(1) windowed lookup
        registers.rs              # Register display with change highlighting
        memory.rs                 # ROM/RAM hex dump with change highlighting
        stack.rs                  # Stack display (3/7-level)
        breakpoints.rs            # Breakpoint management (address/register/memory)
        controls.rs               # Run/Stop/Step/Reset controls
        waveform.rs               # WaveformPanel: logic analyzer view
        die_viewer.rs             # Die photomicrograph overlay (scaffold)

  mcs4-fpga/                      # FPGA synthesis support (42 tests)
    src/
      lib.rs
      verilog.rs                  # Verilog export from gate-level models

  mcs4-intellec/                  # Intellec-4 development system (44 tests)
    src/
      lib.rs
      front_panel.rs              # Front panel (switches, LEDs)
      monitor.rs                  # Monitor ROM (examine/deposit/go/halt)
      prom_programmer.rs          # PROM programmer (4702 interface)
      system.rs                   # System integration

  mcs4-periph/                    # Peripheral devices (30 tests)
    src/
      lib.rs
      seven_segment.rs            # 7-segment LED display
      keyboard.rs                 # 4x4 matrix keyboard scanner
      uart.rs                     # UART serial port (ASR-33 compatible)

  mcs4-core/tests/
    error_paths.rs                # Solver error path validation (12 tests)
    integration_validation.rs     # Cross-solver integration (18 tests)
    nodal_4003.rs                 # 4003 shift register nodal simulation (1 test)
```

## Core Abstractions

### SimulationFidelity and ChipSolverBridge

`SimulationFidelity` (defined in `mcs4-core/src/fidelity.rs`) selects the depth of circuit simulation:

```
Behavioral       -- pure Rust state machine (fastest)
PhaseAccurate    -- bus-cycle-accurate timing
SwitchLevel      -- transistor switch-level (mcs4-core transistor_solver)
NodalLevel       -- analog nodal analysis (mcs4-core nodal_solver)
TCADLevel        -- full device physics via TCAD bridge
```

The `ChipSolverBridge` trait (defined in `mcs4-core/src/bridge.rs`) connects behavioral chip
models to circuit solvers. Each chip that implements `ChipSolverBridge` can escalate its
simulation fidelity on demand, enabling mixed-mode simulation where some chips run at
`Behavioral` level while others run at `NodalLevel` or `TCADLevel`.

The `I4004` clock buffer proof-of-concept (3-inverter chain) demonstrates the full path:
behavioral `tick()` -> `ChipSolverBridge::solve_dc()` / `solve_transient()` -> `DcSolver` /
`TransientSolver` -> `ProcessParams`-based device models.

### Chip Trait

Every chip implements the `Chip` trait for basic lifecycle:

```rust
pub trait Chip: Send + Sync {
    fn name(&self) -> &'static str;
    fn reset(&mut self);
    fn tick(&mut self, phase: BusCycle);
}
```

For bus-connected chips, the `tick_bus()` pattern provides full bus protocol participation:

```rust
fn tick_bus(&mut self, phase: BusCycle, bus: &mut DataBus, ctrl: &ControlSignals);
```

### Bus Protocol

The MCS-4/MCS-40 bus operates on an 8-phase cycle multiplexing address and data on
a shared 4-bit bus:

```
A1: CPU drives address bits 0-3   -> ROM/RAM latch
A2: CPU drives address bits 4-7   -> ROM/RAM latch
A3: CPU drives address bits 8-11  -> ROM/RAM latch + chip select
M1: ROM drives data bits 0-3     -> CPU latches
M2: ROM drives data bits 4-7     -> CPU latches
X1: Decode phase                  -> I/O operations begin
X2: Execute write phase           -> WRM/WMP/WRR: CPU drives, peripherals latch
X3: Execute read phase            -> RDM/RDR: peripherals drive, CPU latches
```

### Control-Line Timing

| Instruction family | io_op asserted | Bus driver | Notes |
|---|---:|---|---|
| WRM, WMP, WRR, WR0..WR3 | X2 | CPU | Write operations |
| RDM, RDR, RD0..RD3, ADM, SBM | X3 | Peripheral | Read operations |
| SRC | X2+X3 | CPU | Address/setup transfer |

### Clock Specifications (from datasheet)

| Parameter | Min | Typ | Max | Unit |
|-----------|-----|-----|-----|------|
| tCY (period) | 1.35 | - | 2.0 | us |
| t0R (rise) | - | 50 | - | ns |
| t0F (fall) | - | 50 | - | ns |
| t0PW (width) | 380 | 480 | - | ns |
| t0D1 (phi1->phi2) | 400 | 550 | - | ns |
| t0D2 (phi2->phi1) | 150 | - | - | ns |

## Test Strategy

- **Unit tests**: Per-chip behavioral correctness (instructions, bus protocol, I/O)
- **Property tests**: Invariant validation across randomized inputs (proptest)
- **Error path tests**: Graceful behavior for missing nodes, empty circuits
- **Integration tests**: End-to-end bus protocol (CPU fetching from ROM via tick_bus)
- **Solver tests**: Convergence, accuracy, cross-validation between solver levels
- **1,128 tests total, 0 failures** (updated 2026-07-11 after 4003 active-low E contract tests; operational metric, see `mcs4-emu/STATUS.md`)

## Build Commands

```sh
cargo check --workspace         # Type check
cargo test --workspace --locked # Run all tests
cargo clippy --all-targets -- -D warnings   # Lint (warnings = errors)
cargo fmt --all --check         # Format check
```

## Dependencies

Key workspace dependencies (pinned in root Cargo.toml):
- `eframe` 0.33: GUI framework (egui + native backend)
- `proptest` 1.4: Property-based testing
- `faer` 0.24: Linear algebra for solver matrices
- `bumpalo` 3.19: Arena allocation for solver temporaries
- `memmap2` 0.9: Memory-mapped ROM loading
- `nalgebra` 0.34: Linear algebra (matrix stamping)

## References

- Intel MCS-4 User Manual (Feb 1973)
- Intel MCS-40 User Manual (Nov 1974)
- Intel 4004/4040 Datasheets
- 4004.com transistor-level masks and schematics
- See `docs/evidence/bibliography.bib` for full bibliography (40+ entries)
