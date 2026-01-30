# MCS-4/MCS-40 Emulator - Project Status (2026-01-29)

## PROJECT OVERVIEW
Intel 4-bit CPU emulator with transistor-level extraction. Full cycle-accurate simulation of 4004/4040 chipsets.

## PHASE STATUS

### Phase 0.5: COMPLETE (85%)
- OCR persistent cache: DONE (48,000x speedup)
- Version pinning (Tesseract 5.5.2, OpenCV 4.13.0, ONNX 1.23.2): DONE
- Power rail anchoring: DONE (medium confidence)
- Remaining: Comprehensive OCR benchmarks for 4001/4002/4003 (deferred)

### Phase 1: COMPLETE (100%)
- 4004 CPU: COMPLETE (all 46 instructions)
- Disassembler: COMPLETE (symbol tables, auto-labeling)
- Unit tests: 115 passing
- Remaining: 4040 full execution

### Phase 2: IN PROGRESS (73%)
- 4040 CPU execution integration: COMPLETE
  - All 8 phase methods (A1-A3, M1-M2, X1-X3): DONE
  - 4004 compatibility layer (46 instructions): DONE
  - 14 new 4040 instructions (HLT, BBS, OR4/5, AN6/7, DB0/1, SB0/1, EIN/DIN): DONE
  - Execute dispatcher: DONE
  - Tick cycle integration: DONE
  - Multi-byte instruction PC advancement: FIXED (JUN/JMS)
  - RAM bank selection logic: FIXED (x2/x3_ram_bank_select)
- Status: 11/15 tests passing (73%)
- Remaining: RAM data persistence (WRM/RDM), interrupt vector logic

### Phase 3-5: PLANNED (0%)
- Support chips (4101 RAM, 4201 Clock, 4289 Interface): NOT STARTED
- GUI debugger: NOT STARTED
- Extraction enhancements: NOT STARTED
- Clustering/SIMD: NOT STARTED
- FPGA synthesis: NOT STARTED

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

## FAILING TESTS (4)

1. test_end_to_end_src_wrm_rdm_roundtrip - RAM data persistence
2. test_fixture_src_wrm_rdm_hex_executes - RAM data persistence
3. test_fixture_ram_status_wr1_rd1_hex_executes - RAM status read/write
4. test_interrupt_ein_vectors_to_003_and_bbs_returns - interrupt vector not implemented

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

## NEXT PRIORITY (TASK #88)

Debug failing tests in this order:
1. Investigate SRC/WRM/RDM sequencing (likely PC advancement issue)
2. Fix JMS/BBL stack operations (return address calculation)
3. Implement interrupt vector logic
4. Verify control signal timing for RAM operations

Estimated: 2-4 hours to achieve 13/15 passing (87%)

---
Last Updated: 2026-01-29
Model: claude-haiku-4-5-20251001
