# Intel MCS-40 System Integration - Complete Architecture

**Date**: 2026-01-29  
**Status**: System Architecture Complete  
**Component**: MCS-40 Microcomputer System  
**Priority**: Phase 3 Capstone  

## 1. SYSTEM OVERVIEW

The **Intel MCS-40** is a complete 4-bit microcomputer system comprising:
- **CPU**: 4040 (16-bit PC, 24 registers, 7-level stack, interrupts)
- **Memory Interface**: 4289 (address multiplexing, control generation)
- **RAM**: 4101 (256×4 bits, 16 banks = 4K total with multiplexing)
- **ROM**: 4001/4102 (512/1024 words × 8 bits)
- **Clock**: 4201 (clock generation, phase distribution)
- **Shift Register**: 4003 (10-bit parallel I/O)
- **I/O Drivers**: 3226/3225 (current amplification)

### System Block Diagram

```
┌────────────────────────────────────────────────────────────────────┐
│                     Intel MCS-40 Microcomputer                     │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│  ┌─────────────┐                                                  │
│  │  4201 Clock │──(PHI1, PHI2)──┐                                 │
│  │ Generator   │──(SYNC)─────────┤                                │
│  └─────────────┘                │                                │
│                                │                                 │
│  ┌──────────────────────────────┼────────────────┐               │
│  │                              ↓                 │               │
│  │  ┌────────────────────────────────────────┐   │               │
│  │  │         4040 CPU (16-bit PC)           │   │               │
│  │  │  - ALU, 24 registers (R0-R23)          │   │               │
│  │  │  - 7-level stack (up to 2048 calls)    │   │               │
│  │  │  - Interrupt handling (INT pin)        │   │               │
│  │  │  - 60 instructions (46 4004 + 14 4040) │   │               │
│  │  └────────────────────────────────────────┘   │               │
│  │                    ↕                           │               │
│  │         ┌──────────────────────┐               │               │
│  │         │  4289 Memory Interface │              │               │
│  │         │  - Address multiplexing │             │               │
│  │         │  - Control generation   │            │               │
│  │         │  - Bank selection (16)  │            │               │
│  │         └──────────────────────┘               │               │
│  │                    ↕                           │               │
│  │  ┌────────────────────────────────────────┐   │               │
│  │  │       Memory Address Space (12-bit)    │   │               │
│  │  │  Banks 0-3:   4101 RAM (256×4 each)   │   │               │
│  │  │  Banks 4-7:   Reserved/Optional       │   │               │
│  │  │  Banks 8-11:  4102 ROM (shared 1K×8)  │   │               │
│  │  │  Banks 12-15: I/O Drivers (optional)  │   │               │
│  │  └────────────────────────────────────────┘   │               │
│  │                                                │               │
│  │  ┌──────────────────────────────────────┐    │               │
│  │  │    4-Bit Bidirectional Data Bus      │    │               │
│  │  │    - D[3:0] multiplexed read/write   │    │               │
│  │  │    - Address/Data time-multiplexing  │    │               │
│  │  │    - Control signal arbitration      │    │               │
│  │  └──────────────────────────────────────┘    │               │
│  │                                                │               │
│  └────────────────────────────────────────────────────────────────┘
│
│  Interconnects: 3226/3225 I/O drivers for external peripherals
└────────────────────────────────────────────────────────────────────┘
```

## 2. SYSTEM COMPONENTS

### 2.1 4040 CPU

**Features**:
- 12-bit program counter (4K address space)
- 16-bit accumulator and flags
- 24 general-purpose registers (R0-R23)
- Register bank switching (2 banks of 8 registers)
- 7-level call stack (vs 3-level in 4004)
- 60 instructions total:
  - 46 4004-compatible instructions
  - 14 new 4040 instructions (HLT, BBS, OR4/5, AN6/7, DB0/1, SB0/1, EIN/DIN, RPM, LCR)
- Interrupt support (INT pin sampling, vector to 0x003)
- Carry flag, accumulator zero flag
- Decimal mode for BCD operations

**Key Registers**:
- A: Accumulator (4 bits)
- PC: Program counter (12 bits)
- Bank: Current register bank (1 bit)
- RAM Bank: Current RAM bank select (2 bits for 4101 addressing)
- Carry/Zero: Status flags
- INT Enable: Interrupt enable flag

### 2.2 4289 Memory Interface

**Functions**:
- Multiplexes 12-bit CPU address to 8-bit RAM address + bank select
- Generates control signals (CS, WE, OE) for memory devices
- Synchronizes with 8-phase MCS-40 bus cycle
- Supports 16 memory banks (each 256 locations for 4101)
- Total addressable: 4096 locations (4K memory)

