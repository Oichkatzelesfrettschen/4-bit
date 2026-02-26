# MCS-4 Emulator Project Status

**Last Updated:** 2026-02-25
**Repository:** https://github.com/Oichkatzelesfrettschen/4-bit

## Phase Summary (85% overall)

| Phase | Description | Status | % |
|-------|-------------|--------|---|
| 0 | Repo hygiene and reproducibility | COMPLETE | 100% |
| 0.5 | OCR pipeline and evidence extraction | COMPLETE | 90% |
| 1 | CPU correctness (4004) | COMPLETE | 100% |
| 2 | 4040 CPU complete | COMPLETE | 100% |
| 3 | Support chips + GUI | COMPLETE | 100% |
| 4 | Performance and clustering | COMPLETE | 100% |
| 5 | FPGA and advanced features | SCAFFOLDING | 5% |

## Test Counts (baseline 2026-02-25)

902 tests passing with SIMD feature (858 without), 0 failures:
- mcs4-bus: 17
- mcs4-chips: 212 (+ 1 fuzz_test + 11 proptest_chips)
- mcs4-core: 454 (+ 12 error_paths + 18 integration_validation)
- mcs4-fpga: 6
- mcs4-gui: 75 (signal trace, disasm, registers, memory, stack, breakpoints, controls, waveform)
- mcs4-system: 87 with simd_cluster / 43 without (+ 9 mcs40_4308_integration)
- mcs4-intellec: 0 (scaffold)
- mcs4-periph: 0 (scaffold)

## Chip Implementation Status

### MCS-4 Family (4004-based)

| Chip | Description | Status |
|------|-------------|--------|
| 4004 | 4-bit CPU | COMPLETE (46 instructions, full ALU) |
| 4001 | 256x8 ROM + 4-bit I/O | COMPLETE (bus protocol, chip select) |
| 4002 | 320-bit RAM + 4-bit output | COMPLETE (bus protocol, bank select) |
| 4003 | 10-bit shift register | COMPLETE (shift, cascade, port-driven, enable, 16 tests) |

### MCS-40 Family (4040-based)

| Chip | Description | Status |
|------|-------------|--------|
| 4040 | Enhanced 4-bit CPU | COMPLETE (60 instructions, interrupts, 43 tests) |
| 4101 | 256x4 static RAM | COMPLETE (read/write, 17 tests) |
| 4201 | Clock generator | COMPLETE (crystal config, non-overlap, reset, STP, 13 tests + proptest) |
| 4289 | Standard memory interface | COMPLETE (address latch, nibble assembly, OE/WE, 13 tests + proptest) |
| 4308 | 1Kx8 ROM | COMPLETE (1Kx8 storage, I/O ports, bus protocol, 13 tests + proptest + 9 integration) |

### MCS-4 Support Chips

| Chip | Description | Status |
|------|-------------|--------|
| 4008 | 12-bit address latch + CM-ROM decode | COMPLETE (10 tests) |
| 4009 | Standard I/O expander | COMPLETE (8 tests) |
| 3216 | 4-bit bus driver (non-inverting) | COMPLETE (8 tests) |
| 3226 | 4-bit bus driver (inverting) | COMPLETE (8 tests) |

### MCS-40 Clock Generators

| Chip | Description | Status |
|------|-------------|--------|
| 4207 | Single-phase crystal clock | COMPLETE (6 tests) |
| 4209 | Single-to-two-phase converter | COMPLETE (5 tests) |
| 4211 | RC oscillator + two-phase clock | COMPLETE (6 tests) |

### MCS-40 Peripheral Chips

| Chip | Description | Status |
|------|-------------|--------|
| 4265 | Programmable I/O (4x4 bits) | COMPLETE (9 tests) |
| 4316 | LCD segment driver | COMPLETE (7 tests) |
| 4702 | 256x8 UV-erasable PROM | COMPLETE (8 tests) |

### Not Started (deferred)

4002-1/4002-2, 4269, 3205, 3404, 2101, 2102, 1302, second-sources, TTL glue, peripherals.

## Architecture

- **Language:** Rust (8 crates in workspace)
- **Accuracy:** Gate-level with transistor/nodal solvers, SPICE-class circuit simulation (450+ tests)
- **SIMD:** 16-lane parallel 4004 execution with full ISA, differential fuzzing (nightly feature)
- **Process models:** Intel 10um pMOS: I/O drivers, power, ESD, ROM/SRAM cells
- **Approach:** Cleanroom from primary Intel documentation
- **Scope:** MCS-4 (4004) + MCS-40 (4040) chip families

## Recent Session Log

- 2026-02-25: Phase 4 complete -- full SIMD ISA (46 instructions), solver bridge, process models, 902 total
  - Wave 1: Fixed SIMD opcode dispatch bugs (ADD/SUB writeback, removed phantom DEC)
  - Wave 2: All 14 accumulator ops (CLB/CLC/CMC/CMA/IAC/DAC/RAL/RAR/TCC/TCS/DAA/KBP/STC/DCL)
  - Wave 3: LDM, BBL, SRC, FIN/JIN, I/O stubs
  - Wave 4: Two-byte fetch infrastructure, JUN/JMS/JCN/ISZ/FIM
  - Wave 5: Scalar reference executor, differential fuzzing with proptest
  - Wave 6: SimulationFidelity, ChipSolverBridge trait, I4004 clock buffer PoC
  - Wave 7: I/O driver, power, ESD, ROM cell, SRAM cell process models
  - Wave 8: mcs4-intellec + mcs4-periph scaffolds, status sync
- 2026-02-25: E.6 waveform viewer -- cursors, measurement markers, signal grouping, 16 new tests, 820 total
- 2026-02-25: E.3-E.5 GUI panels -- stack panel (9 tests), breakpoint panel (13 tests), controls panel (8 tests), 804 total
- 2026-02-25: E.1-E.2 GUI panels -- register panel (8 tests), memory panel (6 tests), 774 total
- 2026-02-25: D.3.2 GUI tests -- 8 new headless tests (signal trace overflow/phase/data, disasm panel data flow), 760 total
- 2026-02-25: D.3.3 disasm cache -- DisasmCache with O(1) windowed lookup, DisasmPanel integration, 10 new tests, 752 total
- 2026-02-25: D.3.1 integration -- 9 MCS-40/4308 ROM bus protocol end-to-end tests, 742 total
- 2026-02-25: D.2 test debt -- 11 proptest (4201/4289/4308 invariants), 12 error path tests (nodal/transistor solver)
- 2026-02-25: BOM buildout -- 10 new chip implementations (4008, 4009, 3216, 3226, 4207, 4209, 4211, 4265, 4316, 4702), 75 new tests, bibliography expansion (16 entries), evidence infrastructure, debt resolution
- 2026-02-25: BOM audit -- manifest reconciliation, URL provenance verification, download scripting, bibliography completion, egui 0.29->0.33, PDF dedup
- 2026-02-25: Debt audit and resolution (organizational, dependency, CI, test, docs, code stubs)
- 2026-01-29: Phase 4F SIMD cluster benchmarking, Phase 5 FPGA design complete
- 2026-01-14: Phase 0.5 evidence consolidation, power rail anchors upgraded

For full session history, see `docs/archive/SESSION_LOG.md`.

## Status File Convention

- **CLAUDE.md**: Canonical status (phase %, test counts, priorities) -- single source of truth
- **ROADMAP.md**: Forward plan (what to build next, dependency order)
- **STATUS.md**: Session log (recent work, chip status tables)
- All three synchronized after each milestone; contradictions are bugs.
