# MCS-4 Emulator Project Status

**Last Updated:** 2025-12-31
**Repository:** https://github.com/Oichkatzelesfrettschen/4-bit

## Session Log
- 2025-12-31T05:37:33Z: Switched repository to rustup nightly; added clippy lint gates (warnings-as-errors) and 4-bit data handling deps (modular-bitfield, bitflags, tinyvec, smallvec). 4040 stack/interrupt/register bank scaffolding landed; decoder stubs and core wiring added.

- 2025-12-31T05:00:42Z: Code review gate established per Oaich standards; proceeding with 4040 scaffolding, disasm core, waveform hooks under warnings-as-errors and >=90% coverage.
- 2025-12-31T05:01:24Z: Senior Code Review Specialist mode engaged; applying OWASP, performance, testing, maintainability gates to Next 10 Tasks before implementation.
- 2025-12-31: Initiated Tier 1 tasks (4040 CPU design, Disassembler scaffolding, GUI Waveform capture hooks). Updating STATUS.md per milestone.
- 2025-12-31: 4040 CPU scaffolding marked started; defining register bank model and stack depth invariants.

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

#### 4040 CPU (HIGH PRIORITY) - DETAILED SPECIFICATION

The 4040 is a backward-compatible extension of the 4004, adding 14 new instructions (total: 60), expanded registers, deeper stack, and interrupt support.

**Architectural Differences from 4004:**

| Feature | 4004 | 4040 |
|---------|------|------|
| Index Registers | 16 (R0-R15) | 24 (R0-R23, two banks of 8) |
| Stack Depth | 3 levels | 7 levels |
| Instructions | 46 | 60 (+14 new) |
| Interrupts | None | Single-level, vectors to 0x003 |
| Register Banks | 1 | 2 (switchable via DB0/DB1) |
| Halt Mode | None | HLT instruction + STP pin |

**New 4040 Instructions to Implement:**

| Opcode | Mnemonic | Description |
|--------|----------|-------------|
| `0x01` | **HLT** | Halt CPU execution (enters low-power mode) |
| `0x02` | **BBS** | Branch Back from interrupt, restore SRC register |
| `0x03` | **LCR** | Load Command RAM (read ROM into RAM) |
| `0x04` | **OR4** | OR accumulator with register R4 |
| `0x05` | **OR5** | OR accumulator with register R5 |
| `0x06` | **AN6** | AND accumulator with register R6 |
| `0x07` | **AN7** | AND accumulator with register R7 |
| `0x08` | **DB0** | Designate register Bank 0 (R0-R7 primary) |
| `0x09` | **DB1** | Designate register Bank 1 (R0-R7 become R16-R23) |
| `0x0A` | **SB0** | Select RAM Bank 0 |
| `0x0B` | **SB1** | Select RAM Bank 1 |
| `0x0C` | **EIN** | Enable Interrupts |
| `0x0D` | **DIN** | Disable Interrupts |
| `0x0E` | **RPM** | Read Program Memory (ROM byte to accumulator) |

**Interrupt Handling Specification:**

1. When INT pin asserted and interrupts enabled (via EIN):
   - Complete current instruction
   - Push PC to stack (uses 1 of 7 levels)
   - Save SRC register to SRC Save Register
   - Disable interrupts automatically
   - Vector to address 0x003
2. Interrupt service routine executes
3. **BBS** instruction restores SRC and returns (like BBL but for interrupts)
4. Re-enable interrupts with **EIN** if desired

**Register Bank Switching:**

```
Bank 0 (DB0):        Bank 1 (DB1):
R0-R7   = Primary    R0-R7   = Shadow (R16-R23)
R8-R15  = Always accessible
```

**Files to Create/Modify:**

```
crates/mcs4-chips/src/i4040/
├── mod.rs                 # Main CPU struct, tick(), execute()
├── registers.rs           # 24-register file with bank switching
├── alu.rs                 # Extended ALU (OR4/5, AN6/7 operations)
├── instruction_decode.rs  # Extended decoder (60 instructions)
├── interrupt.rs           # Interrupt controller state machine
└── tests.rs               # Unit tests for new instructions
```