**Address Mapping**:
- A[11:8] → Bank select (CS[15:0], one-hot)
- A[7:0] → Location address to memory
- Logical address 0x000 → Bank 0, Location 0
- Logical address 0x0FF → Bank 0, Location 255
- Logical address 0x100 → Bank 1, Location 0
- Logical address 0xFFF → Bank 15, Location 255

### 2.3 4101 RAM (Banks 0-3)

**Specifications**:
- 256 locations × 4 bits per bank
- 4 banks × 256 = 1024 locations (1K)
- Static RAM (no refresh)
- Asynchronous read/write
- Tri-state output drivers
- CS, WE, OE control

**Access Pattern**:
- Read: CS=1, WE=0, OE=1 → data on bus
- Write: CS=1, WE=1, OE=0 → accepts data
- Data hold indefinite while powered

### 2.4 4102 ROM (Bank 8, shared with 9-11)

**Specifications**:
- 1024 locations × 8 bits
- Preprogrammed at manufacturing
- Organized as 4 × 256-byte windows
- Bank 8: 0x800-0x8FF (ROM[0:255])
- Bank 9: 0x900-0x9FF (ROM[256:511])
- Bank 10: 0xA00-0xAFF (ROM[512:767])
- Bank 11: 0xB00-0xBFF (ROM[768:1023])
- Read-only (no write operations)

### 2.5 4201 Clock Generator

**Functions**:
- Crystal oscillator interface
- Generates PHI1, PHI2 clock signals
- Distributes SYNC signal
- Controls 8-phase bus cycle:
  - A1, A2, A3: Address phase
  - M1, M2: Memory/Data phase
  - X1, X2, X3: Execute phase

## 3. MEMORY MAP

### 12-Bit Address Space (0x000-0xFFF)

```
0x000 - 0x0FF: Bank 0 (4101 RAM) - 256 × 4 bits
0x100 - 0x1FF: Bank 1 (4101 RAM) - 256 × 4 bits
0x200 - 0x2FF: Bank 2 (4101 RAM) - 256 × 4 bits
0x300 - 0x3FF: Bank 3 (4101 RAM) - 256 × 4 bits
0x400 - 0x7FF: Reserved or optional additional RAM
0x800 - 0x8FF: Bank 8 (4102 ROM window 0, locations 0-255)
0x900 - 0x9FF: Bank 9 (4102 ROM window 1, locations 256-511)
0xA00 - 0xAFF: Bank 10 (4102 ROM window 2, locations 512-767)
0xB00 - 0xBFF: Bank 11 (4102 ROM window 3, locations 768-1023)
0xC00 - 0xDFF: Reserved or I/O (Banks 12-13)
0xE00 - 0xEFF: Reserved or I/O (Bank 14)
0xF00 - 0xFFF: Reserved or I/O (Bank 15)
```

### Special Addresses

**Reset Vector**:
- 0x000: First instruction executed on reset
- Typically: JUN to main() function

**Interrupt Vector**:
- 0x003: Interrupt handler entry point
- Accessed when INT pin asserted
- CPU pushes PC, jumps to 0x003

**ROM Alias**:
- 0x800-0xBFF: Banks 8-11 access same 1024×8 ROM with different windows

## 4. BUS PROTOCOL

### 8-Phase Cycle

Each machine cycle consists of 8 bus phases with specific operations:

**Phase A1 (Address Setup)**:
- CPU outputs 12-bit address on bus
- 4289 latches address
- Decoder selects bank (CS[15:0])
- Memory word line selection begins

**Phase A2 (Address Hold)**:
- Address stable on bus
- Word line activation continues
- Bit line precharge (for reads)

**Phase A3 (Address Valid)**:
- Address decoder fully settled
- Word line selection complete
- Memory ready for read or write

**Phase M1 (Memory Read Begin)**:
- For read operations: OE=1, output drivers active
- Data begins driving bus
- Sense amplifier settling

**Phase M2 (Data Valid)**:
- Memory data valid on bus
- CPU samples data (if read operation)
- Data setup time met

**Phase X1 (Execute/Write Setup)**:
- If write: CPU drives data bus
- Data setup before write pulse
- ALU operations continue

**Phase X2 (Write Pulse Active)**:
- WE=1, write current flows
- Memory latches data
- Latch flip completes

**Phase X3 (Cycle Complete)**:
- WE=0, outputs disabled
- Data latched in memory
- Ready for next cycle

### Timing Specifications

| Event | Phase | Min | Max | Duration |
|-------|-------|-----|-----|----------|
| Address setup | A1 | - | - | 1 phase |
| Address valid | A3 | - | - | settled |
| Data output | M1-M2 | - | 200ns | 200ns |
| Data read | M2 | - | - | 1 phase |
| Write setup | X1 | 50ns | - | setup time |
| Write pulse | X2 | 30ns | 100ns | pulse |
| Write complete | X3 | - | - | settled |

## 5. INTERRUPT HANDLING

### Interrupt Processing

