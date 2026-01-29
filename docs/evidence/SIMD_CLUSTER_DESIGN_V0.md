# SIMD Cluster Execution Design (v0)

**Date**: 2026-01-29
**Status**: DESIGN DOCUMENT
**Target**: Phase 4 implementation
**Rust Version**: 1.94.0-nightly (2026-01-06)

---

## Executive Summary

This document defines the SIMD (Single Instruction, Multiple Data) cluster execution architecture for running multiple instances of the 4004/4040 CPU in parallel using Rust's portable_simd feature. The goal is to enable parallel fuzz testing, differential testing, and ROM validation by executing up to 16 CPU instances simultaneously.

---

## Motivation

### Use Cases

1. **Parallel Fuzz Testing**: Run same ROM with different inputs across lanes
2. **Differential Testing**: Run same inputs, verify bit-identical outputs
3. **ROM Validation**: Execute multiple test ROMs in parallel
4. **Performance**: Achieve near-linear speedup for batch workloads

### Performance Goals

- 16-way parallelism (16 CPU instances)
- <2x overhead vs sequential execution
- Deterministic outputs (reproducible results)
- Zero data races (pure functional operation)

---

## Rust SIMD Capabilities

### portable_simd Feature

**Status**: Nightly-only (as of 2026-01-06)
**Crate**: `std::simd`
**Documentation**: https://doc.rust-lang.org/std/simd/

### SIMD Types Available

```rust
use std::simd::{
    Simd, SimdUint, SimdInt,
    u8x4, u8x8, u8x16, u8x32, u8x64,
    u16x4, u16x8, u16x16, u16x32,
    u32x4, u32x8, u32x16,
    u64x2, u64x4, u64x8,
    mask8x4, mask8x8, mask8x16,
};
```

### Lane Counts Supported

- **4 lanes**: u8x4, u16x4, u32x4, u64x4
- **8 lanes**: u8x8, u16x8, u32x8, u64x8 (most common)
- **16 lanes**: u8x16, u16x16, u32x16 (target for this project)
- **32/64 lanes**: u8x32, u8x64 (AVX-512, less portable)

**Chosen Lane Count**: 16 lanes (u8x16, u16x16)
**Rationale**: Good balance between parallelism and hardware support (AVX2)

---

## 4004/4040 CPU State

### State to Vectorize

**4004 CPU State** (46 instructions):
```rust
struct I4004State {
    accumulator: u8,        // 4-bit
    carry: bool,            // 1-bit
    pc: u16,                // 12-bit (0x000-0xFFF)
    stack: [u16; 3],        // 3 levels x 12-bit
    registers: [u8; 16],    // 16 x 4-bit
    index_pair: u8,         // 3-bit (0-7)
}
```

**4040 CPU State** (60 instructions):
```rust
struct I4040State {
    accumulator: u8,        // 4-bit
    carry: bool,            // 1-bit
    pc: u16,                // 12-bit
    stack: [u16; 7],        // 7 levels x 12-bit (extended)
    registers: [u8; 24],    // 24 x 4-bit (extended)
    bank: u8,               // 1-bit (register bank selection)
    int_enabled: bool,      // 1-bit
    int_pending: bool,      // 1-bit
    src_save: u8,           // 8-bit
    halted: bool,           // 1-bit
}
```

### Vectorization Strategy

**Approach**: Struct-of-Arrays (SoA)

Instead of:
```rust
struct CpuArray {
    cpus: [I4004State; 16],  // Array of Structs (AoS)
}
```

Use:
```rust
struct SimdCpuCluster {
    accumulators: u8x16,     // 16 accumulators (4-bit each, stored in u8)
    carries: mask8x16,       // 16 carry flags
    pcs: u16x16,             // 16 program counters
    stack_0: u16x16,         // Stack level 0 for all 16 CPUs
    stack_1: u16x16,         // Stack level 1
    stack_2: u16x16,         // Stack level 2
    registers: [u8x16; 16],  // 16 register indices x 16 lanes
    index_pairs: u8x16,      // 16 index pairs
}
```

