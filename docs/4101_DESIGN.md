# Intel 4101 256x4 Static RAM - Architecture Design

**Date**: 2026-01-29  
**Status**: Design Phase  
**Component**: MCS-40 Static RAM  
**Priority**: Phase 3 Critical Path  

## 1. OVERVIEW

The Intel 4101 is a 1024-bit static RAM organized as **256 words × 4 bits**,
designed for the MCS-40 microcomputer system. It serves as the primary data
memory when used with the 4289 Standard Memory Interface.

### Key Characteristics
- **Capacity**: 256 × 4 bits (1024 bits total)
- **Organization**: Linear address space (A0-A7 for 256 unique addresses)
- **Data Width**: 4 bits per word (Q3-Q0)
- **Technology**: NMOS static RAM (6-transistor cells)
- **Interface**: 4289 multiplexed address/data bus
- **Speed**: Asynchronous read/write (no clock required)

## 2. MEMORY ORGANIZATION

### Physical Layout

```
Address Space: 0x00 to 0xFF (256 locations)

Each location stores 4 bits:
  [Q3] [Q2] [Q1] [Q0]

Total silicon: ~150-200 transistors (NMOS static cells)
  - 256 × 6 transistors per cell = 1536 transistors for array
  - ~300-400 transistors for peripheral logic (decoders, drivers)
  - Total: ~1800-2000 transistors estimated
```

### Bit Organization

```
Bit 3 (MSB): Q3
Bit 2:       Q2
Bit 1:       Q1
Bit 0 (LSB): Q0

In 4-bit MCS-4 system: maps directly to CPU accumulator
```

### Bank Organization (Future 4040 Extension)

For enhanced MCS-40 systems, the 4101 can be used with multiple banks:
- Single 4101: 256 locations in one bank
- Dual 4101: 512 locations (via chip select demultiplexing)
- Quad 4101: 1024 locations (via 2-bit chip select)

**Current Design**: Single 4101 (256 locations, single chip select)

## 3. MEMORY CELL DESIGN

### 6-Transistor NMOS Static Cell

```
    +--[P1]--+--[P2]--+
    |        |        |
   [N1]     [N2]     [N3]      [N4]
    |        |        |
    +---QA--+--QB----+
        |        |
       Word    Cross-coupled
       Line    inverters
```

**Cell Structure**:
- Two cross-coupled NMOS inverters (N1-P1, N2-P2) = latch
- Two access transistors (N3, N4) controlled by word line
- Bit line and bit line complement for differential sensing

**Cell Stability**:
- Static holding: no refresh required (unlike DRAM)
- Hold time: indefinite while powered
- Read: non-destructive (data retained)
- Write: destructive and reconstructive (new data written, old discarded)

## 4. ADDRESS DECODING

### Address Inputs (8 bits)

```
A7 (MSB) -- Decoder Row 7  (128 locations)
A6       -- Decoder Row 6  (64 locations)
A5       -- Decoder Row 5  (32 locations)
A4       -- Decoder Row 4  (16 locations)
A3       -- Decoder Row 3  (8 locations)
A2       -- Decoder Row 2  (4 locations)
A1       -- Decoder Row 1  (2 locations)
A0 (LSB) -- Decoder Row 0  (1 location)
```

### Decoder Architecture

**3-of-8 Decoder** (for efficiency):
- Divides 8-bit address into groups
- Uses 3-input NAND gates for selection
- Reduces area vs. simple 256-input NOR

**Hierarchical Decoding**:
```
A[7:5] -> Primary decoder (8 rows)  (selects one of 8 row groups)
A[4:2] -> Secondary decoder (8 cols) (selects one of 8 column groups)
A[1:0] -> Tertiary encoder (4 cells) (selects one of 4 cells)

Total: 8 × 8 × 4 = 256 locations
```

### Word Line Structure

```
Address 0x00: Word Line 0 -> Selects Row/Col (0, 0) -> 4 bits
Address 0x01: Word Line 1 -> Selects Row/Col (0, 1) -> 4 bits
...
Address 0xFF: Word Line 255 -> Selects Row/Col (255, 0) -> 4 bits
```

**Word Line Distribution**:
- One word line activated per address
- All 6 transistors in selected cell conduct
- Unselected cells: word line = 0 (transistors off), cross-coupled inverters hold state

## 5. READ OPERATION

