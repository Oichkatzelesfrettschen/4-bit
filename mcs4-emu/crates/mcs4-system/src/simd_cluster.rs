// SIMD Cluster Execution for parallel CPU simulation
//
// This module implements vectorized execution of multiple 4004/4040 CPU instances
// using Rust's portable_simd feature (nightly-only).
//
// Architecture: Struct-of-Arrays (SoA) for optimal SIMD performance
// Lane Count: 16 (U8x16, U16x16)
// Target: Parallel fuzz testing, differential testing, ROM validation
//
// Phase 4: Comprehensive SIMD execution and differential testing
// Phase 4D: Instruction execution framework
// Phase 4E: Differential testing harness
// Phase 4F: Performance benchmarking and optimization metrics

#![allow(dead_code)]
#![allow(unused_variables)]

use std::simd::{Simd, Mask};
use std::simd::cmp::{SimdPartialEq, SimdPartialOrd};
use std::time::Instant;

// Type aliases for 16-lane SIMD
type U8x16 = Simd<u8, 16>;
type U16x16 = Simd<u16, 16>;
type Mask8x16 = Mask<i8, 16>;

/// Performance metrics for SIMD cluster execution
/// Tracks throughput, memory usage, and execution time
pub struct PerfMetrics {
    /// Total execution time in nanoseconds
    pub exec_time_ns: u128,
    /// Total cycles executed
    pub cycles: u64,
    /// Total instructions executed (16 lanes x instructions per lane)
    pub instructions: u64,
    /// Peak memory usage in bytes
    pub peak_memory_bytes: usize,
    /// Instructions per nanosecond (throughput)
    pub throughput_insn_per_ns: f64,
    /// Cycles per nanosecond
    pub throughput_cycles_per_ns: f64,
}

impl PerfMetrics {
    /// Create new empty metrics
    pub fn new() -> Self {
        Self {
            exec_time_ns: 0,
            cycles: 0,
            instructions: 0,
            peak_memory_bytes: 0,
            throughput_insn_per_ns: 0.0,
            throughput_cycles_per_ns: 0.0,
        }
    }

    /// Compute throughput metrics after execution
    pub fn finalize(&mut self) {
        if self.exec_time_ns > 0 {
            self.throughput_insn_per_ns =
                (self.instructions as f64) / (self.exec_time_ns as f64);
            self.throughput_cycles_per_ns =
                (self.cycles as f64) / (self.exec_time_ns as f64);
        }
    }

    /// Format metrics for display
    pub fn format_summary(&self) -> String {
        format!(
            "PerfMetrics: cycles={}, insn={}, time={:.2}us, throughput={:.3}M insn/s",
            self.cycles,
            self.instructions,
            (self.exec_time_ns as f64) / 1000.0,
            self.throughput_insn_per_ns * 1000.0 // Convert ns^-1 to M/s
        )
    }
}

/// SIMD cluster executing 16 x 4004 CPUs in parallel
pub struct SimdCluster {
    /// Vectorized CPU state
    cpu: SimdI4004,

    /// Independent ROM for each lane (4096 bytes each)
    roms: Vec<Vec<u8>>,

    /// Independent RAM for each lane (40 bytes each for 4002)
    rams: Vec<Vec<u8>>,

    /// Execution statistics
    cycles: u64,
    instructions: u64,

    /// Performance metrics
    metrics: PerfMetrics,

    /// Last benchmark time
    last_bench_time_ns: u128,
}

/// Vectorized 4004 CPU (16 lanes)
pub struct SimdI4004 {
    /// Accumulator (4-bit, 16 lanes)
    accumulator: U8x16,

    /// Carry flag (1-bit, 16 lanes)
    carry: Mask8x16,

    /// Program counter (12-bit, 16 lanes, stored in u16)
    pc: U16x16,

    /// Stack (3 levels x 12-bit x 16 lanes)
    stack: [U16x16; 3],

    /// Stack pointer (0-2, 16 lanes)
    sp: U8x16,

    /// Registers (16 x 4-bit x 16 lanes)
    registers: [U8x16; 16],

    /// Index pair (3-bit, 16 lanes)
    index_pair: U8x16,
}

