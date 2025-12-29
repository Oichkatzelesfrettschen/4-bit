# MCS-4 Emulator Project Status

**Last Updated:** 2025-12-29
**Repository:** https://github.com/Oichkatzelesfrettschen/4-bit

## Project Goal

Build a cycle-accurate, timing-accurate, and electrically-accurate virtual MCS-4 and MCS-40 system with GUI emulator/debugger/programmer, including ALL support chips from the original Intel kits plus era-appropriate peripherals.

## Architecture Decisions

- **Language:** Rust with FPGA synthesis support via mcs4-fpga crate
- **Accuracy Level:** Gate-level with transistor-level stubs for future enhancement
- **Approach:** Cleanroom implementation while auditing OpenCores Verilog for reference
- **Scope:** Complete MCS-4 (4004) and MCS-40 (4040) chip families

---

## Chip Implementation Status

### Legend
- **COMPLETE** - Fully implemented with bus protocol and tests
- **STUB** - Basic structure exists, needs full implementation
- **NOT STARTED** - Needs to be created

---

### MCS-4 Family (4004-based, 1971)

| Chip | Description | Status | Notes |
|------|-------------|--------|-------|
| **4004** | 4-bit CPU | **COMPLETE** | All 46 instructions, ALU, registers, 3-level stack |
| **4001** | 256x8 ROM + 4-bit I/O | **COMPLETE** | Bus protocol, chip select, I/O ports |
| **4002** | 320-bit RAM + 4-bit output | **COMPLETE** | 4 regs x 16 chars + status, bus protocol |
| **4003** | 10-bit shift register | STUB | Basic shift logic, needs bus/clock integration |
| **4008** | Address latch (8-bit) | NOT STARTED | For standard memory interface |
| **4009** | I/O buffer/interface | NOT STARTED | Bidirectional bus interface |

### MCS-40 Family (4040-based, 1974)

| Chip | Description | Status | Notes |
|------|-------------|--------|-------|
| **4040** | Enhanced 4-bit CPU | STUB | Needs 24 regs, 7-level stack, interrupts |
| **4101** | 256x4 static RAM | STUB | Basic storage, needs bus protocol |
| **4201** | Clock generator | STUB | Generates PHI1/PHI2 from crystal |
| **4207** | General purpose I/O | NOT STARTED | Parallel I/O expander |
| **4209** | Address latch | NOT STARTED | For program memory interface |
| **4211** | Address latch | NOT STARTED | Variant of 4209 |
| **4265** | Programmable peripheral interface | NOT STARTED | Like 8255 PPI, 24 I/O lines |
| **4269** | Keyboard/display interface | NOT STARTED | Scans keyboard, drives display |
| **4289** | Standard memory interface | STUB | Connects to standard memory |
| **4308** | 1Kx8 ROM | STUB | Basic storage, needs bus protocol |
| **4316** | 2Kx8 ROM | NOT STARTED | Larger ROM variant |
| **4702** | 256x8 EPROM | NOT STARTED | UV-erasable PROM |

### Era-Appropriate Peripherals (1971-1976)

| Category | Chips | Status | Notes |
|----------|-------|--------|-------|
| **Display Drivers** | 7-segment LED drivers | NOT STARTED | Common anode/cathode support |
| **Display Drivers** | Character LCD controllers | NOT STARTED | If era-appropriate |
| **Keyboard Interface** | Matrix keyboard scanner | NOT STARTED | Directly or via 4269 |
| **Printers** | Parallel printer interface | NOT STARTED | Centronics-style |
| **Storage** | Paper tape reader | NOT STARTED | Era-appropriate I/O |
| **Storage** | Cassette interface | NOT STARTED | Audio FSK modulation |
| **Communication** | Serial UART | NOT STARTED | RS-232 interface |
| **Expansion** | Bus buffers/transceivers | NOT STARTED | 74xx series compatible |

---

## Detailed Implementation Notes

### Completed Chips

#### 4004 CPU (crates/mcs4-chips/src/i4004/)
- **instruction_decode.rs**: Complete decoder for all 46 instructions
- **alu.rs**: Accumulator, carry, rotate, KBP, DAA operations
- **registers.rs**: 16 x 4-bit registers (8 pairs), 12-bit PC, 3-level stack
- **timing_io.rs**: Bus cycle timing and I/O control
- **mod.rs**: Execute logic, phase handlers (A1-X3)

