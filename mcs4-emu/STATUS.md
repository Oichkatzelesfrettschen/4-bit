# MCS-4 Emulator Project Status

**Last Updated:** 2025-12-29
**Repository:** https://github.com/Oichkatzelesfrettschen/4-bit

## Project Goal

Build a cycle-accurate, timing-accurate, and electrically-accurate virtual MCS-4 and MCS-40 system with GUI emulator/debugger/programmer, including all support chips (ROM/RAM/I/O).

## Architecture Decisions

- **Language:** Rust with FPGA synthesis support via mcs4-fpga crate
- **Accuracy Level:** Gate-level with transistor-level stubs for future enhancement
- **Approach:** Cleanroom implementation while auditing OpenCores Verilog for reference
- **Scope:** Both MCS-4 (4004) and MCS-40 (4040) systems in parallel

## Completed Tasks

### Core Infrastructure
- [x] mcs4-core: Timing constants, signal types, gate primitives, event-driven simulator
- [x] mcs4-bus: Two-phase clock, 4-bit data bus, control signals, 8-phase cycle state
- [x] Workspace structure with 6 crates (core, bus, chips, system, fpga, gui)

### 4004 CPU Implementation
- [x] Full instruction decoder for all 46 opcodes
- [x] Complete execute() method with all instruction implementations
- [x] JCN condition evaluation (invert, acc-zero, carry, test-pin)
- [x] Register file (16 x 4-bit registers, 8 pairs, 3-level stack)
- [x] ALU with accumulator, carry, rotate, KBP, DAA

### Support Chips
- [x] 4001 ROM: 256x8 memory + 4-bit I/O port + bus protocol
- [x] 4002 RAM: 4 registers x 16 characters + 4 status + output port + bus protocol
- [x] Bus protocol timing: A1-A3 (address), M1-M2 (memory), X1-X3 (execute)

### System Integration
- [x] Mcs4System with proper bus timing (ROM/RAM respond before CPU reads)
- [x] System configurations: minimal(), standard(), maximal()
- [x] Breakpoint support with run_until_breakpoint()
- [x] Memory inspection: read_rom(), read_ram()
- [x] CPU state access: pc(), accumulator(), carry(), register(), register_pair()

### Testing
- [x] 66 tests passing across all crates
- [x] Integration tests for LDM instruction execution
- [x] Breakpoint functionality tested

## Remaining Tasks

### 1. Implement 4040 CPU (High Priority)

The Intel 4040 extends the 4004 with:
- **24 index registers** (vs 16 in 4004) - registers R16-R23
- **7-level stack** (vs 3-level in 4004)
- **Interrupt support** with INT input and acknowledgment
- **HALT/STOP instruction** and single-step capability
- **Additional instructions:**
  - HLT (halt processor)
  - BBS (branch back from interrupt and restore)
  - LCR (load DCL into accumulator)
  - OR4/OR5 (logical OR with register pair)
  - AN4/AN5 (logical AND with register pair)
  - DB0/DB1 (select register bank 0/1)
  - EIN/DIN (enable/disable interrupt)
  - RPM (read program memory - for self-modifying code)

**Files to create/modify:**
- `crates/mcs4-chips/src/i4040/mod.rs` - Main 4040 implementation
- `crates/mcs4-chips/src/i4040/registers.rs` - Extended register file
- `crates/mcs4-chips/src/i4040/interrupt.rs` - Interrupt controller
- `crates/mcs4-system/src/mcs40.rs` - MCS-40 system integration

**Reference:** docs/MCS-40/ directory contains Intel documentation

### 2. Build GUI Waveform Viewer (Medium Priority)

Create egui-based waveform display showing:
- PHI1/PHI2 clock signals
- SYNC signal
- CM-ROM/CM-RAM control lines
- 4-bit data bus values
- CPU state (PC, accumulator, instruction)

**Technical approach:**
- Use signal history from `Signal::history()` method
- Implement zoom/pan for time navigation
- Color-code different signal types
- Show decoded instructions alongside waveforms