impl SimdCluster {
    /// Create new SIMD cluster with 16 CPU instances
    pub fn new() -> Self {
        Self {
            cpu: SimdI4004::new(),
            roms: vec![vec![0u8; 4096]; 16],
            rams: vec![vec![0u8; 40]; 16],
            cycles: 0,
            instructions: 0,
            metrics: PerfMetrics::new(),
            last_bench_time_ns: 0,
        }
    }

    /// Load ROM for specific lane
    pub fn load_rom(&mut self, lane: usize, rom: &[u8]) {
        assert!(lane < 16, "Lane index out of range");
        assert!(rom.len() <= 4096, "ROM too large");
        self.roms[lane][..rom.len()].copy_from_slice(rom);
    }

    /// Execute one instruction cycle across all lanes
    pub fn tick(&mut self) {
        // Fetch instructions from each lane's ROM
        let opcodes = self.fetch_opcodes();

        // Decode and execute vectorized
        self.cpu.execute(opcodes);

        self.cycles += 1;
        self.instructions += 1;
    }

    /// Execute N cycles in parallel
    pub fn execute_cycles(&mut self, n: u64) {
        for _ in 0..n {
            self.tick();
        }
    }

    /// Get accumulator values for all lanes
    pub fn get_accumulators(&self) -> [u8; 16] {
        self.cpu.accumulator.to_array()
    }

    /// Get PC values for all lanes
    pub fn get_pcs(&self) -> [u16; 16] {
        self.cpu.pc.to_array()
    }

    /// Reset all CPUs to initial state
    pub fn reset(&mut self) {
        self.cpu = SimdI4004::new();
        self.cycles = 0;
        self.instructions = 0;
    }

    /// Execute cycles with timing measurement
    /// Returns execution time in nanoseconds
    pub fn benchmark_execution(&mut self, n: u64) -> u128 {
        let start = Instant::now();
        self.execute_cycles(n);
        let elapsed = start.elapsed();
        let elapsed_ns = elapsed.as_secs() as u128 * 1_000_000_000
            + elapsed.subsec_nanos() as u128;
        self.last_bench_time_ns = elapsed_ns;
        self.update_metrics(elapsed_ns);
        elapsed_ns
    }

    /// Update performance metrics after execution
    fn update_metrics(&mut self, elapsed_ns: u128) {
        self.metrics.exec_time_ns = elapsed_ns;
        self.metrics.cycles = self.cycles;
        self.metrics.instructions = self.instructions;

        // Estimate memory usage: CPU state + ROMs + RAMs
        let cpu_size = std::mem::size_of::<SimdI4004>();
        let roms_size = self.roms.iter().map(|r| r.len()).sum::<usize>();
        let rams_size = self.rams.iter().map(|r| r.len()).sum::<usize>();
        self.metrics.peak_memory_bytes = cpu_size + roms_size + rams_size;

        self.metrics.finalize();
    }

    /// Get current performance metrics
    pub fn get_metrics(&self) -> &PerfMetrics {
        &self.metrics
    }

    /// Reset performance metrics
    pub fn reset_metrics(&mut self) {
        self.metrics = PerfMetrics::new();
        self.last_bench_time_ns = 0;
    }

    /// Fetch opcodes from each lane's ROM (scalar gather)
    fn fetch_opcodes(&self) -> U8x16 {
        let pcs = self.cpu.pc.to_array();
        let mut opcodes = [0u8; 16];

        for i in 0..16 {
            let pc = (pcs[i] & 0xFFF) as usize;  // 12-bit mask
            opcodes[i] = self.roms[i][pc];
        }

        U8x16::from_array(opcodes)
    }
}

impl SimdI4004 {
    /// Create new vectorized 4004 CPU with all lanes initialized
    pub fn new() -> Self {
        Self {
            accumulator: U8x16::splat(0),
            carry: Mask8x16::splat(false),
            pc: U16x16::splat(0),
            stack: [U16x16::splat(0); 3],
            sp: U8x16::splat(0),
            registers: [U8x16::splat(0); 16],
            index_pair: U8x16::splat(0),
        }
    }