#### 4001 ROM (crates/mcs4-chips/src/i4001.rs)
- 256 x 8-bit ROM storage
- 4-bit bidirectional I/O port
- CM-ROM chip select (0-15)
- Full bus protocol: A1-A3 address latch, M1-M2 data output, X2-X3 I/O

#### 4002 RAM (crates/mcs4-chips/src/i4002.rs)
- 4 registers x 16 characters x 4 bits (64 nibbles main memory)
- 4 status characters x 4 bits (16 nibbles status)
- 4-bit output port latch
- SRC addressing (chip/register/character)
- CM-RAM bank select (0-3)
- Full bus protocol for read/write operations

### Stub Chips Needing Implementation

#### 4040 CPU (HIGH PRIORITY)
Extended 4004 with:
- 24 index registers (R0-R23) vs 16 in 4004
- Register bank switching (DB0/DB1 instructions)
- 7-level stack vs 3-level
- Interrupt support (INT pin, EIN/DIN, BBS instructions)
- HALT/STOP with single-step capability
- Additional instructions: HLT, BBS, LCR, OR4/OR5, AN4/AN5, RPM

**Files to modify:**
```
crates/mcs4-chips/src/i4040/
├── mod.rs           # Main implementation
├── registers.rs     # Extended 24-register file
├── interrupt.rs     # Interrupt controller (NEW)
└── instruction_decode.rs  # Extended decoder (NEW)
```

#### 4003 Shift Register (LOW PRIORITY)
10-bit serial-in, parallel-out shift register for I/O expansion.
Current stub has basic shift logic but needs:
- Clock input handling
- Serial data input/output
- Parallel output latching
- Cascade support for >10 bits

#### 4101 Static RAM (MEDIUM PRIORITY)
256 x 4-bit static RAM for MCS-40 systems.
Needs:
- Full bus protocol implementation
- Address decoding
- Read/write timing
- Chip enable logic

#### 4201 Clock Generator (LOW PRIORITY)
Generates two-phase non-overlapping clocks from crystal.
Currently the clock is software-generated in mcs4-bus.
This chip would provide hardware-accurate timing including:
- Crystal oscillator interface
- PHI1/PHI2 generation with proper timing
- SYNC signal generation

#### 4289 Standard Memory Interface (MEDIUM PRIORITY)
Allows 4040 to use standard memory chips (not 4001/4002).
Needs:
- Address multiplexing
- Read/write control generation
- Timing for standard memory

#### 4308/4316 ROMs (LOW PRIORITY)
Larger ROM variants (1K/2K x 8-bit).
Similar to 4001 but:
- Larger address space
- Different chip select scheme
- No I/O ports

### Chips to Create from Scratch

#### 4265 Programmable Peripheral Interface (MEDIUM PRIORITY)
Similar to Intel 8255 PPI:
- 24 programmable I/O lines
- 3 ports (A, B, C) with mode control
- Directly interfaces with 4-bit bus

#### 4269 Keyboard/Display Interface (MEDIUM PRIORITY)
Specialized chip for human interface:
- Scans up to 64-key keyboard matrix
- Drives up to 16 7-segment displays
- Debouncing and encoding
- Interrupt on keypress

#### Display Drivers
For authentic period displays:
- 7-segment LED (common anode/cathode)
- Nixie tube interface (via BCD decoder)
- LED matrix support

---

## System Integration Status

### MCS-4 System (crates/mcs4-system/src/mcs4.rs)
- **COMPLETE**: Full integration with proper bus timing
- Configurations: minimal (1 ROM, 1 RAM), standard (4 ROM, 8 RAM), maximal (16 ROM, 16 RAM)
- Breakpoint support
- Memory inspection
- CPU state access

### MCS-40 System (crates/mcs4-system/src/mcs40.rs)
- **NOT STARTED**: Needs to be created after 4040 CPU is complete
- Will support 4040 + extended chip set
- Interrupt handling
- Standard memory interface support

---

## GUI and Tools Status

### Waveform Viewer (crates/mcs4-gui/)
- **NOT STARTED**: egui framework is set up
- Needs signal capture from simulation
- Time-based visualization of PHI1, PHI2, SYNC, data bus
- Zoom/pan navigation

