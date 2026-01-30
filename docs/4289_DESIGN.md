# Intel 4289 Standard Memory Interface - Architecture Design

**Date**: 2026-01-29  
**Status**: Design Phase  
**Component**: MCS-40 Memory Interface Controller  
**Priority**: Phase 3 Foundational  

## 1. OVERVIEW

The Intel 4289 is the **Standard Memory Interface** for MCS-40 microcomputer systems. It provides:
- Address multiplexing (converts 12-bit CPU address to 8-bit RAM address + bank/page select)
- Control signal generation (read/write, chip enable, output enable)
- Memory device selection and timing synchronization
- Support for multiple memory types (4101 RAM, 4102/4104 ROM, 4316 EEPROM)
- Phase-accurate bus protocol with the 4040 CPU

### Key Characteristics
- **Address Space**: 12-bit (4096 locations) with bank/page organization
- **Data Width**: 4 bits (fully compatible with MCS-40 bus)
- **Memory Devices Supported**: 4101 (256×4 RAM), 4102/4104 (1024×8 ROM), 4316 (2K EEPROM)
- **Bank Organization**: 16 banks × 256 locations = 4096 total addressing
- **Interface**: Direct CPU bus connection + discrete memory device connections
- **Timing**: Synchronous with 8-phase MCS-40 bus cycle

## 2. FUNCTIONAL BLOCK DIAGRAM

```
┌─────────────────────────────────────────────────────────────────┐
│                 4289 Memory Interface Controller                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  From 4040 CPU:                                                 │
│  ┌──────────────────────────────────────────────────┐          │
│  │  A[11:0] (12-bit address)                       │          │
│  │  D[3:0]  (4-bit data bidirectional)             │          │
│  │  WE      (write enable)                         │          │
│  │  RD      (read enable - synthesized)            │          │
│  │  SYNC    (bus phase indicator)                  │          │
│  │  Phase[2:0] (current phase: A1-A3, M1-M2, etc) │          │
│  └──────────────────────────────────────────────────┘          │
│            ↓                                                    │
│  ┌──────────────────────────────────────────────────┐          │
│  │    Address Multiplexer & Decoder                │          │
│  │    - A[11:8] → Bank Select (0-15)               │          │
│  │    - A[7:0]  → Memory Address (0-255)           │          │
│  │    - Latch address at A1-A3 phase               │          │
│  └──────────────────────────────────────────────────┘          │
│            ↓                                                    │
│  ┌──────────────────────────────────────────────────┐          │
│  │    Control Signal Generator                     │          │
│  │    - Chip Select (CS): one per memory bank      │          │
│  │    - Write Enable (WE): asserted in X2-X3       │          │
│  │    - Output Enable (OE): asserted in M1-M2      │          │
│  │    - Address Strobe: for multiplexing           │          │
│  │    - Read/Write Control: synthesis from phase   │          │
│  └──────────────────────────────────────────────────┘          │
│            ↓                                                    │
│  To Memory Devices:                                             │
│  ┌──────────────────────────────────────────────────┐          │
│  │  A[7:0]   (memory address)                      │          │
│  │  D[3:0]   (data bidirectional)                  │          │
│  │  CS[15:0] (chip select - 16 banks)              │          │
│  │  WE       (write enable)                        │          │
│  │  OE       (output enable)                       │          │
│  │  STROBE   (address latch strobe)                │          │
│  └──────────────────────────────────────────────────┘          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 3. ADDRESS MULTIPLEXING SCHEME

### 12-Bit CPU Address Format

```
A[11:0] → 12-bit address from 4040 CPU

Upper Bank Address (A[11:8]):
  0000 → Bank 0  (addresses 0x000-0x0FF)
  0001 → Bank 1  (addresses 0x100-0x1FF)
  0010 → Bank 2  (addresses 0x200-0x2FF)
  ...
  1111 → Bank 15 (addresses 0xF00-0xFFF)

Lower Memory Address (A[7:0]):
  00000000 → Location 0x00
  00000001 → Location 0x01
  ...
  11111111 → Location 0xFF