**Implementation Steps:**

1. Copy i4004/ as base, rename to I4040
2. Extend Registers:
   - `regs: [u8; 24]` (was 16)
   - `bank: u8` (0 or 1)
   - `get_r()` / `set_r()` apply bank offset for R0-R7
3. Extend stack from 3 to 7 levels
4. Add interrupt controller:
   - `int_enabled: bool`
   - `int_pending: bool`
   - `src_save: u8`
   - `halted: bool`
5. Extend InstructionDecoder with new opcodes
6. Implement new instructions in execute()
7. Add INT pin handling in tick()

**Test Cases Required:**

- [ ] All 46 original 4004 instructions still work
- [ ] HLT stops execution, resumes on external signal
- [ ] Register bank switching (DB0/DB1) correctly maps R0-R7
- [ ] Stack handles 7 nested calls
- [ ] Interrupt vectors to 0x003
- [ ] BBS restores SRC and returns correctly
- [ ] EIN/DIN enable/disable interrupts
- [ ] OR4/OR5/AN6/AN7 logical operations
- [ ] SB0/SB1 select RAM banks
- [ ] RPM reads from ROM into accumulator
- [ ] Backward compatibility with 4004 programs

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

### Waveform Viewer (crates/mcs4-gui/) - DETAILED SPECIFICATION

**Current State:** Basic egui scaffold exists with Run/Stop/Step/Reset buttons and placeholder panels.

**Architecture Design:**

```
┌─────────────────────────────────────────────────────────────────────┐
│ Toolbar: [Run] [Stop] [Step] [Reset] | Speed: [____] | Zoom: [+][-] │
├─────────────────┬───────────────────────────────────────────────────┤
│ CPU State       │                   Central Panel                   │
│ ─────────────   │ ┌───────────────────────────────────────────────┐ │
│ PC:  0x000      │ │           Waveform Display                    │ │
│ ACC: 0x0        │ │  PHI1  ─┐_┌─┐_┌─┐_┌─┐_┌─┐_┌─┐_┌─┐_┌─        │ │
│ CY:  0          │ │  PHI2  _┌─┘_┌─┘_┌─┘_┌─┘_┌─┘_┌─┘_┌─┘_        │ │
│                 │ │  SYNC  ─┐___┌───┐___┌───┐___┌───┐___        │ │
│ Registers       │ │  D0-D3 ═╪═══╪═══╪═══╪═══╪═══╪═══╪═══        │ │
│ ─────────────   │ │  CM    ─┐___┌───────┐___┌───────┐___        │ │
│ R0-R1: 0x00     │ │         ↑                                     │ │
│ R2-R3: 0x00     │ │      Cursor                                   │ │
│ ...             │ │         Time: 1.35µs  Cycle: A1               │ │
│                 │ └───────────────────────────────────────────────┘ │
│ Stack           │ ┌───────────────────────────────────────────────┐ │
│ ─────────────   │ │           Disassembly View                    │ │
│ [0]: 0x000      │ │  000: D5      LDM  5                          │ │
│ [1]: ---        │ │▶ 001: 20 42   FIM  P0, 0x42                   │ │
│ [2]: ---        │ │  003: 21      SRC  P0                         │ │
│                 │ │  004: E0      WRM                             │ │
├─────────────────┤ └───────────────────────────────────────────────┘ │
│ Memory          │ ┌───────────────────────────────────────────────┐ │
│ ─────────────   │ │           Memory Hex View                     │ │
│ [ROM] [RAM]     │ │  000: D5 20 42 21 E0 00 00 00 00 00 00 00 ... │ │
│ 000: D5 20 42   │ │  010: 00 00 00 00 00 00 00 00 00 00 00 00 ... │ │
│ 003: 21 E0 00   │ └───────────────────────────────────────────────┘ │
└─────────────────┴───────────────────────────────────────────────────┘
```

**Signal Capture System:**

