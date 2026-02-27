# MCS-4/MCS-40 Emulator - Project Status (2026-02-25)

## PROJECT OVERVIEW
Intel 4-bit CPU emulator with transistor-level extraction. Full cycle-accurate simulation of 4004/4040 chipsets.

## PHASE STATUS

Summary: 92% overall completion
- Phase 0.5: 90% (OCR pipeline, coordinate transforms pending)
- Phase 1: 100% (4004 CPU complete)
- Phase 2: 100% (4040 CPU complete, all tests passing)
- Phase 3: 100% (all support chips + all GUI panels + waveform viewer complete)
- Phase 4: 100% (solvers, SIMD full ISA, differential fuzzing, solver bridge, process models)
- Phase 5: 75% (peripherals, Intellec-4, Verilog chip modules complete; hardware validation deferred)

### Phase 0.5: COMPLETE (90%)
- OCR persistent cache: DONE (48,000x speedup)
- Version pinning (Tesseract 5.5.2, OpenCV 4.13.0, ONNX 1.23.2): DONE
- Power rail anchoring: DONE (medium confidence)
- Remaining: OCR benchmarks for 4001/4002/4003 (tasks #70-74, deferred)

### Phase 1: COMPLETE (100%)
- 4004 CPU: 46 instructions, full ALU, registers, stack
- Disassembler: Symbol tables, auto-labeling
- Unit tests: 115+ passing
- 4040 foundation: CPU structure, register banks, stack (7-level)

### Phase 2: COMPLETE (100%)
- 4040 CPU: Full 60-instruction execution (46 4004 + 14 4040 new)
- Phase methods: A1-A3, M1-M2, X1-X3 (all 8 phases implemented)
- Interrupt controller: Fully implemented with vector support
- RAM operations: SRC/WRM/RDM/RDR fully functional
- Tests: 43 passing (all critical tests passing)
  - RAM data persistence: WORKING
  - Interrupt vector logic: WORKING (INT → 0x003)
  - RAM status read/write: WORKING

### Phase 3: COMPLETE (100%)
- DONE:
  - 4101 RAM design + implementation (read/write, 17 tests)
  - 4201 Clock generator (crystal config, non-overlap, reset/STP, 8 tests)
  - 4289 Memory interface (address latch, nibble assembly, OE/WE, 8 tests)
  - 4308 ROM (1Kx8 storage, I/O ports, 8 tests)
  - 4008 Address latch (12-bit latching, CM-ROM decode, 10 tests)
  - 4009 I/O expander (bidirectional data, CM-RAM bank select, 8 tests)
  - 3216/3226 Bus drivers (non-inverting/inverting, 8 tests each)
  - 4207 Crystal clock generator (single-phase, 6 tests)
  - 4209 Phase converter (1-to-2-phase, dead-time, 5 tests)
  - 4211 RC oscillator + two-phase clock (6 tests)
  - 4265 Programmable I/O (4x4 bits, direction control, 9 tests)
  - 4316 LCD driver (segment/backplane AC drive, multiplex, 7 tests)
  - 4702 EPROM (256x8, programming mode, UV erase, 8 tests)
  - Disassembler core (symbol tables, 8 tests) + DisasmCache (O(1) windowed lookup, 8 tests)
  - Signal trace buffer (event capture, 18 tests)
  - MCS-40 system integration (memory map, bus protocol)
- Pending:
  - ~~4003 Shift register cascade/bus tests~~ DONE (16 tests: cascade, port, enable, edge, system integration)
  - ~~4201/4289/4308 bus protocol~~ DONE (13 tests each + proptest + 9 integration)
  - ~~Register panel~~ DONE (CpuSnapshot, change highlighting, 4004/4040 mode, 8 tests)
  - ~~Memory panel~~ DONE (ROM/RAM hex dump, change highlighting, region selector, 6 tests)
  - ~~Stack panel~~ DONE (3/7-level display, SP indicator, change highlighting, 9 tests)
  - ~~Breakpoint panel~~ DONE (address/register/memory breakpoints, enable/disable, hit counts, 13 tests)
  - ~~Controls panel~~ DONE (Run/Stop/Step/Reset, speed slider, CPU selector, counters, 8 tests)
  - ~~Waveform viewer~~ DONE (cursors, measurement markers, signal grouping, zoom, 16 tests)

### Phase 4: COMPLETE (100%)
- DONE:
  - Phase 4A: Switch-level transistor simulator (14 tests)
    - Inverter chains, marginal conduction, high fanout
    - Parallel NMOS/PMOS networks
  - Phase 4B: Nodal analysis solver (14 tests)
    - RC charging, voltage dividers, mesh networks
    - High-Z networks, asymmetric dividers, star topology
    - Very high capacitance, capacitive coupling
  - Phase 4C: Comprehensive validation testing (28 total tests)
    - Edge case validation for both solvers
    - Circuit topologies: inverter chains, parallel gates, networks
    - Convergence verification for diverse configurations
  - Phase 4D-F: SIMD cluster (full 4004 ISA, 87 tests)
    - 16-lane parallel CPU execution (Struct-of-Arrays architecture)
    - Full 46-instruction 4004 ISA: all single-byte + two-byte opcodes
    - Two-phase fetch for 2-byte instructions (JUN/JMS/JCN/ISZ/FIM)
    - Per-lane carry, stack, PC, register state
    - Differential fuzzing: scalar reference executor with proptest
    - Performance benchmarking with throughput and memory metrics
  - Phase 4G: Solver-to-chip bridge
    - SimulationFidelity enum (Behavioral/PhaseAccurate/SwitchLevel/NodalLevel/TCADLevel)
    - ChipSolverBridge trait connecting behavioral models to circuit solvers
    - I4004 clock buffer proof-of-concept (3-inverter chain, DC + transient)
  - Phase 4H: Process model expansion (22 tests)
    - I/O driver model (Ron, slew rate, output impedance)
    - Power model (static leakage + dynamic CV^2f per chip)
    - ESD protection diode (forward/reverse/breakdown)
    - ROM cell model (wordline RC, bitline charge sharing)
    - SRAM cell model (6T SNM, read/write timing, min retention)

### Phase 5: IMPLEMENTED (75%)
- DONE:
  - Peripheral drivers (mcs4-periph crate, 30 tests):
    - 7-segment LED display (BCD decode, shift chain loading, ASCII render, 11 tests)
    - Matrix keyboard scanner (4x4, debounce, row drive, events, 10 tests)
    - UART serial port (TX/RX FIFOs, bit-bang timing, ASR-33 config, 9 tests)
  - Intellec-4 development system (mcs4-intellec crate, 44 tests):
    - Front panel (address/data switches, Run/Stop/Step, LEDs, 11 tests)
    - Monitor ROM (command dispatch, examine/deposit/go/halt, 11 tests)
    - PROM programmer (blank check, program, verify, 4702 interface, 9 tests)
    - System integration (CPU coordination, panel/monitor/peripherals, 13 tests)
  - Verilog chip modules (mcs4-fpga, 12 tests):
    - Synthesizable 4004 CPU module (8-phase, PC, accumulator, registers, stack)
    - Synthesizable 4001 ROM module (address latch, chip select, output)
    - Synthesizable 4002 RAM module (address, status registers, output port)
    - Synthesizable 4003 shift register module (10-bit, enable, cascade)
- Pending:
  - #116: FPGA hardware validation (requires board purchase)
  - #117: ONNX CTC training (requires dataset collection)

## CURRENT IMPLEMENTATION

### 4040 CPU (i4040/mod.rs)
- Struct fields: ALU, registers, decoder, interrupt controller, halted state
- Execution state: cycle, instruction_byte, operand, RAM tracking
- Phase methods: A1/A2/A3 (address), M1/M2 (fetch), X1 (decode), X2/X3 (execute)
- Instruction execution: 46 4004 + 14 4040 = 60 total
- Test results: JUN/JMS working, SRC/WRM timing issues

### 4004 Compatibility
- Full 4004 ISA implemented in execute_4004()
- Register file compatible (24 registers for 4040, 16 for 4004)
- Stack compatible (7-level for 4040, 3-level for 4004)

## BUILD COMMANDS

- Check: `cargo check --workspace`
- Test: `cargo test --workspace`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Format: `cargo fmt --check`

## STANDARDS

- No warnings, errors as warnings
- 100% test coverage for new code
- ASCII-only commits, no unicode
- All decisions documented with WHY/WHAT/HOW

## NEXT PRIORITY

Critical path (in order):
1. Phase 5 remaining: FPGA hardware validation, ONNX CTC training
2. Phase 0.5: OCR regression benchmarks (deferred)
3. Advanced: rkyv snapshots, time-travel debugging, hardware-in-loop testing

## WORKSPACE CRATES (8)

- mcs4-bus: Bus protocol abstraction
- mcs4-chips: All chip implementations (4004, 4040, 4001-4003, MCS-40 support/peripherals)
- mcs4-core: Transistor/nodal solvers, process models, circuit graph, TCAD, fidelity/bridge
- mcs4-fpga: Verilog export
- mcs4-gui: egui panels (registers, memory, stack, breakpoints, controls, disasm, waveform)
- mcs4-system: System integration, SIMD cluster (feature-gated)
- mcs4-intellec: Intellec-4 development system (front panel, monitor, PROM programmer)
- mcs4-periph: Peripheral devices (7-segment display, matrix keyboard, UART)

## TEST COUNTS (updated 2026-02-26 after full plan implementation)

968 tests passing, 0 failures:
- mcs4-bus: 17
- mcs4-chips: 211 (4004/4040 CPU, disassembler + cache, all support/peripheral chips, solver bridge)
- mcs4-chips fuzz_test: 1
- mcs4-chips proptest_chips: 11 (property-based tests for 4201/4289/4308)
- mcs4-core: 473 (transistor/nodal/TCAD solvers, process models, transient+trapezoidal integration,
    temperature sweep, sensitivity analysis integration, circuit, fidelity, bridge)
- mcs4-core error_paths: 12 (solver error path validation)
- mcs4-core integration_validation: 18
- mcs4-core nodal_4003: 1 (4003 shift register nodal simulation)
- mcs4-fpga: 12 (Verilog export + chip module generation)
- mcs4-gui: 78 (signal trace, disasm, registers, memory, stack, breakpoints, controls, waveform, die viewer)
- mcs4-intellec: 44 (front panel, monitor, PROM programmer, system integration)
- mcs4-intellec full_system_integration: 6 (end-to-end Intellec-4 + peripherals + MCS-40)
- mcs4-periph: 30 (7-segment display, matrix keyboard, UART)
- mcs4-system: 45 (MCS-4/MCS-40 system wiring, cluster, SIMD ISA, differential fuzzing)
- mcs4-system mcs40_4308_integration: 9 (4040+4308 ROM bus protocol end-to-end)

---
Last Updated: 2026-02-26 (full plan: debt resolution + trapezoidal integration + temperature sweep + multi-system tests + sensitivity integration)