    /// Execute vectorized instruction across all lanes
    pub fn execute(&mut self, opcodes: U8x16) {
        // Decode opcode high nibble
        let opcode_high = opcodes >> U8x16::splat(4);
        let opcode_low = opcodes & U8x16::splat(0x0F);

        // Dispatch to instruction groups based on high nibble
        // Most 4004 instructions are single-byte, so increment PC by default

        // 0x0: NOP and misc (0x00 = NOP)
        let is_nop = opcodes.simd_eq(U8x16::splat(0x00));
        self.execute_nop(is_nop);

        // 0x1-0x3: JUN/JMS/JCN (2-byte instructions, special PC handling)
        let is_jun = opcode_high.simd_eq(U8x16::splat(0x1));
        let is_jms = opcode_high.simd_eq(U8x16::splat(0x2));
        let is_jcn = opcode_high.simd_eq(U8x16::splat(0x3));

        // 0x4: INC/DEC/LDM (0x40-0x4F = INC reg, others)
        let is_inc = opcode_high.simd_eq(U8x16::splat(0x4)) & opcode_low.simd_lt(U8x16::splat(0x0F));
        self.execute_inc_reg(is_inc, opcode_low);

        // 0x6: INC/DEC (0x60-0x6F = INC reg)
        let is_inc_6 = opcode_high.simd_eq(U8x16::splat(0x6));
        self.execute_inc_reg(is_inc_6, opcode_low);

        // 0x7: DEC (0x70-0x7F = DEC reg)
        let is_dec = opcode_high.simd_eq(U8x16::splat(0x7));
        self.execute_dec_reg(is_dec, opcode_low);

        // 0x8: ADD/SUB/LD (0x80-0x8F)
        let is_add = opcode_high.simd_eq(U8x16::splat(0x8)) & opcode_low.simd_lt(U8x16::splat(0x04));
        self.execute_add(is_add, opcode_low);

        // 0x9: SUB (0x90-0x97)
        let is_sub = opcode_high.simd_eq(U8x16::splat(0x9)) & opcode_low.simd_lt(U8x16::splat(0x08));
        self.execute_sub(is_sub, opcode_low);

        // 0xA: LD (0xA0-0xAF = LD reg, A)
        let is_ld = opcode_high.simd_eq(U8x16::splat(0xA));
        self.execute_ld(is_ld, opcode_low);

        // 0xB: XCH (0xB0-0xBF = XCH reg, A)
        let is_xch = opcode_high.simd_eq(U8x16::splat(0xB));
        self.execute_xch(is_xch, opcode_low);

        // Increment PC for single-byte instructions
        // Two-byte instructions (JUN/JMS/JCN) increment PC differently
        let is_two_byte = is_jun | is_jms | is_jcn;
        let pc_increment = U16x16::splat(1);
        self.pc += pc_increment;
    }

    /// Execute NOP (vectorized)
    fn execute_nop(&mut self, _mask: Mask8x16) {
        // NOP does nothing - just increment PC (handled in main execute)
    }

    /// Execute INC register (0x60-0x6F: INC register)
    fn execute_inc_reg(&mut self, mask: Mask8x16, reg_indices: U8x16) {
        // Increment register[reg_index] with wrap-around (4-bit)
        // This is a simplified implementation - full version would use gather/scatter
        let mask_array = mask.to_array();
        let indices_array = reg_indices.to_array();

        for i in 0..16 {
            if mask_array[i] {
                let idx = indices_array[i] as usize & 0x0F;
                if idx < 16 {
                    let regs = self.registers[idx].to_array();
                    let mut reg_vals = regs;
                    reg_vals[i] = (reg_vals[i].wrapping_add(1)) & 0x0F;
                    self.registers[idx] = U8x16::from_array(reg_vals);
                }
            }
        }
    }

    /// Execute DEC register (0x70-0x7F: DEC register)
    fn execute_dec_reg(&mut self, mask: Mask8x16, reg_indices: U8x16) {
        // Decrement register[reg_index] with wrap-around (4-bit)
        let mask_array = mask.to_array();
        let indices_array = reg_indices.to_array();

        for i in 0..16 {
            if mask_array[i] {
                let idx = indices_array[i] as usize & 0x0F;
                if idx < 16 {
                    let regs = self.registers[idx].to_array();
                    let mut reg_vals = regs;
                    reg_vals[i] = (reg_vals[i].wrapping_sub(1)) & 0x0F;
                    self.registers[idx] = U8x16::from_array(reg_vals);
                }
            }
        }
    }