**Advantages**:
- Natural SIMD operations (add, sub, shift apply to all lanes)
- Better cache locality
- Vectorized conditionals via masks

**Challenges**:
- Control flow (branches differ per lane)
- Memory access (each CPU has independent ROM/RAM)
- Synchronization (bus phases must align)

---

## Control Flow Handling

### Problem: Divergent Execution Paths

When different CPU instances take different branches:
```rust
// Lane 0: JCN (carry set) -> jump
// Lane 1: JCN (carry clear) -> no jump
// Lanes have diverged!
```

### Solution: Masked Execution

**Approach**: Execute both paths, blend results using masks

```rust
// Pseudo-code for JCN (jump if condition)
let condition_mask = self.check_condition(cond);  // mask8x16

// Compute both outcomes
let pc_jump = self.pcs + offset;    // u16x16 (all lanes jump)
let pc_no_jump = self.pcs + 1;      // u16x16 (all lanes continue)

// Blend based on mask
self.pcs = condition_mask.select(pc_jump, pc_no_jump);
```

**Trade-off**:
- Executes both paths (redundant work)
- But stays vectorized (no lane divergence)
- Works well for small conditional code (4004 instructions are simple)

---

## Memory Access Pattern

### Challenge: Independent ROM/RAM per Lane

Each CPU instance has its own:
- ROM (program memory, 4096 x 8-bit)
- RAM (320 bits for 4002 chips)
- I/O ports (chip-specific)

### Solution: Gather/Scatter Operations

**Scalar Read** (lane-by-lane):
```rust
// Read ROM for each lane individually
let mut rom_values = [0u8; 16];
for i in 0..16 {
    let pc = self.pcs.as_array()[i];
    rom_values[i] = self.roms[i][pc as usize];
}
let instruction_bytes = u8x16::from_array(rom_values);
```

**Hardware Gather** (if supported):
```rust
// Future: use AVX2 gather instructions
// Requires unsafe and platform-specific intrinsics
let instruction_bytes = unsafe {
    simd_gather(self.rom_ptrs, self.pcs, scale=1)
};
```

**Decision**: Start with scalar gather, optimize later with intrinsics

---

## Bus Synchronization

### Challenge: Phase-Accurate Emulation

MCS-4 bus operates in 8 phases per instruction cycle:
- A1, A2, A3: Address output
- M1, M2: Memory operation
- X1, X2, X3: Execution

**Synchronization Strategy**:
1. All 16 CPUs execute in lockstep (same phase)
2. Bus signals aggregated across lanes (16 parallel busses)
3. Chip selects computed per-lane (each CPU has independent addressing)

### Simplified Approach

For initial implementation:
- **Instruction-level synchronization** (not phase-level)
- All lanes execute same number of cycles
- No inter-lane communication (CPUs are independent)

---

## Implementation Architecture

### Module Structure

```
mcs4-emu/crates/mcs4-system/src/
├── simd_cluster.rs          (main SIMD cluster)
├── simd_i4004.rs            (vectorized 4004 CPU)
├── simd_memory.rs           (ROM/RAM arrays)
└── simd_tests.rs            (unit tests)
```

### Core Types

```rust
/// SIMD cluster executing 16 x 4004 CPUs in parallel
pub struct SimdCluster<const N: usize = 16> {
    /// CPU state (vectorized)
    cpu: SimdI4004<N>,

    /// Independent ROM for each lane
    roms: Vec<Vec<u8>>,  // [N][4096]

    /// Independent RAM for each lane
    rams: Vec<Vec<u8>>,  // [N][320/8]

    /// Execution statistics
    cycles: u64,
    instructions: u64,
}

/// Vectorized 4004 CPU (16 lanes)
pub struct SimdI4004<const N: usize> {
    accumulator: u8xN,
    carry: mask8xN,
    pc: u16xN,
    stack: [u16xN; 3],
    registers: [u8xN; 16],
    index_pair: u8xN,
}

impl<const N: usize> SimdI4004<N> {
    /// Execute one instruction across all lanes
    pub fn tick(&mut self, roms: &[Vec<u8>]) {
        // 1. Fetch instructions (gather from each lane's ROM)
        let opcodes = self.fetch_opcodes(roms);

        // 2. Decode and execute (vectorized)
        self.execute_vectorized(opcodes);

        // 3. Update program counters
        self.increment_pcs();
    }
}
```