### Read Cycle Timing

```
Address Valid
   |
   |--+----------+----------+
      |          |          |
    A[0:7]   Propagate   Sense    Output
    Valid    (10ns)      (15ns)   Valid
             |          |          |
             +--T1--+---T2---+----T3----+
                    |
                Word Line = 1 (20ns)
                    |
                Bit Lines    Bit Line    Output
                Precharge    Develop     Drivers
                (5ns)        (20ns)      Active
                             |
                        Data Hold
                        while A/CS
                        valid
```

### Read Sequence

1. **Address Setup**: A[0:7] lines valid for >50ns
2. **Chip Select**: CS = 1 (selects this 4101)
3. **Word Line Activation**: Decoder selects word line based on A[0:7]
4. **Bit Line Development**: Storage node voltage developed on Q/Q_bar
5. **Output Drivers**: Q[3:0] driven with sensed data
6. **Data Valid**: Output stable for >100ns (setup for CPU read)

### Timing Constraints

```
tAA (Address to Output):  200ns max
  - Decoder delay: 50ns
  - Bit line develop: 60ns
  - Output driver: 90ns

tACS (Address/CS to Output): 200ns max

tOH (Output Hold): 20ns min (after address change)
  - Ensures no race conditions in multiplexed bus

tOZ (Output Disable Time): 50ns max (after CS goes low)
  - Time to tri-state output drivers
```

## 6. WRITE OPERATION

### Write Cycle Timing

```
Address Valid
   |
   |--+----------+----------+------+
      |          |          |      |
    A[0:7]   Propagate   Data   Write    Data
    Valid    (10ns)      Valid  Pulse    Held
             |          |      (30ns)    |
             +--T1--+---T2--+---T3---+--T4--+
                    |
                Word Line = 1
                    |
                Data on Bus  Write Current  New Data
                (20ns setup) Flows to Cell  Stored
                             (30ns pulse)
                                      |
                                   SR Latch
                                   Flips
```

### Write Sequence

1. **Address Setup**: A[0:7] valid for >50ns before data
2. **Chip Select**: CS = 1
3. **Data Setup**: D[0:7] (or via bus) valid for >50ns
4. **Write Enable**: WE = 1 (internal to 4289 for MCS-40)
5. **Word Line Activation**: Decoder selects word line
6. **Write Current**: Write drivers push current through selected cells
7. **Latch Flip**: Cross-coupled inverters flip to new state
8. **Data Hold**: D[0:7] held for >50ns after write pulse

### Timing Constraints

```
tWP (Write Pulse Width): 30ns min, 100ns max
  - Too short: cell doesn't flip
  - Too long: power waste, unnecessary

tDS (Data Setup before WE): 50ns min
  - Ensures data stable before write current

tDH (Data Hold after WE): 50ns min
  - Ensures latch captures correct data

tWC (Write Cycle Time): 300ns max
  - Total time for address→data→write→settle
```

## 7. BUS INTERFACE

### Signal Connections (via 4289 Interface)

```
From 4289:
  A[7:0]      -- Address inputs
  D[3:0]      -- Data inputs (write)
  CS          -- Chip select (active high)
  WE          -- Write enable (active high)
  OE          -- Output enable (active high) [optional]

To Bus:
  Q[3:0]      -- Data outputs (read)
```

### Bus Protocol Integration

The 4101 operates asynchronously. The 4289 provides timing:

```
MCS-40 Bus Cycle (8 phases: A1-A3, M1-M2, X1-X3):

A1: Address setup (4289 drives A[7:0])
    4101: Word line decoder activates

A2: Address hold
    4101: Bit lines developing

A3: Address valid
    4101: Word line fully enabled

M1: Data propagate (for read)
    4101: Output drivers active

M2: Data valid
    4101: Q[3:0] stable on bus

X1: Data capture (CPU reads or 4289 receives)
    4101: Q[3:0] maintained

X2: Data hold
    4101: Still driving Q[3:0]

X3: Data latch (for write, 4289 latches result)
    4101: Q[3:0] remain valid
    If write: WE = 1, write current flows
```

### Tri-State Output Drivers

```
Q[3:0] drivers:

  If CS=1 and OE=1:
    Drive with sensed data (active output)
  Else:
    High impedance (tri-state)
    
  Rise/Fall time: 20ns typical
  Output impedance: ~100 ohms
  Sink/Source current: 10mA typical
```

