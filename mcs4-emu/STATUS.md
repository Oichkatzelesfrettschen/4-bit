# MCS-4 Emulator Project Status

**Last Updated:** 2025-12-29
**Repository:** https://github.com/Oichkatzelesfrettschen/4-bit

## Project Goal

Build a cycle-accurate, timing-accurate, and electrically-accurate virtual MCS-4 and MCS-40 system with GUI emulator/debugger/programmer, including ALL support chips from the original Intel kits plus era-appropriate peripherals.

## Architecture Decisions

- **Language:** Rust with FPGA synthesis support via mcs4-fpga crate
- **Accuracy Level:** Gate-level with transistor-level stubs for future enhancement
- **Approach:** Cleanroom implementation while auditing OpenCores Verilog for reference
- **Scope:** Complete MCS-4 (4004) and MCS-40 (4040) chip families plus second-sources

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
| **4002-1** | 320-bit RAM (Bank 0 address) | NOT STARTED | Same as 4002, hardwired for CM-RAM0 |
| **4002-2** | 320-bit RAM (Bank 1 address) | NOT STARTED | Same as 4002, hardwired for CM-RAM1 |
| **4003** | 10-bit shift register | STUB | Basic shift logic, needs bus/clock integration |
| **4008** | Address latch (8-bit) | NOT STARTED | For standard memory interface |
| **4009** | I/O buffer/interface | NOT STARTED | Bidirectional bus interface |

**Note:** The 4002-1 and 4002-2 are factory-configured variants of the 4002. The -1 suffix indicates pre-wired response to CM-RAM0, while -2 responds to CM-RAM1. Our 4002 implementation already supports configurable bank_id, so these variants can be emulated with `I4002::new(chip_id, 0)` and `I4002::new(chip_id, 1)`.

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
| **4702A** | 256x8 EPROM (improved) | NOT STARTED | Improved version of 4702 |

### Bus Interface Chips (Schottky Bipolar)

| Chip | Description | Status | Notes |
|------|-------------|--------|-------|
| **3216** | 4-bit parallel bidirectional bus driver | NOT STARTED | Interfaces 4-bit bus to external systems |
| **3226** | 4-bit parallel bidirectional bus driver | NOT STARTED | Inverted enable vs 3216 |

**Note:** The 3216 and 3226 are Schottky bipolar bus drivers used to interface the MCS-4/40 4-bit bus to external 8-bit systems or peripherals. They handle level shifting and bus isolation. The 3216 and 3226 are functionally similar but have inverted chip enable logic.

### Support Logic Chips

| Chip | Description | Status | Notes |
|------|-------------|--------|-------|
| **3205** | 1-of-8 binary decoder | NOT STARTED | High-speed decoder for address decoding |
| **3404** | 6-bit D-type latch | NOT STARTED | Latch for data/address hold |

**Note:** The 3205 is a high-speed 1-of-8 decoder useful for chip select generation. The 3404 provides latching for addresses or data (6-bit, TTL-compatible).

### Standard Memory (via 4289 Interface)

| Chip | Description | Status | Notes |
|------|-------------|--------|-------|
| **2101** | 256x4 static RAM | NOT STARTED | Intel standard SRAM, 350ns |
| **2102** | 1024x1 static RAM | NOT STARTED | Intel 1Kx1 SRAM, 350ns typical |
| **1302** | 2048-bit mask ROM | NOT STARTED | Original designation for 4001-class ROM |

**Note:** The 4289 Standard Memory Interface allows the 4040 to use standard (non-MCS-4) memory chips. The 2101 (256x4) and 2102 (1024x1) were Intel's early static RAM products. The 1302 was the original internal designation before the "4000 family" naming convention was adopted.

### CPU Second-Sources and Clones

| Chip | Manufacturer | Status | Notes |
|------|--------------|--------|-------|
| **INS4004J** | National Semiconductor | NOT STARTED | 4004 clone, ceramic package |
| **INS4004D** | National Semiconductor | NOT STARTED | 4004 clone, CerDIP package |
| **INS4001** | National Semiconductor | NOT STARTED | 4001 ROM clone |
| **INS4002** | National Semiconductor | NOT STARTED | 4002 RAM clone |
| **INS4003** | National Semiconductor | NOT STARTED | 4003 shift register clone |
| **uPD4004** | NEC | NOT STARTED | NEC second-source 4004 |
| **uPD4040** | NEC | NOT STARTED | NEC second-source 4040 |

**Note:** National Semiconductor was Intel's official second-source for the MCS-4 family (mid-1975). NEC also produced second-source versions. These are pin-compatible and functionally identical; no separate implementation needed beyond behavioral verification.

### 74-Series TTL Support

| Category | Chips | Status | Notes |
|----------|-------|--------|-------|
| **Decoders** | 7442, 74154 | NOT STARTED | BCD-to-decimal, 4-to-16 line |
| **Buffers** | 7407, 74125 | NOT STARTED | Open-collector, tri-state |
| **Latches** | 7475, 74175 | NOT STARTED | Quad latches for data hold |
| **Counters** | 7490, 7493 | NOT STARTED | Decade and binary counters |
| **Drivers** | 7447, 7448 | NOT STARTED | 7-segment display drivers |