```rust
/// Signal trace buffer for waveform display
pub struct SignalTrace {
    /// Timestamp in simulation ticks
    timestamps: Vec<u64>,
    /// PHI1 clock states
    phi1: Vec<bool>,
    /// PHI2 clock states
    phi2: Vec<bool>,
    /// SYNC signal states
    sync: Vec<bool>,
    /// 4-bit data bus values
    data_bus: Vec<u8>,
    /// CM-ROM select line (4-bit)
    cm_rom: Vec<u8>,
    /// CM-RAM select lines (2-bit)
    cm_ram: Vec<u8>,
    /// Current bus phase (A1-X3)
    phase: Vec<BusCycle>,
}

impl SignalTrace {
    /// Record current state (called each tick)
    pub fn capture(&mut self, tick: u64, bus: &DataBus, ctrl: &ControlSignals, phase: BusCycle);

    /// Get signal state at timestamp
    pub fn get_at(&self, tick: u64) -> SignalState;

    /// Clear buffer (keep last N samples)
    pub fn truncate(&mut self, keep_samples: usize);
}
```

**Waveform Renderer Component:**

```rust
pub struct WaveformPanel {
    /// Signal trace data source
    trace: Arc<RwLock<SignalTrace>>,
    /// Horizontal zoom level (samples per pixel)
    zoom: f32,
    /// Scroll offset (start sample)
    scroll_x: u64,
    /// Cursor position (sample index)
    cursor: Option<u64>,
    /// Which signals to display
    visible_signals: SignalVisibility,
    /// Signal height in pixels
    signal_height: f32,
}

impl WaveformPanel {
    /// Render waveforms to egui UI
    pub fn show(&mut self, ui: &mut egui::Ui);

    /// Handle zoom gestures (scroll wheel)
    fn handle_zoom(&mut self, delta: f32);

    /// Handle pan gestures (drag)
    fn handle_pan(&mut self, delta: f32);

    /// Render single digital signal
    fn draw_digital(&self, painter: &egui::Painter, signal: &[bool], y: f32, color: Color32);

    /// Render 4-bit bus as hex values
    fn draw_bus(&self, painter: &egui::Painter, signal: &[u8], y: f32, color: Color32);
}
```

**Files to Create/Modify:**

```
crates/mcs4-gui/src/
├── main.rs           # Application entry, window setup
├── app.rs            # EmulatorApp state and main update loop
├── panels/
│   ├── mod.rs        # Panel exports
│   ├── cpu_state.rs  # CPU registers, flags display
│   ├── memory.rs     # ROM/RAM hex viewer with edit
│   ├── waveform.rs   # Signal waveform display (NEW)
│   ├── disasm.rs     # Disassembly view (NEW)
│   └── controls.rs   # Toolbar buttons, speed control
├── signal_trace.rs   # Signal capture buffer (NEW)
└── style.rs          # Theme and colors
```

**Implementation Steps:**

1. Create `SignalTrace` struct in signal_trace.rs
2. Hook trace capture into system tick loop
3. Create `WaveformPanel` in panels/waveform.rs
4. Implement digital signal rendering with egui painter
5. Add zoom/pan with mouse scroll and drag
6. Add cursor for inspecting signal values at time
7. Integrate with main app layout

**Test Cases:**

- [ ] Signal capture records PHI1/PHI2/SYNC correctly
- [ ] Bus values captured at each phase
- [ ] Zoom in/out works smoothly
- [ ] Pan with drag gesture
- [ ] Cursor shows signal values at position
- [ ] Performance: 60fps with 10K+ samples

---

### Disassembler - DETAILED SPECIFICATION

**Current State:** Not started. Instruction decoder exists in i4004 but needs disassembly output format.

**Disassembler Output Format:**

```
; MCS-4 Disassembly
; File: program.bin
; Length: 256 bytes

        ORG     000H

L_000:  LDM     5           ; Load 5 into accumulator
        FIM     P0, 42H     ; Load 0x42 into register pair 0
        SRC     P0          ; Select RAM address from P0
        WRM                 ; Write accumulator to RAM
        JUN     L_010       ; Jump to L_010

L_010:  NOP                 ; No operation
        ISZ     R5, L_010   ; Increment R5, loop if not zero
        BBL     0           ; Return with 0
```