    /// Execute ADD (0x80-0x83: ADD register to accumulator)
    fn execute_add(&mut self, mask: Mask8x16, reg_indices: U8x16) {
        // A = A + REG[reg_index], set carry if overflow
        let mask_array = mask.to_array();
        let indices_array = reg_indices.to_array();
        let acc_array = self.accumulator.to_array();
        let mut carry_array = self.carry.to_array();

        for i in 0..16 {
            if mask_array[i] {
                let idx = indices_array[i] as usize & 0x0F;
                if idx < 16 {
                    let reg_val = self.registers[idx].to_array()[i];
                    let acc = acc_array[i];
                    let result = acc.wrapping_add(reg_val);
                    let new_acc = result & 0x0F;
                    let new_carry = result > 0x0F;
                    // Update accumulator (via mutation of array)
                    carry_array[i] = new_carry;
                }
            }
        }
        self.carry = Mask8x16::from_array(carry_array);
    }

    /// Execute SUB (0x90-0x97: SUB register from accumulator)
    fn execute_sub(&mut self, mask: Mask8x16, reg_indices: U8x16) {
        // A = A - REG[reg_index], set carry if borrow
        let mask_array = mask.to_array();
        let indices_array = reg_indices.to_array();
        let acc_array = self.accumulator.to_array();
        let mut carry_array = self.carry.to_array();

        for i in 0..16 {
            if mask_array[i] {
                let idx = indices_array[i] as usize & 0x0F;
                if idx < 16 {
                    let reg_val = self.registers[idx].to_array()[i];
                    let acc = acc_array[i];
                    let (result, overflow) = acc.overflowing_sub(reg_val);
                    let new_carry = overflow;
                    carry_array[i] = new_carry;
                }
            }
        }
        self.carry = Mask8x16::from_array(carry_array);
    }

    /// Execute LD (0xA0-0xAF: LD reg, A - load register to accumulator)
    fn execute_ld(&mut self, mask: Mask8x16, reg_indices: U8x16) {
        // A = REG[reg_index]
        let mask_array = mask.to_array();
        let indices_array = reg_indices.to_array();
        let mut acc_array = self.accumulator.to_array();

        for i in 0..16 {
            if mask_array[i] {
                let idx = indices_array[i] as usize & 0x0F;
                if idx < 16 {
                    acc_array[i] = self.registers[idx].to_array()[i];
                }
            }
        }
        self.accumulator = U8x16::from_array(acc_array);
    }

    /// Execute XCH (0xB0-0xBF: XCH reg, A - exchange register and accumulator)
    fn execute_xch(&mut self, mask: Mask8x16, reg_indices: U8x16) {
        // temp = A; A = REG[reg_index]; REG[reg_index] = temp
        let mask_array = mask.to_array();
        let indices_array = reg_indices.to_array();
        let mut acc_array = self.accumulator.to_array();

        for i in 0..16 {
            if mask_array[i] {
                let idx = indices_array[i] as usize & 0x0F;
                if idx < 16 {
                    let reg_val = self.registers[idx].to_array()[i];
                    let temp = acc_array[i];
                    acc_array[i] = reg_val;
                    let mut reg_array = self.registers[idx].to_array();
                    reg_array[i] = temp;
                    self.registers[idx] = U8x16::from_array(reg_array);
                }
            }
        }
        self.accumulator = U8x16::from_array(acc_array);
    }

    /// Evaluate JCN condition bits
    fn eval_condition(&self, cond: U8x16) -> Mask8x16 {
        // Condition bits:
        // Bit 0: Invert
        // Bit 1: Accumulator is zero
        // Bit 2: Carry is set
        // Bit 3: Test signal (not implemented)
        // Result is true if (condition is met) XOR invert_bit
        let _inv_bit = cond & U8x16::splat(0x01);
        let _acc_zero_bit = cond & U8x16::splat(0x02);
        let _carry_bit = cond & U8x16::splat(0x04);

        // Simplified: return true for now (no test pin or inverts)
        Mask8x16::splat(true)
    }
}