---

## Differential Testing Strategy

### Test Procedure

1. **Load identical ROM** into all 16 lanes
2. **Initialize with identical state**
3. **Execute N cycles** in parallel
4. **Compare outputs** across lanes

**Expected**: All lanes produce bit-identical results

**Validation**:
```rust
fn test_differential() {
    let mut cluster = SimdCluster::new();

    // Load same ROM to all lanes
    for i in 0..16 {
        cluster.load_rom(i, &test_rom);
    }

    // Execute 1000 cycles
    for _ in 0..1000 {
        cluster.tick();
    }

    // Verify all accumulators match
    let accs = cluster.cpu.accumulator.as_array();
    assert!(accs.windows(2).all(|w| w[0] == w[1]));
}
```

---

## Performance Benchmarking

### Metrics to Measure

1. **Throughput**: Instructions/second (aggregate across 16 lanes)
2. **Speedup**: vs single-lane sequential execution
3. **Efficiency**: Speedup / 16 (ideal = 1.0, i.e., 16x speedup)
4. **Overhead**: Clock cycles per instruction (should be close to 1.0)

### Benchmark Suite

```rust
#[bench]
fn bench_simd_cluster_nop_loop(b: &mut Bencher) {
    let cluster = setup_cluster_with_nop_loop();
    b.iter(|| {
        cluster.execute_cycles(1000);
    });
}

#[bench]
fn bench_sequential_nop_loop(b: &mut Bencher) {
    let cpus = setup_16_sequential_cpus();
    b.iter(|| {
        for cpu in &mut cpus {
            cpu.execute_cycles(1000);
        }
    });
}
```

**Target**: Speedup >= 8x (50% efficiency)

---

## Implementation Plan

### Phase 4 Task 29: Implement simd_cluster.rs

**Step 1: Basic Structure** (Day 1)
- Define SimdCluster and SimdI4004 types
- Implement ROM/RAM loading
- Stub instruction execution

**Step 2: Instruction Execution** (Day 2-3)
- Implement fetch/decode/execute pipeline
- Add vectorized ALU operations (ADD, SUB, INC, DAA, etc.)
- Add control flow with masked execution (JCN, JUN, JMS, BBL)

**Step 3: Memory Operations** (Day 4)
- Implement WRM, WMP, WRR, WR0-3 (RAM/port writes)
- Implement RDM, RDR, RD0-3 (RAM/port reads)
- Handle gather/scatter for per-lane memory

**Step 4: Testing** (Day 5)
- Unit tests for each instruction
- Differential testing (same ROM, verify identical outputs)
- Benchmark suite

**Step 5: Optimization** (Day 6-7)
- Profile hot paths
- Replace scalar gather with SIMD intrinsics (if profitable)
- Tune memory layout

---

## Limitations and Future Work

### Current Limitations

1. **No inter-lane communication**: CPUs are independent
2. **Fixed lane count**: Requires recompilation to change N
3. **Scalar memory access**: Gather/scatter not using hardware instructions
4. **No phase-accurate bus**: Instruction-level granularity only

### Future Enhancements

1. **Dynamic lane count**: Runtime selection based on CPU features
2. **Hardware intrinsics**: Use AVX2 vpgather for memory loads
3. **Nested parallelism**: SIMD + rayon for multi-core + SIMD
4. **Waveform export**: Visualize all 16 lanes simultaneously
5. **FPGA co-simulation**: Offload lanes to FPGA fabric

---

## References

- [Rust portable_simd documentation](https://doc.rust-lang.org/std/simd/)
- [Intel 4004 datasheet](../docs/primary_sources/)
- [Phase 4 plan](../ROADMAP.md#phase-4-clustering-and-performance)

---

**Author**: Claude Sonnet 4.5 (claude-sonnet-4-5-20250929)
**Date**: 2026-01-29
**Status**: DESIGN COMPLETE - READY FOR IMPLEMENTATION