## 8. CONTROL SIGNALS

### Chip Select (CS)

```
CS = 1: 4101 enabled
  - Address decoder active
  - Read or write operation proceeds
  - Output drivers driven (if OE=1)

CS = 0: 4101 disabled
  - Address decoder inactive (word lines off)
  - Write operations inhibited
  - Output drivers tri-stated
  - Memory holds state (static)

Transition timing:
  CS low→high: 50ns for output to enable
  CS high→low: 20ns for output to disable
```

### Write Enable (WE)

```
WE = 1 (with CS=1): Write mode
  - Write drivers activated
  - Data on bus captured into selected cell
  - Data latched after 30ns write pulse

WE = 0 (with CS=1): Read mode
  - Write drivers disabled
  - Output drivers enabled (if OE=1)
  - Data from selected cell driven to bus
```

### Output Enable (OE) [Optional, for tri-state control]

```
OE = 1 (with CS=1): Output drivers active
  - Q[3:0] driven with data

OE = 0: Output drivers tri-stated
  - Useful for multiple RAM banks on same bus
  - Allows time-multiplexing of memory devices
```

## 9. IMPLEMENTATION ARCHITECTURE

### Structural Organization

```
┌─────────────────────────────────────────┐
│         4101 Memory Device              │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────────────────────────────┐  │
│  │   Address Decoder (8-bit)        │  │
│  │   - 256 word line outputs        │  │
│  │   - Selects 1 of 256 locations   │  │
│  └──────────────────────────────────┘  │
│           ↑                             │
│        A[7:0]                           │
│                                         │
│  ┌──────────────────────────────────┐  │
│  │   Memory Array (256×4)           │  │
│  │   - 256 word lines (from decoder)│  │
│  │   - 4 bit lines (BL[3:0])        │  │
│  │   - 1024 6T cells                │  │
│  │   - Cross-coupled inverters      │  │
│  └──────────────────────────────────┘  │
│           ↓  ↑                          │
│          Q   D                          │
│       (read) (write)                    │
│                                         │
│  ┌──────────────────────────────────┐  │
│  │   Read Drivers & Sense Amps      │  │
│  │   - Sense bit line voltage       │  │
│  │   - Amplify to rail voltage      │  │
│  │   - Tri-state output buffers     │  │
│  └──────────────────────────────────┘  │
│           ↓                             │
│        Q[3:0]                           │
│                                         │
│  ┌──────────────────────────────────┐  │
│  │   Write Drivers                  │  │
│  │   - Push/pull transistors        │  │
│  │   - Drive bit lines low/high     │  │
│  │   - Flip cross-coupled inverters │  │
│  └──────────────────────────────────┘  │
│           ↑                             │
│        D[3:0] + WE                      │
│                                         │
│  ┌──────────────────────────────────┐  │
│  │   Control Logic                  │  │
│  │   - CS (chip select)             │  │
│  │   - WE (write enable)            │  │
│  │   - OE (output enable, opt.)     │  │
│  └──────────────────────────────────┘  │
│                                         │
└─────────────────────────────────────────┘
```

### Behavioral Model Structure (Rust)

```rust
pub struct I4101 {
    memory: [u8; 256],           // 256 4-bit words stored in u8 (low nibble)
    latched_address: u8,          // Address latch (8-bit)
    cs: bool,                     // Chip select
    we: bool,                     // Write enable
    oe: bool,                     // Output enable (optional)
    output_data: u8,              // Latched output for this phase
    // For cycle-accurate simulation:
    word_line_active: bool,       // Word line enabled (from decoder)
    bit_line_developing: bool,    // Bit lines being sensed (timing)
    write_pulse_active: bool,     // Write current flowing (timing)
}
```

## 10. TIMING SPECIFICATIONS (Summary)

| Parameter | Symbol | Min | Max | Unit | Notes |
|-----------|--------|-----|-----|------|-------|
| Address to Output | tAA | - | 200 | ns | Full read cycle |
| Address Setup | tAS | 50 | - | ns | Before CS/WE |
| Data Setup | tDS | 50 | - | ns | Before WE |
| Data Hold | tDH | 50 | - | ns | After WE |
| Write Pulse Width | tWP | 30 | 100 | ns | WE pulse duration |
| Write Cycle | tWC | - | 300 | ns | Total write time |
| Output Hold | tOH | 20 | - | ns | After address change |
| Output Disable | tOZ | - | 50 | ns | After OE/CS |
| CS to Output | tCO | - | 100 | ns | Chip select propagation |