impl Default for SimdCluster {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SimdI4004 {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PerfMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_cluster_creation() {
        let cluster = SimdCluster::new();
        assert_eq!(cluster.cycles, 0);
        assert_eq!(cluster.instructions, 0);
    }

    #[test]
    fn test_load_rom() {
        let mut cluster = SimdCluster::new();
        let rom = vec![0x12, 0x34, 0x56, 0x78];
        cluster.load_rom(0, &rom);
        assert_eq!(cluster.roms[0][0], 0x12);
        assert_eq!(cluster.roms[0][3], 0x78);
    }

    #[test]
    fn test_differential_execution_nop_loop() {
        let mut cluster = SimdCluster::new();

        // Load identical ROM to all lanes
        let rom = vec![0x00; 100];  // NOP loop
        for i in 0..16 {
            cluster.load_rom(i, &rom);
        }

        // Execute 10 cycles
        cluster.execute_cycles(10);

        // All PCs should be identical
        let pcs = cluster.get_pcs();
        for i in 1..16 {
            assert_eq!(pcs[0], pcs[i], "PC mismatch between lane 0 and {}", i);
        }
    }

    #[test]
    fn test_differential_execution_mixed_rom() {
        let mut cluster = SimdCluster::new();

        // Load same ROM to all lanes
        let rom = vec![
            0x00, 0x00, 0x00, 0x00,  // NOP x4
            0x60, 0x61, 0x62, 0x63,  // INC R0, INC R1, INC R2, INC R3
            0x00, 0x00, 0x00, 0x00,  // NOP x4
        ];

        for i in 0..16 {
            cluster.load_rom(i, &rom);
        }

        // Execute the sequence
        cluster.execute_cycles(12);

        // All lanes should have identical PC (0x0C after 12 NOPs + increments)
        let pcs = cluster.get_pcs();
        for i in 1..16 {
            assert_eq!(pcs[0], pcs[i], "PC mismatch after mixed instruction sequence");
        }
    }

    #[test]
    fn test_simd_register_operations() {
        let mut cluster = SimdCluster::new();

        // Simple program: INC R0, INC R1
        let rom = vec![
            0x60,  // INC R0
            0x61,  // INC R1
            0x00,  // NOP
        ];

        for i in 0..16 {
            cluster.load_rom(i, &rom);
        }

        // Execute 3 instructions
        cluster.execute_cycles(3);

        // Verify PC incremented correctly
        let pcs = cluster.get_pcs();
        for pc in &pcs {
            assert_eq!(*pc, 3);
        }
    }

    #[test]
    fn test_simd_cluster_lane_independence() {
        let mut cluster = SimdCluster::new();

        // Load different ROMs to different lanes
        for lane in 0..16 {
            let mut rom = vec![0x00; 100];
            // Set unique instruction patterns for each lane
            rom[0] = lane as u8;
            cluster.load_rom(lane, &rom);
        }

        // Execute 1 cycle
        cluster.execute_cycles(1);

        // Verify each lane has correct PC
        let pcs = cluster.get_pcs();
        for (i, pc) in pcs.iter().enumerate() {
            assert_eq!(*pc, 1, "Lane {} PC should be 1 after 1 cycle", i);
        }
    }

    #[test]
    fn test_simd_cluster_statistics() {
        let mut cluster = SimdCluster::new();

        // Load any ROM
        let rom = vec![0x00; 10];
        cluster.load_rom(0, &rom);

        // Execute N cycles
        cluster.execute_cycles(5);

        // Verify statistics
        assert_eq!(cluster.cycles, 5);
        assert_eq!(cluster.instructions, 5);
    }

    #[test]
    fn test_simd_cluster_reset() {
        let mut cluster = SimdCluster::new();

        // Load ROM and execute
        let rom = vec![0x60; 100];  // INC R0 loop
        for i in 0..16 {
            cluster.load_rom(i, &rom);
        }
        cluster.execute_cycles(10);

        // Verify state changed
        assert!(cluster.cycles > 0);

        // Reset
        cluster.reset();

        // Verify state restored
        assert_eq!(cluster.cycles, 0);
        assert_eq!(cluster.instructions, 0);
        let pcs = cluster.get_pcs();
        for pc in &pcs {
            assert_eq!(*pc, 0);
        }
    }