### Disassembler
- **NOT STARTED**: Create crates/mcs4-chips/src/disasm.rs
- Decode ROM to assembly mnemonics
- Handle 1-byte and 2-byte instructions
- Support both 4004 and 4040 instruction sets

### Debugger UI
- **NOT STARTED**: Needs disassembler first
- Register display
- Memory viewer (ROM/RAM)
- Stack display
- Single-step execution
- Breakpoint management

---

## Test Coverage

| Crate | Tests | Status |
|-------|-------|--------|
| mcs4-core | 22 | PASS |
| mcs4-bus | 16 | PASS |
| mcs4-chips | 23 | PASS |
| mcs4-system | 5 | PASS |
| **Total** | **66** | **ALL PASS** |

---

## Technical Reference

### Bus Protocol Timing
```
Cycle:  A1 -> A2 -> A3 -> M1 -> M2 -> X1 -> X2 -> X3
        |     |     |     |     |     |     |     |
        |     |     |     |     |     |     |     +-- I/O read (RDM, RDR)
        |     |     |     |     |     |     +-------- I/O write (WRM, WRR)
        |     |     |     |     |     +-------------- Decode instruction
        |     |     |     |     +-------------------- Read OPR (high nibble)
        |     |     |     +-------------------------- Read OPA (low nibble)
        |     |     +-------------------------------- Select ROM (CM-ROM)
        |     +-------------------------------------- Address bits 4-7
        +-------------------------------------------- Address bits 0-3, SYNC
```

### Instruction Encoding Quick Reference
- `0x00-0x0F`: NOP, JCN conditions
- `0x10-0x1F`: JCN (2-byte conditional jump)
- `0x20-0x2F`: FIM, SRC, FIN, JIN
- `0x30-0x3F`: FIM data, SRC pairs
- `0x40-0x4F`: JUN (2-byte unconditional jump)
- `0x50-0x5F`: JMS (2-byte call)
- `0x60-0x6F`: INC (increment register)
- `0x70-0x7F`: ISZ (2-byte increment and skip if zero)
- `0x80-0x8F`: ADD (add register to accumulator)
- `0x90-0x9F`: SUB (subtract register from accumulator)
- `0xA0-0xAF`: LD (load register to accumulator)
- `0xB0-0xBF`: XCH (exchange accumulator and register)
- `0xC0-0xCF`: BBL (branch back and load)
- `0xD0-0xDF`: LDM (load immediate)
- `0xE0-0xEF`: I/O and RAM instructions
- `0xF0-0xFF`: Accumulator group instructions

### Clock Specifications
- Typical: 740 kHz (1.35 us period)
- Range: 500 kHz - 740 kHz
- PHI1/PHI2: Non-overlapping, ~480ns pulse width
- 8 phases per machine cycle, 2 cycles per instruction (most)

---

## Documentation References

### In Repository
- `docs/MCS-4/MCS-4_UsersManual_Feb73.pdf` - Original user manual
- `docs/MCS-4/MCS-4_Assembly_Language_Programming_Manual_Dec73.pdf`
- `docs/MCS-4/i4001-schematic.gif`, `i4002-schematic.gif`, `i4003-schematic.gif`
- `docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf` - MCS-40 manual
- `docs/MCS-40/4040_Datasheet.pdf` - 4040 specifications
- `docs/MCS-40/Intel_Intellec_4_MOD_40_Reference_Schematics.pdf` (78MB)
- `docs/MCS-40/1975_Intel_Data_Catalog.pdf` - Full chip catalog

### External
- [Intel 4004 50th Anniversary](http://www.intelmemory.com/4004/)
- [4004 on Wikichip](https://en.wikichip.org/wiki/intel/mcs-4/4004)

---

## Priority Order for Next Session

1. **4040 CPU** - Core functionality for MCS-40 support
2. **Disassembler** - Needed for debugging
3. **GUI Waveform Viewer** - Visual debugging
4. **4265 PPI** - Key peripheral for I/O
5. **4269 Keyboard/Display** - Human interface
6. **4101 RAM** - MCS-40 memory
7. **4308/4316 ROM** - Larger program storage
8. **Display Drivers** - Visual output
9. **Remaining support chips**

---

## Build and Run

```bash
cd mcs4-emu
cargo build              # Build all crates
cargo test               # Run all 66 tests
cargo run                # Run GUI (stub)
cargo doc --open         # Generate and view documentation
```