**Core Disassembler API:**

```rust
/// Disassembler for MCS-4 (4004) and MCS-40 (4040) instruction sets
pub struct Disassembler {
    /// CPU type affects instruction decode
    cpu_type: CpuType,
    /// Symbol table (address -> label)
    symbols: HashMap<u16, String>,
    /// Comments (address -> comment)
    comments: HashMap<u16, String>,
}

pub enum CpuType {
    I4004,  // 46 instructions
    I4040,  // 60 instructions
}

impl Disassembler {
    /// Disassemble single instruction at address
    /// Returns (mnemonic, operands, length, cycles)
    pub fn disasm_one(&self, rom: &[u8], addr: u16) -> DisasmLine;

    /// Disassemble ROM range
    pub fn disasm_range(&self, rom: &[u8], start: u16, end: u16) -> Vec<DisasmLine>;

    /// Disassemble entire ROM
    pub fn disasm_all(&self, rom: &[u8]) -> Vec<DisasmLine>;

    /// Format as assembly listing
    pub fn format_listing(&self, lines: &[DisasmLine]) -> String;

    /// Add symbol at address
    pub fn add_symbol(&mut self, addr: u16, name: &str);

    /// Auto-generate labels for jump targets
    pub fn auto_label(&mut self, rom: &[u8]);
}

pub struct DisasmLine {
    pub address: u16,
    pub bytes: Vec<u8>,
    pub mnemonic: String,
    pub operands: String,
    pub comment: Option<String>,
    pub is_jump_target: bool,
}
```

**Instruction Format Strings:**

| Instruction | Format |
|-------------|--------|
| NOP | `NOP` |
| LDM n | `LDM     {n}` |
| LD r | `LD      R{r}` |
| FIM Pn, data | `FIM     P{n}, {data:02X}H` |
| SRC Pn | `SRC     P{n}` |
| JUN addr | `JUN     L_{addr:03X}` |
| JMS addr | `JMS     L_{addr:03X}` |
| JCN cond, addr | `JCN     {cond}, L_{addr:03X}` |
| ISZ r, addr | `ISZ     R{r}, L_{addr:03X}` |
| BBL n | `BBL     {n}` |

**Condition Code Mnemonics (JCN):**

| Code | Mnemonic | Meaning |
|------|----------|---------|
| 0x1 | T | Test pin = 1 |
| 0x2 | C | Carry = 1 |
| 0x4 | Z | Accumulator = 0 |
| 0x9 | TN | Test pin = 0 (NOT) |
| 0xA | CN | Carry = 0 (NOT) |
| 0xC | ZN | Accumulator != 0 (NOT) |

**Files to Create:**

```
crates/mcs4-chips/src/
├── disasm.rs          # Core disassembler
└── disasm/
    ├── mod.rs         # Module exports
    ├── format.rs      # Output formatting
    ├── symbols.rs     # Symbol table management
    └── analysis.rs    # Control flow analysis (future)
```

**Implementation Steps:**

1. Create `DisasmLine` struct
2. Implement `disasm_one()` using existing InstructionDecoder
3. Add operand formatting for each instruction type
4. Implement `auto_label()` to find jump targets
5. Create listing formatter
6. Add to mcs4-chips public API
7. Integrate into GUI disasm panel

**Test Cases:**

- [ ] All 46 4004 instructions disassemble correctly
- [ ] All 14 new 4040 instructions disassemble correctly
- [ ] Two-byte instructions show full opcode
- [ ] Jump targets get auto-labeled
- [ ] Round-trip: disasm output can be assembled back
- [ ] Handles invalid opcodes gracefully

---

### Debugger UI - DETAILED SPECIFICATION

**Current State:** Basic Run/Stop/Step/Reset buttons exist. Needs full debugger integration.

**Feature Requirements:**

1. **Register Panel** (partially exists)
   - All 16 registers (R0-R15) or 24 (4040)
   - Accumulator and carry flag
   - Register pair view (P0-P7)
   - Edit registers by clicking