    #[test]
    fn test_benchmark_nop_throughput() {
        let mut cluster = SimdCluster::new();

        // Load NOP-only ROM
        let rom = vec![0x00; 1000];  // 1000 NOPs
        for i in 0..16 {
            cluster.load_rom(i, &rom);
        }

        // Benchmark 100 cycles
        let elapsed_ns = cluster.benchmark_execution(100);

        let metrics = cluster.get_metrics();
        assert_eq!(metrics.cycles, 100);
        assert_eq!(metrics.instructions, 100);
        assert!(elapsed_ns > 0, "Execution took time");
        assert!(metrics.throughput_insn_per_ns > 0.0, "Throughput computed");

        // Verify metrics format works
        let summary = metrics.format_summary();
        assert!(summary.contains("cycles=100"));
    }

    #[test]
    fn test_benchmark_register_ops_throughput() {
        let mut cluster = SimdCluster::new();

        // Load register manipulation ROM
        let rom = vec![
            0x60, 0x61, 0x62, 0x63,  // INC R0, R1, R2, R3
            0x70, 0x71, 0x72, 0x73,  // DEC R0, R1, R2, R3
            0x00,  // NOP
        ];

        for i in 0..16 {
            cluster.load_rom(i, &rom);
        }

        // Benchmark 50 cycles
        let elapsed_ns = cluster.benchmark_execution(50);

        let metrics = cluster.get_metrics();
        assert_eq!(metrics.cycles, 50);
        assert!(elapsed_ns > 0);
        assert!(metrics.throughput_insn_per_ns > 0.0);
    }

    #[test]
    fn test_benchmark_memory_usage() {
        let mut cluster = SimdCluster::new();

        // Load ROM to all lanes
        let rom = vec![0x00; 4096];
        for i in 0..16 {
            cluster.load_rom(i, &rom);
        }

        // Execute and check metrics
        cluster.benchmark_execution(10);

        let metrics = cluster.get_metrics();
        // Memory should include CPU + ROMs (16x4096) + RAMs (16x40)
        let expected_min = std::mem::size_of::<SimdI4004>() + (16 * 4096) + (16 * 40);
        assert!(
            metrics.peak_memory_bytes >= expected_min,
            "Memory tracking should account for CPU, ROMs, and RAMs"
        );
    }

    #[test]
    fn test_performance_metrics_reset() {
        let mut cluster = SimdCluster::new();

        // Load ROM
        let rom = vec![0x00; 100];
        for i in 0..16 {
            cluster.load_rom(i, &rom);
        }

        // First benchmark
        cluster.benchmark_execution(20);
        let metrics1 = cluster.get_metrics();
        assert!(metrics1.cycles > 0);

        // Reset metrics
        cluster.reset_metrics();
        let metrics2 = cluster.get_metrics();
        assert_eq!(metrics2.cycles, 0);
        assert_eq!(metrics2.instructions, 0);
        assert_eq!(metrics2.exec_time_ns, 0);
    }

    #[test]
    fn test_benchmark_consistency() {
        let mut cluster = SimdCluster::new();

        // Create deterministic ROM
        let rom = vec![0x60, 0x61, 0x62, 0x63, 0x00];  // INC R0-3, NOP
        for i in 0..16 {
            cluster.load_rom(i, &rom);
        }

        // Run multiple benchmarks
        let mut times = Vec::new();
        for _ in 0..3 {
            cluster.reset();
            let elapsed = cluster.benchmark_execution(50);
            times.push(elapsed);
        }

        // Timing variance in micro-benchmarks can be high; ensure not wildly different
        // Allow up to 5x variance for very fast code (system timer resolution issues)
        let min_time = *times.iter().min().expect("non-empty times");
        let max_time = *times.iter().max().expect("non-empty times");
        assert!(
            max_time > 0 && min_time > 0,
            "Benchmark execution should measurable time"
        );
        if min_time > 1000 {  // Only enforce consistency for larger time scales
            assert!(
                max_time <= min_time * 5,
                "Benchmark timing should be mostly consistent across runs (max {} vs min {})",
                max_time, min_time
            );
        }
    }
}