**Note:** These 74-series TTL chips are commonly used as glue logic in MCS-4/40 systems. Implementation priority is lower than native MCS-4 chips.

### Era-Appropriate Peripherals (1971-1976)

| Category | Chips | Status | Notes |
|----------|-------|--------|-------|
| **Display Drivers** | 7-segment LED drivers | NOT STARTED | Common anode/cathode support |
| **Display Drivers** | Nixie tube drivers | NOT STARTED | High-voltage BCD decoders |
| **Keyboard Interface** | Matrix keyboard scanner | NOT STARTED | Directly or via 4269 |
| **Printers** | Parallel printer interface | NOT STARTED | Centronics-style |
| **Storage** | Paper tape reader | NOT STARTED | Era-appropriate I/O |
| **Storage** | Cassette interface | NOT STARTED | Audio FSK modulation |
| **Communication** | Serial UART | NOT STARTED | RS-232 interface |
| **Expansion** | Bus buffers/transceivers | NOT STARTED | Via 3216/3226 |

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
- **Already supports 4002-1/4002-2 variants via bank_id parameter**

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
- Support for 2101, 2102, 4316, 4702A

#### 4308/4316 ROMs (LOW PRIORITY)
Larger ROM variants (1K/2K x 8-bit).
Similar to 4001 but:
- Larger address space
- Different chip select scheme
- No I/O ports

### Chips to Create from Scratch

#### 3216/3226 Bus Drivers (MEDIUM PRIORITY)
4-bit parallel bidirectional bus drivers for system expansion:
- Direction control
- Output enable logic
- TTL-compatible levels
- Useful for interfacing MCS-4 to external buses

#### 3205 Decoder (LOW PRIORITY)
High-speed 1-of-8 binary decoder:
- 3-bit binary input, 8 outputs
- Active-low outputs
- Enable inputs for cascading
- Used for chip select generation

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

#### 4702A EPROM (LOW PRIORITY)
256 x 8-bit UV-erasable PROM:
- Same pinout as 4001 ROM (for development)
- Programming voltage support (for completeness)
- Read-only mode for normal operation

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

### External Resources
- [Intel 4004 50th Anniversary](http://www.4004.com/)
- [4004 on Wikichip](https://en.wikichip.org/wiki/intel/mcs-4/4004)
- [MCS-40 on Wikichip](https://en.wikichip.org/wiki/intel/mcs-40)
- [MCS-40 Users Manual (PDF)](http://bitsavers.informatik.uni-stuttgart.de/components/intel/MCS40/MCS-40_Users_Manual_Nov74.pdf)
- [Intel 3205 Datasheet](https://www.cpu-galaxy.at/cpu/ram%20rom%20eprom/other_intel_chips/other_intel-Dateien/P3205_Datasheet.pdf)
- [Intel 3216 Datasheet (Archive.org)](https://archive.org/details/intel3216parallelbidirectionalbusdriver)
- [Intel 2102 Static RAM Info](https://w140.com/tekwiki/wiki/Intel_2102)
- [OpenCores MCS-4 Project](https://opencores.org/projects/mcs-4)

---

## Priority Order for Next Session

### Tier 1 - Core Functionality
1. **4040 CPU** - Core functionality for MCS-40 support
2. **Disassembler** - Needed for debugging
3. **GUI Waveform Viewer** - Visual debugging

### Tier 2 - Key Peripherals
4. **4265 PPI** - Key peripheral for I/O
5. **4269 Keyboard/Display** - Human interface
6. **3216/3226 Bus Drivers** - System expansion

### Tier 3 - Memory Expansion
7. **4101 RAM** - MCS-40 memory
8. **4289 Standard Memory Interface** - Enables 2101/2102/4316
9. **4308/4316 ROM** - Larger program storage
10. **4702A EPROM** - Development memory

### Tier 4 - Support Logic
11. **3205 Decoder** - Address decoding
12. **3404 Latch** - Data/address hold
13. **74-series TTL** - Glue logic

### Tier 5 - Display and I/O
14. **Display Drivers** - Visual output
15. **Keyboard Scanner** - User input
16. **Serial UART** - Communication

---

## Chip Naming History

Intel's original chip-naming scheme used a four-digit number:
- First digit: Process technology (1 = PMOS, 2 = NMOS, 3 = Bipolar, 4 = MCS-4 bus)
- Second digit: Generic function (0 = ROM, 1 = RAM, 2 = shift register, etc.)
- Last two digits: Sequential number

Under this scheme, the MCS-4 chips would have been:
- **1302** -> became **4001** (2048-bit ROM)
- **1105** -> became **4002** (320-bit RAM)
- **1507** -> became **4003** (shift register)
- **1202** -> became **4004** (CPU)

Federico Faggin renamed them to the "4000 family" to emphasize they formed a coherent chipset.

---

## Build and Run

```bash
cd mcs4-emu
cargo build              # Build all crates
cargo test               # Run all 66 tests
cargo run                # Run GUI (stub)
cargo doc --open         # Generate and view documentation
```