2. **Memory Panel**
   - ROM viewer (read-only hex)
   - RAM viewer (editable hex)
   - Status registers view
   - Output port states
   - Go-to address

3. **Disassembly Panel** (needs disassembler)
   - Current instruction highlighted
   - Click to set breakpoint
   - Show labels from symbol table
   - Follow jumps

4. **Stack Panel**
   - All stack levels (3 for 4004, 7 for 4040)
   - Highlight current level
   - Show return addresses

5. **Breakpoint Manager**
   - List of active breakpoints
   - Enable/disable individual
   - Conditional breakpoints (future)
   - Break on PC, memory access, register change

6. **Execution Control**
   - Run / Stop
   - Step (1 phase, 1 cycle, 1 instruction)
   - Run to cursor
   - Run N cycles
   - Speed control (1Hz to max)

7. **Waveform Integration**
   - Synchronized with execution
   - Cursor follows PC
   - Click waveform to seek

**Keyboard Shortcuts:**

| Key | Action |
|-----|--------|
| F5 | Run / Continue |
| F6 | Stop |
| F7 | Step Into (1 instruction) |
| F8 | Step Over (skip JMS) |
| F9 | Toggle Breakpoint |
| F10 | Step Cycle (8 phases) |
| Ctrl+G | Go to Address |

**Files to Create/Modify:**

```
crates/mcs4-gui/src/
├── app.rs            # Main app with debugger state
├── debugger.rs       # Debugger controller (NEW)
├── breakpoints.rs    # Breakpoint management (NEW)
├── panels/
│   ├── cpu_state.rs  # Enhanced register view
│   ├── memory.rs     # Enhanced with edit support
│   ├── disasm.rs     # Disassembly panel (NEW)
│   ├── stack.rs      # Stack view (NEW)
│   └── breakpoints.rs # Breakpoint list panel (NEW)
└── shortcuts.rs      # Keyboard handler (NEW)
```

**Debugger State Machine:**

```rust
pub enum DebugState {
    /// Stopped, waiting for user action
    Stopped,
    /// Running freely
    Running,
    /// Single-stepping (phase level)
    SteppingPhase,
    /// Single-stepping (instruction level)
    SteppingInstruction,
    /// Running until breakpoint
    RunningToBreakpoint,
    /// Running to specific address
    RunningToAddress(u16),
}

pub struct Debugger {
    state: DebugState,
    breakpoints: Vec<Breakpoint>,
    step_over_return: Option<u16>,
    execution_speed: ExecutionSpeed,
}
```

**Implementation Steps:**

1. Create debugger controller with state machine
2. Implement breakpoint system in mcs4-system
3. Add step modes (phase, cycle, instruction)
4. Create enhanced register panel with edit
5. Create memory panel with hex editor
6. Integrate disassembler into disasm panel
7. Add stack display panel
8. Implement keyboard shortcuts
9. Add execution speed control
10. Synchronize waveform with execution

**Test Cases:**

- [ ] Step instruction works correctly
- [ ] Breakpoints halt at correct address
- [ ] Run/Stop toggles execution
- [ ] Register edits take effect immediately
- [ ] Memory view updates during execution
- [ ] Disassembly highlights current instruction
- [ ] Stack shows correct depth and values
- [ ] Keyboard shortcuts work

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

## Next 10 Tasks (Execution Plan)

Quality Gates Applied: warnings-as-errors; clippy clean; tests >=90% coverage; shellcheck for scripts (if any).
1) 4040 registers.rs: implement 24-reg file + bank switching [DONE]
   - Review gates: unit tests for get/set, bank switch; no unwrap; clippy clean; docs on mapping; property tests for index bounds.
   - Review gates: unit tests for get/set, bank switch; no unwrap; clippy clean; docs on mapping; property tests for index bounds.