## 11. TEST STRATEGY

### Unit Tests

1. **Read Operations**
   - Read from empty locations (all zeros)
   - Read from written locations
   - Read with different addresses
   - Read timing verification

2. **Write Operations**
   - Write single word
   - Write multiple words to different addresses
   - Write pulse timing
   - Data corruption during partial write (verify doesn't happen)

3. **Address Decoding**
   - All 256 addresses unique
   - No address conflicts
   - Decoder timing meets spec

4. **Control Signals**
   - CS high/low transitions
   - WE timing relative to data
   - Output enable functionality

5. **Edge Cases**
   - CS goes low during write (write may not complete)
   - Address changes during read (valid hold time)
   - Simultaneous read/write (not standard, but test behavior)

### Integration Tests

1. **With 4289 Interface**
   - Address multiplexing works
   - Timing synchronization with bus cycles
   - Data corruption doesn't occur

2. **With 4040 CPU**
   - CPU can read data written by previous instructions
   - CPU write followed by read returns correct data
   - Interrupt during RAM operation

3. **Stress Tests**
   - Rapid address changes
   - Pattern testing (alternating bits)
   - Temperature/voltage variations (if available)

### Validation Tests

1. **Truth Table Verification**
   - For each address, write value, read value, verify match

2. **Pattern Testing**
   - 0x0000, 0xFFFF, alternating patterns
   - Verify no bit sticking

3. **Endurance Verification**
   - Multiple read/write cycles on same address
   - Verify no degradation

## 12. DESIGN DECISIONS & TRADEOFFS

### Decision 1: Linear vs. Hierarchical Decoder

**Option A (Linear)**: One 256-input NOR gate
- **Pros**: Simplest logic
- **Cons**: Large transistor count, slow propagation

**Option B (Hierarchical)**: 3-stage decoder (8×8×4)
- **Pros**: Balanced speed/area, ~10-15% less silicon
- **Cons**: Slightly more complex logic

**Decision**: Hierarchical (Option B)  
**Rationale**: Better speed/area tradeoff, consistent with 1970s NMOS design practices

### Decision 2: Sense Amplifiers vs. Direct Read

**Option A (No Sense Amp)**: Bit line voltage directly to output drivers
- **Pros**: Simpler circuit
- **Cons**: Slow, power-hungry, sensitive to noise

**Option B (Sense Amplifier)**: Dedicated amplifiers per bit line
- **Pros**: Fast, sensitive, low power
- **Cons**: More transistors, more delay stages

**Decision**: Sense Amplifiers (Option B)  
**Rationale**: Historical accuracy, matches 4101 datasheet behavior

### Decision 3: Static vs. Dynamic Cells

**Option A (Static - 6T)**: Cross-coupled inverters + access transistors
- **Pros**: No refresh needed, data retention indefinite
- **Cons**: Larger cell area (6 transistors vs. 1 for DRAM)

**Option B (Dynamic - 1T+C)**: Single transistor + capacitor
- **Pros**: Smaller cell area
- **Cons**: Refresh required, complex refresh logic

**Decision**: Static (Option A)  
**Rationale**: 4101 is defined as static RAM, no refresh logic needed for behavioral model

## 13. NEXT STEPS

1. **Phase 3.1**: Implement full behavioral model with timing
2. **Phase 3.2**: Add cycle-accurate simulation (track bit line charging, write pulse timing)
3. **Phase 3.3**: Integrate with 4289 Standard Memory Interface
4. **Phase 3.4**: Add comprehensive test suite (unit + integration)
5. **Phase 4**: Transistor-level extraction from die (if pursuing physical extraction)

## REFERENCES

- Intel 4101 Datasheet (1973)
- MCS-40 User Manual (Section on Memory Interface)
- Rabaey, Pedram: "Digital Integrated Circuits" (static RAM design chapter)
- Personal extraction notes: netlist_v0, netlist_v1 for 4002 (similar architecture)

---

**Created**: 2026-01-29  
**Author**: Claude Haiku 4.5  
**Status**: Design Document Complete - Ready for Implementation