```

### Address Decoding Logic

```
Bank Selection (from A[11:8]):
  CS[0]  ← (A[11:8] == 4'h0)  ↔ Bank 0 selected
  CS[1]  ← (A[11:8] == 4'h1)  ↔ Bank 1 selected
  CS[2]  ← (A[11:8] == 4'h2)  ↔ Bank 2 selected
  ...
  CS[15] ← (A[11:8] == 4'hF)  ↔ Bank 15 selected

One-hot encoding: exactly one CS line active per address
```

### Memory Bank Organization

```
Bank 0:  4101 RAM[0]    256×4 bits  (addresses 0x000-0x0FF)
Bank 1:  4101 RAM[1]    256×4 bits  (addresses 0x100-0x1FF)
Bank 2:  4101 RAM[2]    256×4 bits  (addresses 0x200-0x2FF)
Bank 3:  4101 RAM[3]    256×4 bits  (addresses 0x300-0x3FF)
Bank 4-7: Reserved or additional RAM banks
Bank 8:  4102 ROM[0]    1024×8 bits (addresses 0x800-0x8FF - lower 256)
Bank 9:  4102 ROM[0]    1024×8 bits (addresses 0x900-0x9FF - middle 256)
Bank 10: 4102 ROM[0]    1024×8 bits (addresses 0xA00-0xAFF - upper 256)
Bank 11: 4102 ROM[0]    1024×8 bits (addresses 0xB00-0xBFF - highest 256)
Bank 12-15: Additional ROM or specialized memory
```

## 4. CONTROL SIGNAL GENERATION

### Read/Write Control

**Read Operation** (Phase M1-M2):
- WE = 0 (not writing)
- OE = 1 (enable output drivers)
- CS[bank] = 1 (chip selected)
- Result: Memory outputs data to CPU

**Write Operation** (Phase X2-X3):
- WE = 1 (enable write)
- OE = 0 (disable outputs, prevent contention)
- CS[bank] = 1 (chip selected)
- Data on bus
- Result: Memory latches data

### Timing Signals

```
Synthesis from Phase:

A1 Phase:   Address setup begins, decoder activates
            ADDR_STROBE = 1 (latch address in memory)
            
A2 Phase:   Address valid on bus
            Memory decoder selects word line
            
A3 Phase:   Address hold, word line selection completes
            
M1 Phase:   Memory phase begins
            If read: OE = 1, output drivers active
            
M2 Phase:   Data valid from memory
            CPU samples data from bus
            
X1 Phase:   Execute phase begins
            If write: WE = 1, data prepared
            
X2 Phase:   Write pulse active
            Write current flows, memory latches data
            
X3 Phase:   Cycle complete
            WE = 0, outputs disabled, cleanup
```

### Control Signal Timing

```
Control Signal State Machine:

State 0 (A1-A3): Address Phase
  ├─ A[11:0] valid on bus
  ├─ Decoder latches address
  ├─ Bank select (CS) changes (one-hot)
  ├─ ADDR_STROBE pulsed to latch in 4289
  └─ WE = 0, OE = 0 (inactive)

State 1 (M1-M2): Memory Phase (Read)
  ├─ If RD operation:
  │  ├─ OE = 1 (output enable)
  │  ├─ Memory outputs data
  │  └─ D[3:0] driven by memory
  └─ If WR operation:
     ├─ OE = 0 (output disabled)
     └─ CPU prepares write data

State 2 (X1-X3): Execute Phase (Write)
  ├─ If WR operation:
  │  ├─ WE = 1 (write enable)
  │  ├─ Write pulse generated (~100ns)
  │  └─ Memory latches D[3:0]
  └─ If RD operation:
     ├─ WE = 0 (always inactive on read)
     └─ OE may remain = 1 (hold for CPU read)
```

## 5. BUS PROTOCOL INTEGRATION

### 8-Phase MCS-40 Cycle

```
┌─────────────────────────────────────────────────────────────┐
│                    One Machine Cycle                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  A1 Phase: Address Setup                                    │
│  ├─ A[11:0] valid on bus from CPU                          │
│  ├─ 4289: Latch A[11:0]                                    │
│  ├─ Decoder: Update CS[15:0]                               │
│  └─ Memory: Word line decoder starts                        │
│                                                              │
│  A2 Phase: Address Hold                                     │
│  ├─ A[11:0] stable on bus                                  │
│  ├─ CS[bank] selection stable                              │
│  └─ Memory: Word line activation continues                 │
│                                                              │
│  A3 Phase: Address Valid                                    │
│  ├─ Address decoder fully settled                          │
│  ├─ Word line selection complete                           │
│  └─ Bit line precharge (for reads)                         │
│                                                              │
│  M1 Phase: Data Read Begin                                  │
│  ├─ OE = 1 for read operations                             │
│  ├─ Memory output drivers active                           │
│  ├─ D[3:0] begins driving bus                              │
│  └─ Sense amplifier output begins settling                 │
│                                                              │
│  M2 Phase: Data Valid                                       │
│  ├─ D[3:0] valid from memory                               │
│  ├─ CPU samples D[3:0] if read operation                   │
│  └─ Data setup time met (>100ns from address)              │
│                                                              │
│  X1 Phase: Execute/Write Setup                              │
│  ├─ If write: CPU drives D[3:0] with new data              │
│  ├─ Data setup time before write pulse                     │
│  └─ Memory: Prepare to accept write                        │
│                                                              │
│  X2 Phase: Write Pulse Active                               │
│  ├─ WE = 1 (write enable pulse)                            │
│  ├─ Write current flows in memory                          │
│  ├─ Cross-coupled inverters flip (latch data)              │
│  └─ Write pulse width: ~100ns                              │
│                                                              │
│  X3 Phase: Cycle Complete                                   │
│  ├─ WE = 0 (write pulse ends)                              │
│  ├─ OE = 0 (outputs disabled for write)                    │
│  ├─ Data latched in memory                                 │
│  └─ Ready for next cycle                                   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## 6. MEMORY DEVICE SUPPORT

### Supported Memory Devices

| Device | Type | Capacity | Address Bits | Data Width | Interface |
|--------|------|----------|--------------|------------|-----------|
| 4101 | RAM | 256×4 | 8 | 4 | Asynchronous |
| 4102 | ROM | 1024×8 | 10 | 8 | Asynchronous |
| 4104 | ROM | 512×8 | 9 | 8 | Asynchronous |
| 4316 | EEPROM | 2K×8 | 11 | 8 | Asynchronous |
| 3226 | I/O Driver | N/A | N/A | 4 | Buffered |

### Device Selection by Address Range

```
Address 0x000-0x3FF: 4101 RAM Banks (Banks 0-3, 256×4 each)
  Total: 1024 locations, 4-bit wide

Address 0x400-0x7FF: Optional Additional RAM
  (Future expansion)

Address 0x800-0xFFF: 4102 ROM Banks (Banks 8-15, 1K×8 each)
  Organized as 4 × 256-byte windows into single 1024×8 ROM
  Bank 8:  0x800-0x8FF (ROM[0:255])
  Bank 9:  0x900-0x9FF (ROM[256:511])
  Bank 10: 0xA00-0xAFF (ROM[512:767])
  Bank 11: 0xB00-0xBFF (ROM[768:1023])
```

### Device Configuration

**4101 RAM (Banks 0-3)**:
- 256 locations × 4 bits
- CS, WE, OE control signals
- Asynchronous (no clock required)
- Data hold indefinite while powered
- Address to output time: ~200ns

**4102 ROM (Bank 8, shared across banks 9-11)**:
- 1024 locations × 8 bits
- Address multiplexing from 12-bit to 10-bit
- CS for bank selection
- OE for output enable
- Read-only (no WE signal)
- Preprogrammed at manufacturing

**3226 I/O Driver (Optional, Bank 12-15)**:
- Buffered I/O for external devices
- Can drive/sink higher current
- Controlled by 4289 via CS selection
- Useful for 7-segment displays, relays, etc.

## 7. STRUCTURAL IMPLEMENTATION

### Module Organization

```rust
pub struct I4289 {
    // Address latching and decoding
    latched_address: u16,           // 12-bit address from CPU
    bank_select: u16,               // 16-bit one-hot CS[15:0]
    memory_address: u8,             // 8-bit address to memory
    
    // Control signals
    write_enable: bool,             // WE to memory
    output_enable: bool,            // OE to memory
    addr_strobe: bool,              // Address latch strobe
    
    // Bus interface
    data_bus: u8,                   // 4-bit data (lower nibble)
    data_valid: bool,               // Data on bus is valid
    read_not_write: bool,           // 1=read, 0=write
    
    // Timing state machine
    current_phase: BusPhase,        // Current bus cycle phase
    write_pulse_timer: u8,          // Write pulse duration counter
    
    // Memory device references
    ram_banks: [I4101; 4],          // Banks 0-3 (4101 RAM)
    rom_bank: I4102,                // Bank 8 (4102 ROM, shared 9-11)
    io_drivers: [I3226; 4],         // Banks 12-15 (optional I/O)
}
```

### Key Methods

```rust
impl I4289 {
    // Address and control
    pub fn set_address(&mut self, addr: u16)
    pub fn set_write_enable(&mut self, we: bool)
    pub fn set_output_enable(&mut self, oe: bool)
    pub fn set_read_not_write(&mut self, rnw: bool)
    
    // Bus interface
    pub fn write_data(&mut self, data: u8)
    pub fn read_data(&self) -> u8
    
    // Cycle control
    pub fn tick(&mut self, phase: BusPhase)
    pub fn on_address_phase(&mut self)
    pub fn on_memory_phase(&mut self)
    pub fn on_execute_phase(&mut self)
    
    // Chip select
    fn decode_chip_select(&mut self)
    fn get_active_bank(&self) -> usize
}
```

## 8. TIMING SPECIFICATIONS

| Parameter | Min | Max | Unit | Description |
|-----------|-----|-----|------|-------------|
| tAA | - | 200 | ns | Address to output (read) |
| tACS | - | 200 | ns | Address/CS to output |
| tDS | 50 | - | ns | Data setup before WE |
| tDH | 50 | - | ns | Data hold after WE |
| tWP | 30 | 100 | ns | Write pulse width |
| tWC | - | 300 | ns | Write cycle time |
| tOE | - | 50 | ns | Output enable time |
| tOZ | - | 50 | ns | Output disable (tri-state) |
| tCS | - | 100 | ns | Chip select to output |

## 9. DESIGN DECISIONS & TRADEOFFS

### Decision 1: Address Multiplexing Approach

**Option A (Full Multiplexing)**: Time-divide 12-bit address into 4 phases
- Pros: Reduces pin count significantly
- Cons: Complex state machine, timing critical

**Option B (Static Address)**:12-bit latched address during entire cycle
- Pros: Simpler implementation, transparent to CPU
- Cons: More pins, slightly higher complexity

**Decision**: Static Address (Option B)  
**Rationale**: Simpler protocol, better integration with 4040 bus

### Decision 2: Bank Selection Encoding

**Option A (Binary)**: A[11:8] encodes 16 banks with decoder
- Pros: Fewer decoder gates
- Cons: More complex decoding logic

**Option B (One-Hot)**: CS[15:0] with independent latch per bank
- Pros: Simpler, faster selection
- Cons: More control lines (16 vs 4)

**Decision**: One-Hot (Option B)  
**Rationale**: Cleaner implementation, matches historical 4289 design

### Decision 3: Data Bus Direction

**Option A (Separate buses)**: Dedicated read + write buses (8 pins total)
- Pros: No contention, faster
- Cons: More wiring, more complexity

**Option B (Bidirectional)**: Single shared data bus with tri-state
- Pros: Fewer pins, matches MCS-40 architecture
- Cons: Requires proper tri-state control

**Decision**: Bidirectional (Option B)  
**Rationale**: Matches 4040 CPU bus design philosophy

## 10. TESTING STRATEGY

### Unit Tests

1. **Address Decoding**
   - All 16 banks produce correct CS encoding
   - Address A[7:0] correctly latched
   - No address conflicts or glitches

2. **Control Signal Generation**
   - WE/OE timing per phase
   - No simultaneous write+read
   - CS transitions clean

3. **Memory Access**
   - Read from each bank
   - Write to each bank
   - Data consistency (read-after-write)

4. **Timing Compliance**
   - Address setup time met
   - Data setup/hold time met
   - Write pulse duration within spec

### Integration Tests

1. **With 4040 CPU**
   - CPU can read data written to RAM
   - ROM reads work correctly
   - Interrupts don't corrupt memory access

2. **Multi-Bank Operations**
   - Switch banks, verify addressing
   - No cross-bank interference
   - Bank selection stable

3. **Stress Tests**
   - Rapid address changes
   - Alternating read/write
   - All banks exercised in sequence

## 11. NEXT STEPS

1. **Phase 3.2**: Implement behavioral I4289 model
2. **Phase 3.3**: Add cycle-accurate timing simulation
3. **Phase 3.4**: Integrate with 4040 system
4. **Phase 3.5**: Comprehensive test suite
5. **Phase 4**: Transistor-level extraction (if pursuing)

## REFERENCES

- Intel 4289 Datasheet (1973)
- MCS-40 User Manual (Memory Interface section)
- 4101 RAM Datasheet (256×4 static RAM)
- 4102 ROM Datasheet (1024×8 ROM)

---

**Created**: 2026-01-29  
**Author**: Claude Haiku 4.5  
**Status**: Design Document Complete - Ready for Implementation