2) 4040 stack: expand to 7 levels with push/pop invariants [STARTED]
3) 4040 interrupt.rs: EIN/DIN state, INT vector to 0x003, BBS restore [PLANNED]
4) 4040 instruction_decode.rs: add 14 new opcodes [PLANNED]
5) 4040 mod.rs: tick() integrates INT, HLT, bank ops [PLANNED]
6) Disassembler core: disasm_one/range + format_listing [PLANNED]
7) Disassembler 4040 support: operand formatting for new ops [PLANNED]
8) SignalTrace buffer: implement and hook into system tick [PLANNED]
9) WaveformPanel renderer: digital signals + 4-bit bus hex [PLANNED]
10) Tests: unit for 4040 ops, interrupts, stack; integration baseline [PLANNED]

## Priority Order for Next Session

### Tier 1 - Core Functionality (Granular Checklist)
- 4040 CPU
  - [x] Create crates/mcs4-chips/src/i4040/ module scaffolding (mod.rs, registers.rs, instruction_decode.rs, interrupt.rs)
  - [ ] Implement register bank switching (R0-R7 mapped to R0-R7 or R16-R23)
    - Design: bank: u8 (0/1); idx_map(r in 0..7) => r + bank*16; R8-R15 fixed
  - [ ] Extend stack to 7 levels with push/pop invariants and overflow tests
  - [ ] INT pin handling: vector 0x003, auto-disable, SRC save/restore via BBS
  - [ ] Implement new opcodes (HLT,BBS,LCR,OR4,OR5,AN6,AN7,DB0,DB1,SB0,SB1,EIN,DIN,RPM)
  - [ ] Backward-compat with 4004: full ISA regression
  - [ ] Unit tests per instruction + interrupt/stack edge cases
- Disassembler
  - [ ] Disasm core (disasm_one, disasm_range, format_listing)
  - [ ] Operand formatting and auto-labeling
  - [ ] 4040 opcodes support and tests
- GUI Waveform Viewer
  - [ ] SignalTrace buffer implementation and capture hook in tick()
  - [ ] WaveformPanel renderer (digital lines + 4-bit bus hex)
  - [ ] Zoom/pan/cursor interactions and performance test (10k+ samples)

### Tier 2 - Key Peripherals
- 4265 PPI
  - [ ] Define register set and bus protocol
  - [ ] Mode control and unit tests
- 4269 Keyboard/Display
  - [ ] Matrix scan algorithm and display driver hooks
- 3216/3226 Bus Drivers
  - [ ] Direction/OE modeling and TTL level mapping

### Tier 3 - Memory Expansion
- 4101 RAM
  - [ ] Addressing, R/W timing, CE logic and tests
- 4289 Interface
  - [ ] Address/data muxing and standard memory timings
- 4308/4316 ROM
  - [ ] Larger ROM support, CS scheme

### Tier 4 - Support Logic
- 3205 Decoder, 3404 Latch, 74-series TTL
  - [ ] Minimal behavioral models and validation

### Tier 5 - Display and I/O
- Display drivers, Keyboard scanner, Serial UART
  - [ ] Behavioral models and integration tests

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

## Risk & Quality Gates

## Pre-Commit Quality Review Checklist (Oaich)

Reviewer Mode: Senior Code Review Specialist active as of 2025-12-31T05:01:32Z; applying OWASP, performance, testing (>=90%), maintainability, documentation gates to all Next 10 Tasks and subsequent commits.
- Security: OWASP pass, input validation for GUI loads, no secrets, proper error handling.
- Performance: O(n) paths, avoid N+1, no unnecessary cloning; benchmark hot loops.
- Testing: >=90% coverage backend; unit + integration; property tests for bus protocol; no flakiness.
- Maintainability: Idiomatic Rust, SRP, cyclomatic < 10, DRY; boundaries clear.
- Documentation: Public APIs documented, complex algorithms explained, README accurate.

- Security: No unsafe Rust; no secrets; input validation on GUI file loads.
- Performance: Event-driven sim; avoid unnecessary clone; benchmark critical paths (criterion).
- Testing: Maintain >=90% coverage; unit + integration; property tests for bus protocol.
- Documentation: Update ARCHITECTURE.md upon 4040 completion; disasm API docs.

## Build and Run

```bash
cd mcs4-emu
cargo build              # Build all crates
cargo test               # Run all 66 tests
cargo run                # Run GUI (stub)
cargo doc --open         # Generate and view documentation
```
