//! SIMD-accelerated parallel CPU execution
//!
//! Vectorizes multiple CPU instances to run in parallel on a single thread.
//! Useful for fuzzing and large-scale simulation.

use std::simd::prelude::*;

/// Number of parallel CPU lanes (adjust based on target architecture)
pub const LANES: usize = 8; // AVX2 friendly

pub struct CpuClusterSimd {
    // Register file (SoA layout)
    pub regs: [Simd<u8, LANES>; 16],
    pub acc: Simd<u8, LANES>,
    pub carry: Mask<i8, LANES>,
    pub pc: Simd<u16, LANES>,

    // Cycle state
    pub phase: Simd<u8, LANES>,

    // Internal status
    pub halted: Mask<i8, LANES>,
}

impl CpuClusterSimd {
    pub fn new() -> Self {
        Self {
            regs: [Simd::splat(0); 16],
            acc: Simd::splat(0),
            carry: Mask::splat(false),
            pc: Simd::splat(0),
            phase: Simd::splat(0),
            halted: Mask::splat(false),
        }
    }

    /// Step all lanes by one machine cycle (simplified behavioral model)
    pub fn step(&mut self, roms: &[&[u8]; LANES]) {
        // 1. Fetch
        let mut opcodes = Simd::<u8, LANES>::splat(0);
        let bitmask = self.halted.to_bitmask();
        for i in 0..LANES {
            if (bitmask & (1 << i)) == 0 {
                let addr = self.pc[i] as usize;
                opcodes.as_mut_array()[i] = roms[i][addr % roms[i].len()];
            }
        }

        // 2. Decode & Execute (Vectorized logic where possible)
        // Example: ADD instruction (OPR=0x8)
        let opr = (opcodes >> Simd::splat(4)) & Simd::splat(0x0F);
        let opa = opcodes & Simd::splat(0x0F);

        let is_add = opr.simd_eq(Simd::splat(0x8));

        // Vectorized ADD
        if is_add.any() {
            let mut reg_vals = Simd::<u8, LANES>::splat(0);
            for i in 0..LANES {
                let reg_idx = opa[i] as usize;
                if reg_idx < 16 {
                    reg_vals.as_mut_array()[i] = self.regs[reg_idx][i];
                }
            }

            let sum = self.acc + reg_vals;
            let new_acc = sum & Simd::splat(0x0F);
            let new_carry = sum.simd_gt(Simd::splat(0x0F));

            self.acc = is_add.select(new_acc, self.acc);

            // For masks, we use bitwise logic instead of select
            self.carry = (is_add & new_carry) | (!is_add & self.carry);
        }

        // Increment PC
        // Need to convert Mask<i8, 8> to Mask<i16, 8> for use with Simd<u16, 8>
        let not_halted = !self.halted;
        let mut pc_mask_array = [false; LANES];
        pc_mask_array[..LANES].copy_from_slice(&not_halted.to_array()[..LANES]);
        let pc_mask = Mask::<i16, LANES>::from_array(pc_mask_array);

        self.pc = pc_mask.select(self.pc + Simd::splat(1), self.pc);
    }
}

impl Default for CpuClusterSimd {
    fn default() -> Self {
        Self::new()
    }
}