**Trigger**:
1. INT pin asserted
2. 4040 samples INT at A1 phase
3. Interrupt enabled (EIN instruction executed)

**Interrupt Service**:
1. Current PC pushed to stack
2. Stack pointer decremented
3. PC loaded with 0x003
4. Interrupts auto-disabled
5. Execute interrupt handler code

**Return from Interrupt**:
1. BBS instruction executes
2. Pops return address from stack
3. Restores PC
4. Re-enables interrupts
5. Returns to normal execution

### Example Interrupt Handler

```asm
0x000: EIN        ; Enable interrupts
0x001: NOP        ; Main program...
0x003: (Interrupt handler starts here)
       ...        ; Handle interrupt
       BBS        ; Return to 0x001
```

## 6. SYSTEM INTEGRATION POINTS

### 6.1 CPU-to-Memory Interface

```
CPU → Memory:
  A[11:0]      12-bit address
  D[3:0]       4-bit data (write)
  WE           Write enable
  RD           Read enable (synthesized)
  
Memory → CPU:
  D[3:0]       4-bit data (read)
  VALID        Data valid signal
```

### 6.2 4289 Interfacing

```
From CPU: A[11:0], phase signal, read/write indication
To CPU: Multiplexed bus timing
From RAM: CS[15:0], A[7:0], WE, OE
From ROM: Similar control generation
```

### 6.3 Clock Distribution

```
4201 Clock → All chips:
  PHI1         Phase 1 clock
  PHI2         Phase 2 clock
  SYNC         Sync signal (start of cycle)
```

## 7. TESTING STRATEGY

### Unit Tests

1. **CPU Tests** (43 tests): All instructions, stack, interrupts
2. **RAM Tests** (17 tests): Read/write, all banks, patterns
3. **Disassembler Tests** (8 tests): All instructions, listing format
4. **Trace Buffer Tests** (18 tests): Recording, filtering, analysis

Total: 86 unit tests

### Integration Tests

1. **Memory Access**:
   - Write to RAM bank 0, read back
   - Repeat for all 4 banks
   - Verify no cross-bank interference

2. **Interrupt Service**:
   - Raise INT, execute handler
   - Return with BBS
   - Verify correct return address

3. **Program Flow**:
   - Load ROM with program
   - Execute multi-byte instructions (JUN, JMS)
   - Verify PC advancement correct

### System Tests

1. **Complete Programs**:
   - Fibonacci sequence
   - Counter with RAM increment
   - Interrupt-driven timer

2. **Stress Tests**:
   - Rapid bank switching
   - Nested subroutines (7 deep)
   - Mixed read/write pattern

## 8. DESIGN DECISIONS

### Decision 1: Bank Organization

**Option A**: Linear 4096-entry RAM (no banking)
- Simpler addressing
- More memory required per device

**Option B**: Banked 1K RAM (4 × 4101 chips)
- Matches historical 4289 design
- Efficient use of available devices
- Requires bank selection logic

**Chosen**: Option B (Banked)
**Rationale**: Matches historical system, efficient device utilization

### Decision 2: Address Space

**Option A**: 10-bit address (1K maximum)
- Fewer address lines
- More limited addressability

**Option B**: 12-bit address (4K addressable)
- More flexible for future expansion
- Matches 4040 PC width
- Better growth path

**Chosen**: Option B (12-bit)
**Rationale**: Better long-term flexibility, matches CPU design

## 9. PERFORMANCE CHARACTERISTICS

### Execution Timing

- **Single-byte instruction**: 1 machine cycle = 8 bus phases = ~800ns (10MHz clock)
- **Two-byte instruction**: 2 machine cycles = 16 bus phases = ~1600ns
- **RAM access**: 1 read or write per machine cycle
- **Interrupt latency**: 1-2 cycles (pending current instruction)

### Throughput

- **Instruction rate**: 1.25 MIPS (at 10MHz clock)
- **Memory bandwidth**: 4 bits × 1.25 MHz = 5 Mbits/sec
- **Stack operations**: 7-level → up to 7 nested calls before limit

### Memory Capacity

- **RAM**: 1024 × 4 bits = 4096 bits (512 bytes)
- **ROM**: 1024 × 8 bits = 8192 bits (1024 bytes)
- **Total**: 12,288 bits (1536 bytes)

## 10. NEXT STEPS

1. **Phase 3.5**: System integration testing
2. **Phase 3.6**: Waveform viewer integration with trace buffer
3. **Phase 4**: Transistor-level simulation (if pursuing)
4. **Phase 5**: FPGA synthesis and hardware implementation

## REFERENCES

- Intel MCS-40 User Manual (1973)
- 4040 CPU Datasheet
- 4289 Memory Interface Datasheet
- 4101 RAM Datasheet
- 4102 ROM Datasheet

---

**Created**: 2026-01-29  
**Author**: Claude Haiku 4.5  
**Status**: System Integration Architecture Complete