**Files to modify:**
- `crates/mcs4-gui/src/main.rs` - Already has egui setup
- Create `crates/mcs4-gui/src/waveform.rs` - Waveform rendering
- Create `crates/mcs4-gui/src/timing_diagram.rs` - Signal layout

### 3. Add Disassembler and Debugger UI (Medium Priority)

**Disassembler features:**
- Decode ROM contents to assembly
- Show instruction addresses and hex bytes
- Handle two-byte instructions correctly
- Mark current PC position

**Debugger UI features:**
- Register display (R0-R15 or R0-R23 for 4040)
- Stack display (3 or 7 levels)
- Memory viewer (ROM and RAM)
- Single-step execution
- Run/pause/reset controls
- Breakpoint management UI

**Files to create:**
- `crates/mcs4-chips/src/disasm.rs` - Disassembler logic
- `crates/mcs4-gui/src/debugger.rs` - Debugger panel
- `crates/mcs4-gui/src/memory_view.rs` - Memory hex viewer

### 4. Additional Support Chips (Lower Priority)

- **4003:** 10-bit shift register for I/O expansion
- **4008/4009:** Standard memory interface
- **4289:** Standard memory interface (MCS-40)
- **4308/4316:** ROM variants (stubs exist)

### 5. FPGA Verilog Export (Future)

The mcs4-fpga crate stub exists for generating synthesizable Verilog from the Rust gate-level model. This enables running the emulator on real FPGA hardware.

## Technical Notes

### Bus Protocol Timing

The MCS-4 uses an 8-phase bus cycle:
```
A1 -> A2 -> A3 -> M1 -> M2 -> X1 -> X2 -> X3
 |     |     |     |     |     |     |     |
 |     |     |     |     |     |     |     +-- I/O read (RDM, RDR, etc.)
 |     |     |     |     |     |     +-------- I/O write (WRM, WRR, etc.)
 |     |     |     |     |     +-------------- Decode instruction
 |     |     |     |     +-------------------- Read OPR (high nibble)
 |     |     |     +-------------------------- Read OPA (low nibble)
 |     |     +-------------------------------- Select ROM chip (CM-ROM)
 |     +-------------------------------------- Address bits 4-7
 +-------------------------------------------- Address bits 0-3, SYNC
```

**Critical timing:** ROM/RAM must put data on bus BEFORE CPU reads during M1/M2/X3 phases.

### Instruction Encoding

Single-byte instructions: 0x00-0xCF, 0xE0-0xFF
Two-byte instructions: 0x10-0x1F (JCN), 0x20-0x2F (FIM/SRC/FIN/JIN), 0x40-0x4F (JUN), 0x50-0x5F (JMS), 0x70-0x7F (ISZ)

### Clock Specifications

- Typical frequency: 740 kHz
- Period: ~1.35 us (1,350,000 ps)
- Non-overlapping PHI1/PHI2 with defined rise/fall times

## File Structure

```
mcs4-emu/
├── Cargo.toml                 # Workspace manifest
├── STATUS.md                  # This file
├── crates/
│   ├── mcs4-core/            # Timing, signals, gates, simulator
│   ├── mcs4-bus/             # Clock, data bus, control, cycles
│   ├── mcs4-chips/           # 4004, 4040, 4001, 4002, etc.
│   ├── mcs4-system/          # MCS-4 and MCS-40 system integration
│   ├── mcs4-fpga/            # Verilog export (stub)
│   └── mcs4-gui/             # egui-based GUI (stub)
└── src/
    └── main.rs               # CLI entry point
```

## Running Tests

```bash
cd mcs4-emu
cargo test              # Run all 66 tests
cargo test -p mcs4-system  # Test system integration only
cargo run               # Run GUI (stub)
```

## References

- `docs/MCS-4/` - Intel 4004/4001/4002/4003 documentation
- `docs/MCS-40/` - Intel 4040 and Intellec 4 MOD 40 documentation
- Original MCS-4 User Manual (1971)
- Intel Intellec 4 MOD 40 Reference Schematics (78MB PDF)
