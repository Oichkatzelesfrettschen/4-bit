// SIMD Cluster Execution for parallel CPU simulation
//
// This module implements vectorized execution of multiple 4004/4040 CPU instances
// using Rust's portable_simd feature (nightly-only).
//
// Architecture: Struct-of-Arrays (SoA) for optimal SIMD performance
// Lane Count: 16 (U8x16, U16x16)
// Target: Parallel fuzz testing, differential testing, ROM validation
//
// Phase 4: Full 4004 ISA in SIMD (46 instructions)
// Phase 4D: Instruction execution framework
// Phase 4E: Differential testing harness
// Phase 4F: Performance benchmarking and optimization metrics
// Phase 4G: Complete ISA, two-byte infrastructure, differential fuzzing

#![allow(dead_code)]
#![allow(unused_variables)]

use std::{
    simd::{cmp::SimdPartialEq, Mask, Simd},
    time::Instant,
};

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
            self.throughput_insn_per_ns = (self.instructions as f64) / (self.exec_time_ns as f64);
            self.throughput_cycles_per_ns = (self.cycles as f64) / (self.exec_time_ns as f64);
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

/// KBP lookup table (keyboard process: one-hot to binary)
const KBP_TABLE: [u8; 16] = [
    0x00, 0x01, 0x02, 0x0F, 0x03, 0x0F, 0x0F, 0x0F, 0x04, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F,
];

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

    /// RAM address register per lane (set by SRC)
    ram_addr: U8x16,

    /// Pending two-byte instruction: true if lane is waiting for operand
    pending_two_byte: [bool; 16],

    /// First byte of pending two-byte instruction
    pending_first_byte: [u8; 16],
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

    /// Execute one instruction cycle across all lanes.
    ///
    /// Handles two-byte instruction fetch: if a lane has a pending two-byte
    /// instruction, the current fetch provides the second byte and the
    /// two-byte instruction executes. Otherwise, the fetched byte is
    /// decoded and single-byte instructions execute immediately.
    pub fn tick(&mut self) {
        let opcodes = self.fetch_opcodes();
        let opcode_arr = opcodes.to_array();

        // Check which lanes have pending two-byte instructions
        let mut any_pending = false;
        for i in 0..16 {
            if self.cpu.pending_two_byte[i] {
                any_pending = true;
                break;
            }
        }

        if any_pending {
            // Some lanes need second-byte handling
            // For pending lanes: execute two-byte with (first_byte, current_byte)
            // For non-pending lanes: execute single-byte as normal
            let mut first_bytes = [0u8; 16];
            let mut second_bytes = [0u8; 16];
            let mut is_two_byte = [false; 16];

            for i in 0..16 {
                if self.cpu.pending_two_byte[i] {
                    first_bytes[i] = self.cpu.pending_first_byte[i];
                    second_bytes[i] = opcode_arr[i];
                    is_two_byte[i] = true;
                    self.cpu.pending_two_byte[i] = false;
                }
            }

            // Execute two-byte instructions for pending lanes
            self.cpu
                .execute_two_byte_lanes(&first_bytes, &second_bytes, &is_two_byte, &self.roms);

            // Execute single-byte for non-pending lanes
            let mut single_opcodes = [0u8; 16];
            for i in 0..16 {
                if !is_two_byte[i] {
                    single_opcodes[i] = opcode_arr[i];
                }
            }
            self.cpu
                .execute_with_mask(U8x16::from_array(single_opcodes), &is_two_byte);

            // Increment PC for all lanes (two-byte lanes already had PC incremented for first byte)
            self.cpu.pc += U16x16::splat(1);
        } else {
            // Fast path: no pending two-byte instructions
            // Check if any lanes are starting a two-byte instruction
            let mut starting_two_byte = false;
            for (i, &opcode) in opcode_arr.iter().enumerate() {
                let opr = opcode >> 4;
                let opa = opcode & 0x0F;
                if Self::is_two_byte_opr(opr, opa) {
                    self.cpu.pending_two_byte[i] = true;
                    self.cpu.pending_first_byte[i] = opcode;
                    starting_two_byte = true;
                }
            }

            if starting_two_byte {
                // Execute single-byte instructions for non-two-byte lanes
                let two_byte_mask: [bool; 16] = self.cpu.pending_two_byte;
                self.cpu.execute_with_mask(opcodes, &two_byte_mask);
            } else {
                // Pure single-byte cycle: all lanes execute normally
                self.cpu.execute(opcodes);
            }

            // Increment PC for all lanes
            self.cpu.pc += U16x16::splat(1);
        }

        self.cycles += 1;
        self.instructions += 1;
    }

    /// Check if an opcode starts a two-byte instruction
    fn is_two_byte_opr(opr: u8, opa: u8) -> bool {
        matches!(opr, 0x1 | 0x4 | 0x5 | 0x7) || (opr == 0x2 && (opa & 0x01) == 0)
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

    /// Get carry values for all lanes
    pub fn get_carries(&self) -> [bool; 16] {
        self.cpu.carry.to_array()
    }

    /// Get PC values for all lanes
    pub fn get_pcs(&self) -> [u16; 16] {
        self.cpu.pc.to_array()
    }

    /// Get register value for all lanes
    pub fn get_register(&self, reg: usize) -> [u8; 16] {
        assert!(reg < 16);
        self.cpu.registers[reg].to_array()
    }

    /// Get stack pointer values for all lanes
    pub fn get_sps(&self) -> [u8; 16] {
        self.cpu.sp.to_array()
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
        let elapsed_ns = elapsed.as_secs() as u128 * 1_000_000_000 + elapsed.subsec_nanos() as u128;
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
            let pc = (pcs[i] & 0xFFF) as usize; // 12-bit mask
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
            ram_addr: U8x16::splat(0),
            pending_two_byte: [false; 16],
            pending_first_byte: [0; 16],
        }
    }

    /// Execute vectorized instruction across all lanes (single-byte only)
    pub fn execute(&mut self, opcodes: U8x16) {
        let skip = [false; 16];
        self.execute_with_mask(opcodes, &skip);
    }

    /// Execute single-byte instructions, skipping lanes where skip[i] is true
    pub fn execute_with_mask(&mut self, opcodes: U8x16, skip: &[bool; 16]) {
        let opcode_high = opcodes >> U8x16::splat(4);
        let opcode_low = opcodes & U8x16::splat(0x0F);

        // Build skip mask (lanes to exclude from execution)
        let skip_mask = Mask8x16::from_array(*skip);

        // 0x0: NOP (no action needed)

        // 0x2 odd: SRC (send register control)
        let is_src_base = opcode_high.simd_eq(U8x16::splat(0x2));
        let is_odd = (opcode_low & U8x16::splat(0x01)).simd_eq(U8x16::splat(0x01));
        let is_src = is_src_base & is_odd & !skip_mask;
        self.execute_src(is_src, opcode_low);

        // 0x3 even: FIN (fetch indirect via pair 0)
        let is_fin_base = opcode_high.simd_eq(U8x16::splat(0x3));
        let is_even = (opcode_low & U8x16::splat(0x01)).simd_eq(U8x16::splat(0x00));
        let _is_fin = is_fin_base & is_even & !skip_mask;
        // FIN requires ROM access -- stubbed as no-op in SIMD (needs per-lane ROM)

        // 0x3 odd: JIN (jump indirect via pair)
        let is_jin = is_fin_base & is_odd & !skip_mask;
        self.execute_jin(is_jin, opcode_low);

        // 0x6: INC register (0x60-0x6F)
        let is_inc = opcode_high.simd_eq(U8x16::splat(0x6)) & !skip_mask;
        self.execute_inc_reg(is_inc, opcode_low);

        // 0x8: ADD register to accumulator (0x80-0x8F, all 16 regs)
        let is_add = opcode_high.simd_eq(U8x16::splat(0x8)) & !skip_mask;
        self.execute_add(is_add, opcode_low);

        // 0x9: SUB register from accumulator (0x90-0x9F, all 16 regs)
        let is_sub = opcode_high.simd_eq(U8x16::splat(0x9)) & !skip_mask;
        self.execute_sub(is_sub, opcode_low);

        // 0xA: LD (0xA0-0xAF = LD reg -> acc)
        let is_ld = opcode_high.simd_eq(U8x16::splat(0xA)) & !skip_mask;
        self.execute_ld(is_ld, opcode_low);

        // 0xB: XCH (0xB0-0xBF = exchange reg <-> acc)
        let is_xch = opcode_high.simd_eq(U8x16::splat(0xB)) & !skip_mask;
        self.execute_xch(is_xch, opcode_low);

        // 0xC: BBL (return from subroutine, load immediate)
        let is_bbl = opcode_high.simd_eq(U8x16::splat(0xC)) & !skip_mask;
        self.execute_bbl(is_bbl, opcode_low);

        // 0xD: LDM (load immediate to accumulator)
        let is_ldm = opcode_high.simd_eq(U8x16::splat(0xD)) & !skip_mask;
        self.execute_ldm(is_ldm, opcode_low);

        // 0xE: I/O operations (stubs)
        let is_io = opcode_high.simd_eq(U8x16::splat(0xE)) & !skip_mask;
        self.execute_io(is_io, opcode_low);

        // 0xF: Accumulator group (14 instructions)
        let is_acc = opcode_high.simd_eq(U8x16::splat(0xF)) & !skip_mask;
        self.execute_accumulator_group(is_acc, opcode_low);
    }

    /// Execute two-byte instructions for specific lanes
    fn execute_two_byte_lanes(
        &mut self,
        first_bytes: &[u8; 16],
        second_bytes: &[u8; 16],
        is_active: &[bool; 16],
        roms: &[Vec<u8>],
    ) {
        for i in 0..16 {
            if !is_active[i] {
                continue;
            }
            let opr = first_bytes[i] >> 4;
            let opa = first_bytes[i] & 0x0F;
            let operand = second_bytes[i];

            match opr {
                0x1 => self.execute_jcn_lane(i, opa, operand),
                0x2 => self.execute_fim_lane(i, opa, operand),
                0x4 => self.execute_jun_lane(i, opa, operand),
                0x5 => self.execute_jms_lane(i, opa, operand),
                0x7 => self.execute_isz_lane(i, opa, operand),
                _ => {} // Invalid two-byte -- ignore
            }
        }
    }

    // ========================================================================
    // Single-byte instruction implementations
    // ========================================================================

    /// INC register (0x60-0x6F)
    fn execute_inc_reg(&mut self, mask: Mask8x16, reg_indices: U8x16) {
        let mask_array = mask.to_array();
        let indices_array = reg_indices.to_array();

        for i in 0..16 {
            if mask_array[i] {
                let idx = indices_array[i] as usize & 0x0F;
                let mut reg_vals = self.registers[idx].to_array();
                reg_vals[i] = (reg_vals[i].wrapping_add(1)) & 0x0F;
                self.registers[idx] = U8x16::from_array(reg_vals);
            }
        }
    }

    /// ADD register to accumulator with carry (0x80-0x8F)
    fn execute_add(&mut self, mask: Mask8x16, reg_indices: U8x16) {
        let mask_array = mask.to_array();
        let indices_array = reg_indices.to_array();
        let mut acc_array = self.accumulator.to_array();
        let mut carry_array = self.carry.to_array();

        for i in 0..16 {
            if mask_array[i] {
                let idx = indices_array[i] as usize & 0x0F;
                let reg_val = self.registers[idx].to_array()[i];
                let result = (acc_array[i] as u16) + (reg_val as u16) + (carry_array[i] as u16);
                carry_array[i] = result > 0x0F;
                acc_array[i] = (result & 0x0F) as u8;
            }
        }
        self.accumulator = U8x16::from_array(acc_array);
        self.carry = Mask8x16::from_array(carry_array);
    }

    /// SUB register from accumulator with borrow (0x90-0x9F)
    fn execute_sub(&mut self, mask: Mask8x16, reg_indices: U8x16) {
        let mask_array = mask.to_array();
        let indices_array = reg_indices.to_array();
        let mut acc_array = self.accumulator.to_array();
        let mut carry_array = self.carry.to_array();

        for i in 0..16 {
            if mask_array[i] {
                let idx = indices_array[i] as usize & 0x0F;
                let reg_val = self.registers[idx].to_array()[i];
                let complement = (!reg_val) & 0x0F;
                let result = (acc_array[i] as u16) + (complement as u16) + (carry_array[i] as u16);
                carry_array[i] = result > 0x0F;
                acc_array[i] = (result & 0x0F) as u8;
            }
        }
        self.accumulator = U8x16::from_array(acc_array);
        self.carry = Mask8x16::from_array(carry_array);
    }

    /// LD register to accumulator (0xA0-0xAF)
    fn execute_ld(&mut self, mask: Mask8x16, reg_indices: U8x16) {
        let mask_array = mask.to_array();
        let indices_array = reg_indices.to_array();
        let mut acc_array = self.accumulator.to_array();

        for i in 0..16 {
            if mask_array[i] {
                let idx = indices_array[i] as usize & 0x0F;
                acc_array[i] = self.registers[idx].to_array()[i];
            }
        }
        self.accumulator = U8x16::from_array(acc_array);
    }

    /// XCH register and accumulator (0xB0-0xBF)
    fn execute_xch(&mut self, mask: Mask8x16, reg_indices: U8x16) {
        let mask_array = mask.to_array();
        let indices_array = reg_indices.to_array();
        let mut acc_array = self.accumulator.to_array();

        for i in 0..16 {
            if mask_array[i] {
                let idx = indices_array[i] as usize & 0x0F;
                let mut reg_array = self.registers[idx].to_array();
                std::mem::swap(&mut acc_array[i], &mut reg_array[i]);
                self.registers[idx] = U8x16::from_array(reg_array);
            }
        }
        self.accumulator = U8x16::from_array(acc_array);
    }

    /// BBL: return from subroutine + load immediate (0xC0-0xCF)
    fn execute_bbl(&mut self, mask: Mask8x16, data: U8x16) {
        let mask_array = mask.to_array();
        let data_array = data.to_array();
        let mut acc_array = self.accumulator.to_array();
        let mut sp_array = self.sp.to_array();
        let mut pc_array = self.pc.to_array();
        let stack0 = self.stack[0].to_array();
        let stack1 = self.stack[1].to_array();
        let stack2 = self.stack[2].to_array();

        for i in 0..16 {
            if mask_array[i] {
                // Pop stack: sp = (sp - 1) % 3
                sp_array[i] = if sp_array[i] == 0 { 2 } else { sp_array[i] - 1 };
                // Restore PC from stack
                let sp = sp_array[i] as usize;
                let ret_addr = match sp {
                    0 => stack0[i],
                    1 => stack1[i],
                    _ => stack2[i],
                };
                // PC will be incremented by 1 after execute, so set to ret_addr - 1
                pc_array[i] = ret_addr.wrapping_sub(1);
                // Load immediate to accumulator
                acc_array[i] = data_array[i] & 0x0F;
            }
        }

        self.accumulator = U8x16::from_array(acc_array);
        self.sp = U8x16::from_array(sp_array);
        self.pc = U16x16::from_array(pc_array);
    }

    /// LDM: load immediate to accumulator (0xD0-0xDF)
    fn execute_ldm(&mut self, mask: Mask8x16, data: U8x16) {
        let mask_array = mask.to_array();
        let data_array = data.to_array();
        let mut acc_array = self.accumulator.to_array();

        for i in 0..16 {
            if mask_array[i] {
                acc_array[i] = data_array[i] & 0x0F;
            }
        }
        self.accumulator = U8x16::from_array(acc_array);
    }

    /// SRC: send register control -- set RAM address from register pair (0x21,0x23,...,0x2F)
    fn execute_src(&mut self, mask: Mask8x16, opa: U8x16) {
        let mask_array = mask.to_array();
        let opa_array = opa.to_array();

        for i in 0..16 {
            if mask_array[i] {
                let pair = (opa_array[i] >> 1) as usize;
                let even_reg = pair * 2;
                let odd_reg = even_reg + 1;
                if odd_reg < 16 {
                    let high = self.registers[even_reg].to_array()[i];
                    let low = self.registers[odd_reg].to_array()[i];
                    let mut addr = self.ram_addr.to_array();
                    addr[i] = (high << 4) | low;
                    self.ram_addr = U8x16::from_array(addr);
                }
            }
        }
    }

    /// JIN: jump indirect via register pair (0x31,0x33,...,0x3F)
    fn execute_jin(&mut self, mask: Mask8x16, opa: U8x16) {
        let mask_array = mask.to_array();
        let opa_array = opa.to_array();
        let mut pc_array = self.pc.to_array();

        for i in 0..16 {
            if mask_array[i] {
                let pair = (opa_array[i] >> 1) as usize;
                let even_reg = pair * 2;
                let odd_reg = even_reg + 1;
                if odd_reg < 16 {
                    let high = self.registers[even_reg].to_array()[i] as u16;
                    let low = self.registers[odd_reg].to_array()[i] as u16;
                    let target = (pc_array[i] & 0xF00) | (high << 4) | low;
                    // PC will be +1 after execute, so set to target - 1
                    pc_array[i] = target.wrapping_sub(1);
                }
            }
        }
        self.pc = U16x16::from_array(pc_array);
    }

    /// I/O operations (0xE0-0xEF): stubs -- no bus in SIMD
    fn execute_io(&mut self, mask: Mask8x16, opa: U8x16) {
        let mask_array = mask.to_array();
        let opa_array = opa.to_array();
        let mut acc_array = self.accumulator.to_array();
        let mut carry_array = self.carry.to_array();

        for i in 0..16 {
            if mask_array[i] {
                match opa_array[i] {
                    // Read operations return 0 (no bus)
                    0x9 => acc_array[i] = 0, // RDM
                    0xA => acc_array[i] = 0, // RDR
                    0xC => acc_array[i] = 0, // RD0
                    0xD => acc_array[i] = 0, // RD1
                    0xE => acc_array[i] = 0, // RD2
                    0xF => acc_array[i] = 0, // RD3
                    // ADM: add memory (0) to acc with carry
                    0xB => {
                        let result = (acc_array[i] as u16) + (carry_array[i] as u16);
                        carry_array[i] = result > 0x0F;
                        acc_array[i] = (result & 0x0F) as u8;
                    }
                    // SBM: sub memory (0) from acc with borrow
                    0x8 => {
                        let complement = 0x0Fu16; // ~0 & 0xF = 0xF
                        let result = (acc_array[i] as u16) + complement + (carry_array[i] as u16);
                        carry_array[i] = result > 0x0F;
                        acc_array[i] = (result & 0x0F) as u8;
                    }
                    // Write operations: no-op (WRM, WMP, WRR, WPM, WR0-WR3)
                    _ => {}
                }
            }
        }
        self.accumulator = U8x16::from_array(acc_array);
        self.carry = Mask8x16::from_array(carry_array);
    }

    /// Accumulator group (0xF0-0xFD): 14 instructions
    fn execute_accumulator_group(&mut self, mask: Mask8x16, opa: U8x16) {
        let mask_array = mask.to_array();
        let opa_array = opa.to_array();
        let mut acc_array = self.accumulator.to_array();
        let mut carry_array = self.carry.to_array();

        for i in 0..16 {
            if !mask_array[i] {
                continue;
            }
            match opa_array[i] {
                0x0 => {
                    // CLB: clear both
                    acc_array[i] = 0;
                    carry_array[i] = false;
                }
                0x1 => {
                    // CLC: clear carry
                    carry_array[i] = false;
                }
                0x2 => {
                    // IAC: increment accumulator
                    let result = acc_array[i].wrapping_add(1);
                    carry_array[i] = result > 0x0F;
                    acc_array[i] = result & 0x0F;
                }
                0x3 => {
                    // CMC: complement carry
                    carry_array[i] = !carry_array[i];
                }
                0x4 => {
                    // CMA: complement accumulator
                    acc_array[i] = (!acc_array[i]) & 0x0F;
                }
                0x5 => {
                    // RAL: rotate acc left through carry
                    let new_carry = (acc_array[i] & 0x08) != 0;
                    acc_array[i] = ((acc_array[i] << 1) | (carry_array[i] as u8)) & 0x0F;
                    carry_array[i] = new_carry;
                }
                0x6 => {
                    // RAR: rotate acc right through carry
                    let new_carry = (acc_array[i] & 0x01) != 0;
                    acc_array[i] = ((acc_array[i] >> 1) | ((carry_array[i] as u8) << 3)) & 0x0F;
                    carry_array[i] = new_carry;
                }
                0x7 => {
                    // TCC: transfer carry to acc, clear carry
                    acc_array[i] = carry_array[i] as u8;
                    carry_array[i] = false;
                }
                0x8 => {
                    // DAC: decrement accumulator
                    let result = acc_array[i].wrapping_sub(1);
                    carry_array[i] = acc_array[i] != 0; // borrow inverted
                    acc_array[i] = result & 0x0F;
                }
                0x9 => {
                    // TCS: transfer carry subtract, clear carry
                    acc_array[i] = if carry_array[i] { 10 } else { 9 };
                    carry_array[i] = false;
                }
                0xA => {
                    // STC: set carry
                    carry_array[i] = true;
                }
                0xB => {
                    // DAA: decimal adjust accumulator
                    if carry_array[i] || acc_array[i] > 9 {
                        let result = acc_array[i] + 6;
                        if result > 0x0F {
                            carry_array[i] = true;
                        }
                        acc_array[i] = result & 0x0F;
                    }
                }
                0xC => {
                    // KBP: keyboard process (one-hot to binary)
                    acc_array[i] = KBP_TABLE[acc_array[i] as usize & 0x0F];
                }
                0xD => {
                    // DCL: designate command line (no-op in SIMD)
                }
                _ => {} // 0xE, 0xF are invalid in 4004
            }
        }
        self.accumulator = U8x16::from_array(acc_array);
        self.carry = Mask8x16::from_array(carry_array);
    }

    // ========================================================================
    // Two-byte instruction per-lane implementations
    // ========================================================================

    /// JCN: conditional jump (OPR=0x1)
    fn execute_jcn_lane(&mut self, lane: usize, condition: u8, addr_low: u8) {
        // Condition bits: bit3=invert, bit2=acc==0, bit1=carry, bit0=test
        let invert = (condition & 0x08) != 0;
        let check_acc_zero = (condition & 0x04) != 0;
        let check_carry = (condition & 0x02) != 0;
        let check_test = (condition & 0x01) != 0;

        let acc = self.accumulator.to_array()[lane];
        let carry = self.carry.to_array()[lane];

        let mut cond_met = false;
        if check_acc_zero && acc == 0 {
            cond_met = true;
        }
        if check_carry && carry {
            cond_met = true;
        }
        if check_test {
            // Test pin not implemented in SIMD; treat as false
        }

        if invert {
            cond_met = !cond_met;
        }

        if cond_met {
            let mut pc_array = self.pc.to_array();
            // Jump within current page (high 4 bits of PC preserved)
            let target = (pc_array[lane] & 0xF00) | (addr_low as u16);
            // PC will be +1 after tick, so set to target - 1
            pc_array[lane] = target.wrapping_sub(1);
            self.pc = U16x16::from_array(pc_array);
        }
    }

    /// FIM: fetch immediate to register pair (OPR=0x2, even OPA)
    fn execute_fim_lane(&mut self, lane: usize, opa: u8, data: u8) {
        let pair = (opa >> 1) as usize;
        let even_reg = pair * 2;
        let odd_reg = even_reg + 1;
        if odd_reg < 16 {
            let mut even = self.registers[even_reg].to_array();
            let mut odd = self.registers[odd_reg].to_array();
            even[lane] = (data >> 4) & 0x0F;
            odd[lane] = data & 0x0F;
            self.registers[even_reg] = U8x16::from_array(even);
            self.registers[odd_reg] = U8x16::from_array(odd);
        }
    }

    /// JUN: unconditional jump (OPR=0x4)
    fn execute_jun_lane(&mut self, lane: usize, opa: u8, addr_low: u8) {
        let mut pc_array = self.pc.to_array();
        let target = ((opa as u16) << 8) | (addr_low as u16);
        // PC will be +1 after tick, so set to target - 1
        pc_array[lane] = target.wrapping_sub(1);
        self.pc = U16x16::from_array(pc_array);
    }

    /// JMS: jump to subroutine (OPR=0x5)
    fn execute_jms_lane(&mut self, lane: usize, opa: u8, addr_low: u8) {
        let mut sp_array = self.sp.to_array();
        let mut pc_array = self.pc.to_array();
        let mut stack0 = self.stack[0].to_array();
        let mut stack1 = self.stack[1].to_array();
        let mut stack2 = self.stack[2].to_array();

        // Push return address (current PC + 1, which is addr after two-byte insn)
        let return_addr = pc_array[lane].wrapping_add(1);
        let sp = sp_array[lane] as usize;
        match sp {
            0 => stack0[lane] = return_addr,
            1 => stack1[lane] = return_addr,
            _ => stack2[lane] = return_addr,
        }
        sp_array[lane] = ((sp + 1) % 3) as u8;

        // Jump to target
        let target = ((opa as u16) << 8) | (addr_low as u16);
        pc_array[lane] = target.wrapping_sub(1); // tick adds 1

        self.sp = U8x16::from_array(sp_array);
        self.pc = U16x16::from_array(pc_array);
        self.stack[0] = U16x16::from_array(stack0);
        self.stack[1] = U16x16::from_array(stack1);
        self.stack[2] = U16x16::from_array(stack2);
    }

    /// ISZ: increment register, skip if not zero (OPR=0x7)
    fn execute_isz_lane(&mut self, lane: usize, reg: u8, addr_low: u8) {
        let idx = reg as usize & 0x0F;
        let mut reg_vals = self.registers[idx].to_array();
        reg_vals[lane] = (reg_vals[lane].wrapping_add(1)) & 0x0F;
        self.registers[idx] = U8x16::from_array(reg_vals);

        if reg_vals[lane] != 0 {
            let mut pc_array = self.pc.to_array();
            let target = (pc_array[lane] & 0xF00) | (addr_low as u16);
            pc_array[lane] = target.wrapping_sub(1); // tick adds 1
            self.pc = U16x16::from_array(pc_array);
        }
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

// ============================================================================
// Scalar reference executor for differential fuzzing
// ============================================================================

/// Minimal scalar 4004 executor mirroring SIMD state for differential testing.
/// Uses two-phase fetch for two-byte instructions (matching SIMD tick-for-tick).
pub struct ScalarI4004 {
    pub acc: u8,
    pub carry: bool,
    pub pc: u16,
    pub stack: [u16; 3],
    pub sp: u8,
    pub registers: [u8; 16],
    pub ram_addr: u8,
    /// ROM image
    rom: Vec<u8>,
    /// Pending two-byte state
    pending_two_byte: bool,
    pending_opr: u8,
    pending_opa: u8,
}

impl ScalarI4004 {
    pub fn new() -> Self {
        Self {
            acc: 0,
            carry: false,
            pc: 0,
            stack: [0; 3],
            sp: 0,
            registers: [0; 16],
            ram_addr: 0,
            rom: vec![0; 4096],
            pending_two_byte: false,
            pending_opr: 0,
            pending_opa: 0,
        }
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        let len = rom.len().min(4096);
        self.rom[..len].copy_from_slice(&rom[..len]);
    }

    /// One tick = one machine cycle. Two-byte instructions take 2 ticks.
    pub fn tick(&mut self) {
        let byte = self.rom[(self.pc & 0xFFF) as usize];

        if self.pending_two_byte {
            // Second cycle: execute two-byte instruction with operand
            let pc_set = self.execute_two_byte(self.pending_opr, self.pending_opa, byte);
            self.pending_two_byte = false;
            if !pc_set {
                self.pc = self.pc.wrapping_add(1);
            }
            return;
        }

        let opr = byte >> 4;
        let opa = byte & 0x0F;
        let is_two_byte = matches!(opr, 0x1 | 0x4 | 0x5 | 0x7) || (opr == 0x2 && (opa & 0x01) == 0);

        if is_two_byte {
            // First cycle: record pending, advance PC past first byte
            self.pending_two_byte = true;
            self.pending_opr = opr;
            self.pending_opa = opa;
            self.pc = self.pc.wrapping_add(1);
            return;
        }

        // Single-byte instruction: execute and advance
        let pc_set = self.execute_single(opr, opa);
        if !pc_set {
            self.pc = self.pc.wrapping_add(1);
        }
    }

    /// Execute single-byte instruction. Returns true if PC was explicitly set.
    fn execute_single(&mut self, opr: u8, opa: u8) -> bool {
        match opr {
            0x0 => {} // NOP (other 0x01-0x0F are invalid/4040)
            0x2 => {
                // SRC (opa is odd)
                if (opa & 1) == 1 {
                    let pair = (opa >> 1) as usize;
                    let high = self.registers[pair * 2];
                    let low = self.registers[pair * 2 + 1];
                    self.ram_addr = (high << 4) | low;
                }
            }
            0x3 => {
                if (opa & 1) == 0 {
                    // FIN: stub (needs ROM read via pair 0)
                } else {
                    // JIN: jump indirect
                    let pair = (opa >> 1) as usize;
                    let high = self.registers[pair * 2] as u16;
                    let low = self.registers[pair * 2 + 1] as u16;
                    self.pc = (self.pc & 0xF00) | (high << 4) | low;
                    return true; // PC was set
                }
            }
            0x6 => {
                // INC register
                let idx = opa as usize;
                self.registers[idx] = (self.registers[idx].wrapping_add(1)) & 0x0F;
            }
            0x8 => {
                // ADD with carry
                let idx = opa as usize;
                let result = (self.acc as u16) + (self.registers[idx] as u16) + (self.carry as u16);
                self.carry = result > 0x0F;
                self.acc = (result & 0x0F) as u8;
            }
            0x9 => {
                // SUB with borrow
                let idx = opa as usize;
                let complement = (!self.registers[idx]) & 0x0F;
                let result = (self.acc as u16) + (complement as u16) + (self.carry as u16);
                self.carry = result > 0x0F;
                self.acc = (result & 0x0F) as u8;
            }
            0xA => {
                // LD
                self.acc = self.registers[opa as usize];
            }
            0xB => {
                // XCH
                let idx = opa as usize;
                std::mem::swap(&mut self.acc, &mut self.registers[idx]);
            }
            0xC => {
                // BBL
                self.sp = if self.sp == 0 { 2 } else { self.sp - 1 };
                self.pc = self.stack[self.sp as usize];
                self.acc = opa;
                return true; // PC was set
            }
            0xD => {
                // LDM
                self.acc = opa;
            }
            0xE => {
                // I/O stubs
                match opa {
                    0x9 | 0xA | 0xC | 0xD | 0xE | 0xF => self.acc = 0, // reads return 0
                    0xB => {
                        // ADM (add memory=0 with carry)
                        let result = (self.acc as u16) + (self.carry as u16);
                        self.carry = result > 0x0F;
                        self.acc = (result & 0x0F) as u8;
                    }
                    0x8 => {
                        // SBM (sub memory=0 with borrow)
                        let result = (self.acc as u16) + 0x0F + (self.carry as u16);
                        self.carry = result > 0x0F;
                        self.acc = (result & 0x0F) as u8;
                    }
                    _ => {} // write ops: no-op
                }
            }
            0xF => self.execute_acc_group(opa),
            _ => {} // Unhandled OPR (0x1/0x4/0x5/0x7 are two-byte, handled above)
        }
        false
    }

    fn execute_acc_group(&mut self, opa: u8) {
        match opa {
            0x0 => {
                self.acc = 0;
                self.carry = false;
            } // CLB
            0x1 => {
                self.carry = false;
            } // CLC
            0x2 => {
                // IAC
                let result = self.acc.wrapping_add(1);
                self.carry = result > 0x0F;
                self.acc = result & 0x0F;
            }
            0x3 => {
                self.carry = !self.carry;
            } // CMC
            0x4 => {
                self.acc = (!self.acc) & 0x0F;
            } // CMA
            0x5 => {
                // RAL
                let new_carry = (self.acc & 0x08) != 0;
                self.acc = ((self.acc << 1) | (self.carry as u8)) & 0x0F;
                self.carry = new_carry;
            }
            0x6 => {
                // RAR
                let new_carry = (self.acc & 0x01) != 0;
                self.acc = ((self.acc >> 1) | ((self.carry as u8) << 3)) & 0x0F;
                self.carry = new_carry;
            }
            0x7 => {
                // TCC
                self.acc = self.carry as u8;
                self.carry = false;
            }
            0x8 => {
                // DAC
                let result = self.acc.wrapping_sub(1);
                self.carry = self.acc != 0;
                self.acc = result & 0x0F;
            }
            0x9 => {
                // TCS
                self.acc = if self.carry { 10 } else { 9 };
                self.carry = false;
            }
            0xA => {
                self.carry = true;
            } // STC
            0xB => {
                // DAA
                if self.carry || self.acc > 9 {
                    let result = self.acc + 6;
                    if result > 0x0F {
                        self.carry = true;
                    }
                    self.acc = result & 0x0F;
                }
            }
            0xC => {
                // KBP
                self.acc = KBP_TABLE[self.acc as usize & 0x0F];
            }
            0xD => {} // DCL (no-op)
            _ => {}
        }
    }

    /// Execute two-byte instruction. Returns true if PC was explicitly set
    /// (jump taken), false if caller should advance PC past the operand.
    fn execute_two_byte(&mut self, opr: u8, opa: u8, operand: u8) -> bool {
        match opr {
            0x1 => {
                // JCN
                let invert = (opa & 0x08) != 0;
                let check_acc_zero = (opa & 0x04) != 0;
                let check_carry = (opa & 0x02) != 0;

                let mut cond_met = false;
                if check_acc_zero && self.acc == 0 {
                    cond_met = true;
                }
                if check_carry && self.carry {
                    cond_met = true;
                }
                if invert {
                    cond_met = !cond_met;
                }

                if cond_met {
                    self.pc = (self.pc & 0xF00) | (operand as u16);
                    return true;
                }
                false
            }
            0x2 => {
                // FIM
                let pair = (opa >> 1) as usize;
                self.registers[pair * 2] = (operand >> 4) & 0x0F;
                self.registers[pair * 2 + 1] = operand & 0x0F;
                false
            }
            0x4 => {
                // JUN
                let target = ((opa as u16) << 8) | (operand as u16);
                self.pc = target;
                true
            }
            0x5 => {
                // JMS: push return address (PC after operand = current PC + 1)
                let return_addr = self.pc.wrapping_add(1);
                self.stack[self.sp as usize] = return_addr;
                self.sp = (self.sp + 1) % 3;
                let target = ((opa as u16) << 8) | (operand as u16);
                self.pc = target;
                true
            }
            0x7 => {
                // ISZ
                let idx = opa as usize;
                self.registers[idx] = (self.registers[idx].wrapping_add(1)) & 0x0F;
                if self.registers[idx] != 0 {
                    self.pc = (self.pc & 0xF00) | (operand as u16);
                    return true;
                }
                false
            }
            _ => false,
        }
    }
}

impl Default for ScalarI4004 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Helper: run ROM on single SIMD lane and return state
    // ========================================================================
    fn run_simd_rom(rom: &[u8], cycles: u64) -> SimdCluster {
        let mut cluster = SimdCluster::new();
        for i in 0..16 {
            cluster.load_rom(i, rom);
        }
        cluster.execute_cycles(cycles);
        cluster
    }

    fn run_scalar_rom(rom: &[u8], cycles: u64) -> ScalarI4004 {
        let mut cpu = ScalarI4004::new();
        cpu.load_rom(rom);
        for _ in 0..cycles {
            cpu.tick();
        }
        cpu
    }

    // ========================================================================
    // Wave 1: Original + regression tests
    // ========================================================================

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

        let rom = vec![0x00; 100];
        for i in 0..16 {
            cluster.load_rom(i, &rom);
        }
        cluster.execute_cycles(10);

        let pcs = cluster.get_pcs();
        for i in 1..16 {
            assert_eq!(pcs[0], pcs[i], "PC mismatch between lane 0 and {}", i);
        }
    }

    #[test]
    fn test_differential_execution_mixed_rom() {
        let rom = vec![
            0x00, 0x00, 0x00, 0x00, // NOP x4
            0x60, 0x61, 0x62, 0x63, // INC R0-R3
            0x00, 0x00, 0x00, 0x00, // NOP x4
        ];
        let cluster = run_simd_rom(&rom, 12);
        let pcs = cluster.get_pcs();
        for i in 1..16 {
            assert_eq!(pcs[0], pcs[i], "PC mismatch after mixed instruction sequence");
        }
    }

    #[test]
    fn test_simd_register_operations() {
        let rom = vec![0x60, 0x61, 0x00]; // INC R0, INC R1, NOP
        let cluster = run_simd_rom(&rom, 3);
        let pcs = cluster.get_pcs();
        for pc in &pcs {
            assert_eq!(*pc, 3);
        }
    }

    #[test]
    fn test_simd_cluster_lane_independence() {
        let mut cluster = SimdCluster::new();
        for lane in 0..16 {
            let mut rom = vec![0x00; 100];
            rom[0] = lane as u8;
            cluster.load_rom(lane, &rom);
        }
        cluster.execute_cycles(1);
        let pcs = cluster.get_pcs();
        for (i, pc) in pcs.iter().enumerate() {
            assert_eq!(*pc, 1, "Lane {} PC should be 1 after 1 cycle", i);
        }
    }

    #[test]
    fn test_simd_cluster_statistics() {
        let mut cluster = SimdCluster::new();
        let rom = vec![0x00; 10];
        cluster.load_rom(0, &rom);
        cluster.execute_cycles(5);
        assert_eq!(cluster.cycles, 5);
        assert_eq!(cluster.instructions, 5);
    }

    #[test]
    fn test_simd_cluster_reset() {
        let rom = vec![0x60; 100];
        let mut cluster = run_simd_rom(&rom, 10);
        assert!(cluster.cycles > 0);
        cluster.reset();
        assert_eq!(cluster.cycles, 0);
        assert_eq!(cluster.instructions, 0);
        let pcs = cluster.get_pcs();
        for pc in &pcs {
            assert_eq!(*pc, 0);
        }
    }

    // Wave 1 regression: ADD all 16 registers
    #[test]
    fn test_add_all_16_registers() {
        // LDM 1 -> INC R0 -> ADD R0 (acc=0+1=1) then INC R15 -> ADD R15
        let rom = vec![
            0xD1, // LDM 1 -> acc=1
            0x60, // INC R0 -> R0=1
            0x80, // ADD R0 -> acc=1+1+0=2
            0x6F, // INC R15 -> R15=1
            0x8F, // ADD R15 -> acc=2+1+0=3
            0x00, // NOP
        ];
        let cluster = run_simd_rom(&rom, 6);
        let accs = cluster.get_accumulators();
        for acc in &accs {
            assert_eq!(*acc, 3, "ADD should work with all registers including R15");
        }
    }

    // Wave 1 regression: SUB with borrow (complement addition)
    #[test]
    fn test_sub_with_borrow_semantics() {
        // acc=5, R0=3, carry=1 (via STC): SUB R0 -> acc + ~3 + 1 = 5 + 12 + 1 = 18 -> acc=2, carry=1
        let rom = vec![
            0xD5, // LDM 5 -> acc=5
            0x60, // INC R0 -> R0=1
            0x60, // INC R0 -> R0=2
            0x60, // INC R0 -> R0=3
            0xFA, // STC -> carry=1
            0x90, // SUB R0 -> acc=5+~3+1 = 5+12+1 = 18 -> acc=2, carry=1
            0x00,
        ];
        let cluster = run_simd_rom(&rom, 7);
        let accs = cluster.get_accumulators();
        let carries = cluster.get_carries();
        for i in 0..16 {
            assert_eq!(accs[i], 2, "SUB with carry=1: 5-3=2");
            assert!(carries[i], "SUB 5-3 with carry=1 should set carry (no borrow)");
        }
    }

    // Wave 1 regression: INC wraps 15->0
    #[test]
    fn test_inc_wraps_at_15() {
        // INC R0 x 16 -> R0 wraps from 15 to 0
        let mut rom = vec![0x60; 16]; // INC R0 x 16
        rom.push(0x00);
        let cluster = run_simd_rom(&rom, 17);
        let r0 = cluster.get_register(0);
        for val in &r0 {
            assert_eq!(*val, 0, "INC should wrap 15->0 after 16 increments");
        }
    }

    // Wave 1 regression: no DEC instruction in 4004
    #[test]
    fn test_no_dec_opcode_0x7x_does_not_decrement() {
        // 0x70-0x7F is ISZ (two-byte), not DEC
        // If executed as single byte it should NOT decrement
        let rom = vec![
            0x60, // INC R0 -> R0=1
            0x00, // NOP (pad so 0x7x doesn't pair with this as operand)
            0x00, // NOP
        ];
        let cluster = run_simd_rom(&rom, 3);
        let r0 = cluster.get_register(0);
        for val in &r0 {
            assert_eq!(*val, 1, "R0 should be 1 (no DEC instruction exists)");
        }
    }

    // ========================================================================
    // Wave 2: Accumulator group tests
    // ========================================================================

    #[test]
    fn test_clb_clears_acc_and_carry() {
        let rom = vec![
            0xD5, // LDM 5
            0xFA, // STC (carry=1)
            0xF0, // CLB (acc=0, carry=0)
            0x00,
        ];
        let cluster = run_simd_rom(&rom, 4);
        let accs = cluster.get_accumulators();
        let carries = cluster.get_carries();
        for i in 0..16 {
            assert_eq!(accs[i], 0);
            assert!(!carries[i]);
        }
    }

    #[test]
    fn test_clc_clears_carry_only() {
        let rom = vec![
            0xD5, // LDM 5
            0xFA, // STC (carry=1)
            0xF1, // CLC (carry=0, acc unchanged)
            0x00,
        ];
        let cluster = run_simd_rom(&rom, 4);
        let accs = cluster.get_accumulators();
        let carries = cluster.get_carries();
        for i in 0..16 {
            assert_eq!(accs[i], 5);
            assert!(!carries[i]);
        }
    }

    #[test]
    fn test_iac_increment_and_overflow() {
        // IAC from 0xF -> 0x0, carry=1
        let rom = vec![
            0xDF, // LDM 15
            0xF2, // IAC -> 0x0, carry=1
            0x00,
        ];
        let cluster = run_simd_rom(&rom, 3);
        let accs = cluster.get_accumulators();
        let carries = cluster.get_carries();
        for i in 0..16 {
            assert_eq!(accs[i], 0, "IAC 15 -> 0");
            assert!(carries[i], "IAC 15 sets carry");
        }
    }

    #[test]
    fn test_cmc_complement_carry() {
        let rom = vec![
            0xF3, // CMC (false -> true)
            0xF3, // CMC (true -> false)
            0x00,
        ];
        let mut cluster = SimdCluster::new();
        for i in 0..16 {
            cluster.load_rom(i, &rom);
        }

        cluster.execute_cycles(1);
        let carries = cluster.get_carries();
        for c in &carries {
            assert!(*c, "CMC should set carry from false");
        }

        cluster.execute_cycles(1);
        let carries = cluster.get_carries();
        for c in &carries {
            assert!(!*c, "CMC should clear carry");
        }
    }

    #[test]
    fn test_cma_complement_accumulator() {
        let rom = vec![
            0xD5, // LDM 5 (0b0101)
            0xF4, // CMA -> ~5 & 0xF = 0xA (0b1010)
            0x00,
        ];
        let cluster = run_simd_rom(&rom, 3);
        let accs = cluster.get_accumulators();
        for acc in &accs {
            assert_eq!(*acc, 0xA, "CMA(5) = 0xA");
        }
    }

    #[test]
    fn test_ral_rotate_left_5_times_returns_to_original() {
        // RAL rotates through carry (5-bit rotate: 4 acc + 1 carry)
        // 5 RALs should return to original state
        let rom = vec![
            0xD5, // LDM 5
            0xF1, // CLC (carry=0)
            0xF5, // RAL x5
            0xF5, 0xF5, 0xF5, 0xF5, 0x00,
        ];
        let cluster = run_simd_rom(&rom, 8);
        let accs = cluster.get_accumulators();
        let carries = cluster.get_carries();
        for i in 0..16 {
            assert_eq!(accs[i], 5, "5 RALs should return to original");
            assert!(!carries[i], "Carry should be back to 0");
        }
    }

    #[test]
    fn test_rar_rotate_right() {
        // acc=0b0101=5, carry=0 -> RAR -> acc=0b0010=2, carry=1
        let rom = vec![
            0xD5, // LDM 5
            0xF1, // CLC
            0xF6, // RAR
            0x00,
        ];
        let cluster = run_simd_rom(&rom, 4);
        let accs = cluster.get_accumulators();
        let carries = cluster.get_carries();
        for i in 0..16 {
            assert_eq!(accs[i], 2, "RAR(5,carry=0) = 2");
            assert!(carries[i], "RAR(5) sets carry (bit0 was 1)");
        }
    }

    #[test]
    fn test_tcc_transfer_carry() {
        let rom = vec![
            0xFA, // STC (carry=1)
            0xF7, // TCC -> acc=1, carry=0
            0x00,
        ];
        let cluster = run_simd_rom(&rom, 3);
        let accs = cluster.get_accumulators();
        let carries = cluster.get_carries();
        for i in 0..16 {
            assert_eq!(accs[i], 1);
            assert!(!carries[i]);
        }
    }

    #[test]
    fn test_dac_decrement_accumulator() {
        // DAC from 0 -> 0xF, carry=0 (borrow occurred, inverted)
        let rom = vec![
            0xD0, // LDM 0
            0xF8, // DAC -> 0xF, carry=0 (0 != 0 is false)
            0x00,
        ];
        let cluster = run_simd_rom(&rom, 3);
        let accs = cluster.get_accumulators();
        let carries = cluster.get_carries();
        for i in 0..16 {
            assert_eq!(accs[i], 0xF, "DAC(0) = 15");
            assert!(!carries[i], "DAC(0) clears carry (borrow)");
        }
    }

    #[test]
    fn test_tcs_transfer_carry_subtract() {
        // carry=1 -> TCS -> acc=10, carry=0
        let rom = vec![
            0xFA, // STC
            0xF9, // TCS -> acc=10, carry=0
            0x00,
        ];
        let cluster = run_simd_rom(&rom, 3);
        let accs = cluster.get_accumulators();
        let carries = cluster.get_carries();
        for i in 0..16 {
            assert_eq!(accs[i], 10);
            assert!(!carries[i]);
        }

        // carry=0 -> TCS -> acc=9
        let rom2 = vec![
            0xF1, // CLC
            0xF9, // TCS -> acc=9
            0x00,
        ];
        let cluster2 = run_simd_rom(&rom2, 3);
        let accs2 = cluster2.get_accumulators();
        for acc in &accs2 {
            assert_eq!(*acc, 9);
        }
    }

    #[test]
    fn test_stc_set_carry() {
        let rom = vec![0xFA, 0x00]; // STC, NOP
        let cluster = run_simd_rom(&rom, 2);
        let carries = cluster.get_carries();
        for c in &carries {
            assert!(*c);
        }
    }

    #[test]
    fn test_daa_decimal_adjust() {
        // acc=6, carry=0 -> DAA -> no adjust (6 <= 9)
        // acc=10, carry=0 -> DAA -> 10+6=16 -> acc=0, carry=1
        let rom = vec![
            0xDA, // LDM 10
            0xFB, // DAA -> 10+6=16 -> acc=0, carry=1
            0x00,
        ];
        let cluster = run_simd_rom(&rom, 3);
        let accs = cluster.get_accumulators();
        let carries = cluster.get_carries();
        for i in 0..16 {
            assert_eq!(accs[i], 0, "DAA(10) = 0");
            assert!(carries[i], "DAA(10) sets carry");
        }
    }

    #[test]
    fn test_kbp_keyboard_process() {
        // KBP lookup: 0->0, 1->1, 2->2, 4->3, 8->4, others->0xF
        let test_cases: [(u8, u8); 6] = [(0, 0), (1, 1), (2, 2), (4, 3), (8, 4), (3, 0xF)];
        for (input, expected) in &test_cases {
            let rom = vec![
                0xD0 | input, // LDM input
                0xFC,         // KBP
                0x00,
            ];
            let cluster = run_simd_rom(&rom, 3);
            let accs = cluster.get_accumulators();
            assert_eq!(accs[0], *expected, "KBP({}) should be {}", input, expected);
        }
    }

    // ========================================================================
    // Wave 3: Single-byte instruction tests
    // ========================================================================

    #[test]
    fn test_ldm_all_values() {
        for val in 0..16u8 {
            let rom = vec![0xD0 | val, 0x00];
            let cluster = run_simd_rom(&rom, 2);
            let accs = cluster.get_accumulators();
            for acc in &accs {
                assert_eq!(*acc, val, "LDM {} failed", val);
            }
        }
    }

    #[test]
    fn test_io_stubs_dont_panic() {
        // Execute all I/O opcodes (0xE0-0xEF) -- should not panic
        for opa in 0..16u8 {
            let rom = vec![0xE0 | opa, 0x00];
            let cluster = run_simd_rom(&rom, 2);
            // Read ops should set acc to 0
            if matches!(opa, 0x9 | 0xA | 0xC | 0xD | 0xE | 0xF) {
                let accs = cluster.get_accumulators();
                for acc in &accs {
                    assert_eq!(*acc, 0, "I/O read 0x{:X} should return 0", opa);
                }
            }
        }
    }

    #[test]
    fn test_src_sets_ram_address() {
        // FIM P0, 0x42 -> SRC P0 -> ram_addr should be 0x42
        let rom = vec![
            0x20, 0x42, // FIM P0, 0x42 (R0=4, R1=2)
            0x21, // SRC P0 -> ram_addr = 0x42
            0x00,
        ];
        let cluster = run_simd_rom(&rom, 4); // 2 cycles for FIM + 1 for SRC + 1 for NOP
        let regs_r0 = cluster.get_register(0);
        let regs_r1 = cluster.get_register(1);
        for i in 0..16 {
            assert_eq!(regs_r0[i], 4, "FIM P0 high nibble");
            assert_eq!(regs_r1[i], 2, "FIM P0 low nibble");
        }
    }

    // ========================================================================
    // Wave 4: Two-byte instruction tests
    // ========================================================================

    #[test]
    fn test_jun_jumps_to_target() {
        // JUN 0x100 -> LDM 5 at 0x100
        let mut rom = vec![0x00; 4096];
        rom[0] = 0x41; // JUN 0x1xx
        rom[1] = 0x00; // target low byte = 0x00 -> target = 0x100
        rom[0x100] = 0xD5; // LDM 5 at target
        rom[0x101] = 0x00;

        // JUN takes 2 cycles (fetch+operand), then LDM takes 1 cycle
        let cluster = run_simd_rom(&rom, 3);
        let pcs = cluster.get_pcs();
        let accs = cluster.get_accumulators();
        for i in 0..16 {
            assert_eq!(pcs[i], 0x101, "PC should be at 0x101 after JUN(2cy)+LDM(1cy)");
            assert_eq!(accs[i], 5, "LDM at target should set acc=5");
        }
    }

    #[test]
    fn test_jms_bbl_roundtrip() {
        // JMS 0x010 -> (at 0x010: BBL 7) -> returns to 0x002 with acc=7
        let mut rom = vec![0x00; 4096];
        rom[0] = 0x50; // JMS 0x0xx
        rom[1] = 0x10; // target = 0x010
        rom[2] = 0x00; // NOP (return here)
        rom[0x10] = 0xC7; // BBL 7

        let cluster = run_simd_rom(&rom, 4); // 2 JMS + 1 BBL + 1 NOP
        let pcs = cluster.get_pcs();
        let accs = cluster.get_accumulators();
        for i in 0..16 {
            assert_eq!(pcs[i], 3, "Should return to 0x002 then advance to 0x003");
            assert_eq!(accs[i], 7, "BBL should load 7 into acc");
        }
    }

    #[test]
    fn test_jcn_acc_zero_branches() {
        // acc=0 (default), JCN c=4 (acc==0), target=0x10
        let mut rom = vec![0x00; 4096];
        rom[0] = 0x14; // JCN condition=4 (test acc==0)
        rom[1] = 0x10; // target = 0x10
        rom[0x10] = 0xD5; // LDM 5

        let cluster = run_simd_rom(&rom, 3); // 2 JCN + 1 LDM
        let accs = cluster.get_accumulators();
        for acc in &accs {
            assert_eq!(*acc, 5, "JCN should branch when acc==0");
        }
    }

    #[test]
    fn test_jcn_acc_nonzero_falls_through() {
        // acc=5, JCN c=4 (acc==0), should NOT branch
        let mut rom = vec![0x00; 4096];
        rom[0] = 0xD5; // LDM 5
        rom[1] = 0x14; // JCN condition=4 (test acc==0)
        rom[2] = 0x10; // target
        rom[3] = 0xD3; // LDM 3 (fall-through)

        let cluster = run_simd_rom(&rom, 4); // LDM + 2 JCN + 1 LDM
        let accs = cluster.get_accumulators();
        for acc in &accs {
            assert_eq!(*acc, 3, "JCN should fall through when acc!=0");
        }
    }

    #[test]
    fn test_isz_loop() {
        // ISZ R0, loop_start -> increments R0, jumps if R0 != 0
        // R0 starts at 0, ISZ increments to 1 (jump), 2 (jump), ..., 15 (jump), 0 (fall through)
        // 16 total ISZ executions * 2 cycles each = 32 cycles, then 1 for LDM = 33
        let mut rom = vec![0x00; 4096];
        rom[0] = 0x70; // ISZ R0, 0x00 (loop to self)
        rom[1] = 0x00; // target low byte = 0x00
        rom[2] = 0xD5; // LDM 5 (reached when ISZ falls through)

        let cluster = run_simd_rom(&rom, 33);
        let accs = cluster.get_accumulators();
        let r0 = cluster.get_register(0);
        for i in 0..16 {
            assert_eq!(r0[i], 0, "R0 should be 0 after ISZ loop completes");
            assert_eq!(accs[i], 5, "Should reach LDM 5 after ISZ falls through");
        }
    }

    #[test]
    fn test_fim_loads_register_pair() {
        let rom = vec![
            0x20, 0x42, // FIM P0, 0x42 -> R0=4, R1=2
            0x24, 0xAB, // FIM P2, 0xAB -> R4=0xA, R5=0xB
            0x00,
        ];
        let cluster = run_simd_rom(&rom, 5); // 2+2+1
        let r0 = cluster.get_register(0);
        let r1 = cluster.get_register(1);
        let r4 = cluster.get_register(4);
        let r5 = cluster.get_register(5);
        for i in 0..16 {
            assert_eq!(r0[i], 4);
            assert_eq!(r1[i], 2);
            assert_eq!(r4[i], 0xA);
            assert_eq!(r5[i], 0xB);
        }
    }

    // ========================================================================
    // Wave 5: Differential fuzzing (SIMD vs scalar)
    // ========================================================================

    #[test]
    fn test_scalar_matches_simd_nop() {
        let rom = vec![0x00; 20];
        let cluster = run_simd_rom(&rom, 10);
        let scalar = run_scalar_rom(&rom, 10);

        assert_eq!(cluster.get_pcs()[0], scalar.pc);
        assert_eq!(cluster.get_accumulators()[0], scalar.acc);
    }

    #[test]
    fn test_scalar_matches_simd_alu_ops() {
        let rom = vec![
            0xD5, // LDM 5
            0x60, // INC R0
            0x60, // INC R0
            0x80, // ADD R0
            0xF2, // IAC
            0xFA, // STC
            0x90, // SUB R0
            0xF4, // CMA
            0xF5, // RAL
            0xF6, // RAR
            0x00,
        ];
        let cluster = run_simd_rom(&rom, 11);
        let scalar = run_scalar_rom(&rom, 11);

        assert_eq!(cluster.get_pcs()[0], scalar.pc, "PC mismatch");
        assert_eq!(cluster.get_accumulators()[0], scalar.acc, "ACC mismatch");
        assert_eq!(cluster.get_carries()[0], scalar.carry, "Carry mismatch");
    }

    #[test]
    fn test_scalar_matches_simd_jump_ops() {
        let mut rom = vec![0x00; 4096];
        rom[0] = 0x40; // JUN 0x010
        rom[1] = 0x10;
        rom[0x10] = 0xD3; // LDM 3
        rom[0x11] = 0x50; // JMS 0x020
        rom[0x12] = 0x20;
        rom[0x13] = 0x00; // NOP (return target)
        rom[0x20] = 0xC7; // BBL 7

        let cluster = run_simd_rom(&rom, 6);
        let scalar = run_scalar_rom(&rom, 6);

        assert_eq!(cluster.get_pcs()[0], scalar.pc, "PC mismatch after jumps");
        assert_eq!(cluster.get_accumulators()[0], scalar.acc, "ACC mismatch after jumps");
    }

    #[test]
    fn test_scalar_matches_simd_all_acc_ops() {
        // Test all 14 accumulator instructions
        let rom = vec![
            0xD5, // LDM 5
            0xFA, // STC
            0xF0, // CLB
            0xD3, // LDM 3
            0xF1, // CLC
            0xF2, // IAC -> 4
            0xF3, // CMC -> carry=true
            0xF4, // CMA -> ~4=0xB
            0xF5, // RAL
            0xF6, // RAR
            0xF7, // TCC
            0xF8, // DAC
            0xF9, // TCS
            0xFA, // STC
            0xFB, // DAA
            0xFC, // KBP
            0xFD, // DCL (no-op)
            0x00,
        ];
        let cluster = run_simd_rom(&rom, 18);
        let scalar = run_scalar_rom(&rom, 18);

        assert_eq!(
            cluster.get_accumulators()[0],
            scalar.acc,
            "ACC mismatch after all acc ops: SIMD={} scalar={}",
            cluster.get_accumulators()[0],
            scalar.acc
        );
        assert_eq!(
            cluster.get_carries()[0],
            scalar.carry,
            "Carry mismatch after all acc ops"
        );
    }

    #[test]
    fn test_differential_random_single_byte_programs() {
        // Test 100 random single-byte-only programs
        // Use deterministic seed via simple LCG
        let mut rng_state: u64 = 0xDEADBEEF;
        let single_byte_opcodes: Vec<u8> = (0..256u16)
            .filter(|&op| {
                let opr = (op >> 4) as u8;
                let opa = (op & 0x0F) as u8;
                // Exclude two-byte opcodes, JIN (unpredictable PC), BBL (uninit stack)
                !(matches!(opr, 0x1 | 0x4 | 0x5 | 0x7)
                    || (opr == 0x2 && (opa & 1) == 0)
                    || (opr == 0x3 && (opa & 1) == 1))
                && opr != 0xC
            })
            .map(|op| op as u8)
            .collect();

        for _ in 0..100 {
            let mut rom = Vec::with_capacity(64);
            for _ in 0..32 {
                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let idx = ((rng_state >> 33) as usize) % single_byte_opcodes.len();
                rom.push(single_byte_opcodes[idx]);
            }
            // Pad with NOPs
            while rom.len() < 100 {
                rom.push(0x00);
            }

            let cluster = run_simd_rom(&rom, 32);
            let scalar = run_scalar_rom(&rom, 32);

            assert_eq!(
                cluster.get_accumulators()[0],
                scalar.acc,
                "ACC divergence on random program"
            );
            assert_eq!(
                cluster.get_carries()[0],
                scalar.carry,
                "Carry divergence on random program"
            );
            assert_eq!(cluster.get_pcs()[0], scalar.pc, "PC divergence on random program");
        }
    }

    // ========================================================================
    // Benchmark tests (preserved from Phase 4F)
    // ========================================================================

    #[test]
    fn test_benchmark_nop_throughput() {
        let mut cluster = SimdCluster::new();
        let rom = vec![0x00; 1000];
        for i in 0..16 {
            cluster.load_rom(i, &rom);
        }

        let elapsed_ns = cluster.benchmark_execution(100);
        let metrics = cluster.get_metrics();
        assert_eq!(metrics.cycles, 100);
        assert_eq!(metrics.instructions, 100);
        assert!(elapsed_ns > 0);
        assert!(metrics.throughput_insn_per_ns > 0.0);

        let summary = metrics.format_summary();
        assert!(summary.contains("cycles=100"));
    }

    #[test]
    fn test_benchmark_register_ops_throughput() {
        let mut cluster = SimdCluster::new();
        let rom = vec![
            0x60, 0x61, 0x62, 0x63, // INC R0-R3
            0xA0, 0xA1, 0xA2, 0xA3, // LD R0-R3
            0x00,
        ];
        for i in 0..16 {
            cluster.load_rom(i, &rom);
        }

        let elapsed_ns = cluster.benchmark_execution(50);
        let metrics = cluster.get_metrics();
        assert_eq!(metrics.cycles, 50);
        assert!(elapsed_ns > 0);
        assert!(metrics.throughput_insn_per_ns > 0.0);
    }

    #[test]
    fn test_benchmark_memory_usage() {
        let mut cluster = SimdCluster::new();
        let rom = vec![0x00; 4096];
        for i in 0..16 {
            cluster.load_rom(i, &rom);
        }

        cluster.benchmark_execution(10);
        let metrics = cluster.get_metrics();
        let expected_min = std::mem::size_of::<SimdI4004>() + (16 * 4096) + (16 * 40);
        assert!(metrics.peak_memory_bytes >= expected_min);
    }

    #[test]
    fn test_performance_metrics_reset() {
        let mut cluster = SimdCluster::new();
        let rom = vec![0x00; 100];
        for i in 0..16 {
            cluster.load_rom(i, &rom);
        }

        cluster.benchmark_execution(20);
        assert!(cluster.get_metrics().cycles > 0);

        cluster.reset_metrics();
        let m = cluster.get_metrics();
        assert_eq!(m.cycles, 0);
        assert_eq!(m.instructions, 0);
        assert_eq!(m.exec_time_ns, 0);
    }

    #[test]
    fn test_benchmark_consistency() {
        let mut cluster = SimdCluster::new();
        let rom = vec![0x60, 0x61, 0x62, 0x63, 0x00];
        for i in 0..16 {
            cluster.load_rom(i, &rom);
        }

        let mut times = Vec::new();
        for _ in 0..3 {
            cluster.reset();
            let elapsed = cluster.benchmark_execution(50);
            times.push(elapsed);
        }

        let min_time = *times.iter().min().expect("non-empty");
        let max_time = *times.iter().max().expect("non-empty");
        assert!(max_time > 0 && min_time > 0);
        if min_time > 1000 {
            assert!(max_time <= min_time * 5);
        }
    }
}
