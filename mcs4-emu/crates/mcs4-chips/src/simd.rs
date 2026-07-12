//! SIMD-accelerated parallel execution for a narrow 4004 instruction subset.
//!
//! The kernel executes fetch, register ADD, and 12-bit program-counter advance
//! across independent lanes. It is a fuzzing helper, not a full I4004 model.

use std::simd::{prelude::*, Select};

/// Number of parallel CPU lanes.
pub const LANES: usize = 8;

const PROGRAM_COUNTER_MASK: u16 = 0x0FFF;

/// Struct-of-arrays state for the supported SIMD execution subset.
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

    /// Step all active lanes through the supported simplified execution subset.
    ///
    /// An empty ROM supplies a NOP. A non-empty ROM wraps addresses within its
    /// own length. The program counter retains the 4004 12-bit address range.
    pub fn step(&mut self, roms: &[&[u8]; LANES]) {
        // 1. Fetch
        let mut opcodes = Simd::<u8, LANES>::splat(0);
        let bitmask = self.halted.to_bitmask();
        for lane in 0..LANES {
            if (bitmask & (1 << lane)) == 0 && !roms[lane].is_empty() {
                let address = (self.pc[lane] & PROGRAM_COUNTER_MASK) as usize;
                opcodes.as_mut_array()[lane] = roms[lane][address % roms[lane].len()];
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
            for lane in 0..LANES {
                let reg_idx = opa[lane] as usize;
                if reg_idx < 16 {
                    reg_vals.as_mut_array()[lane] = self.regs[reg_idx][lane];
                }
            }

            let sum = (self.acc & Simd::splat(0x0F)) + (reg_vals & Simd::splat(0x0F));
            let new_acc = sum & Simd::splat(0x0F);
            let new_carry = sum.simd_gt(Simd::splat(0x0F));

            self.acc = is_add.select(new_acc, self.acc);

            // For masks, we use bitwise logic instead of select
            self.carry = (is_add & new_carry) | (!is_add & self.carry);
        }

        // Increment PC
        // Need to convert Mask<i8, 8> to Mask<i16, 8> for use with Simd<u16, 8>
        let not_halted = !self.halted;
        let pc_mask = Mask::<i16, LANES>::from_array(not_halted.to_array());
        let current_pc = self.pc & Simd::splat(PROGRAM_COUNTER_MASK);

        self.pc = pc_mask.select(
            (current_pc + Simd::splat(1)) & Simd::splat(PROGRAM_COUNTER_MASK),
            self.pc,
        );
    }
}

impl Default for CpuClusterSimd {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug)]
    struct ScalarLane {
        regs: [u8; 16],
        acc: u8,
        carry: bool,
        pc: u16,
        halted: bool,
    }

    impl ScalarLane {
        fn step(&mut self, rom: &[u8]) {
            if self.halted {
                return;
            }

            let opcode = if rom.is_empty() {
                0
            } else {
                let address = (self.pc & PROGRAM_COUNTER_MASK) as usize;
                rom[address % rom.len()]
            };

            if opcode >> 4 == 0x8 {
                let register = (opcode & 0x0F) as usize;
                let sum = (self.acc & 0x0F) + (self.regs[register] & 0x0F);
                self.acc = sum & 0x0F;
                self.carry = sum > 0x0F;
            }

            self.pc = ((self.pc & PROGRAM_COUNTER_MASK) + 1) & PROGRAM_COUNTER_MASK;
        }
    }

    fn next_u8(state: &mut u32) -> u8 {
        *state ^= *state << 13;
        *state ^= *state >> 17;
        *state ^= *state << 5;
        (*state >> 8) as u8
    }

    fn assert_cluster_matches_scalar(cluster: &CpuClusterSimd, scalar: &[ScalarLane; LANES]) {
        let acc = cluster.acc.to_array();
        let carry = cluster.carry.to_array();
        let pc = cluster.pc.to_array();
        let halted = cluster.halted.to_array();
        for lane in 0..LANES {
            assert_eq!(acc[lane], scalar[lane].acc, "lane {lane} accumulator");
            assert_eq!(carry[lane], scalar[lane].carry, "lane {lane} carry");
            assert_eq!(pc[lane], scalar[lane].pc, "lane {lane} program counter");
            assert_eq!(halted[lane], scalar[lane].halted, "lane {lane} halt state");
        }
    }

    #[test]
    fn supported_subset_matches_scalar_lanes_over_deterministic_corpus() {
        let mut random = 0x4D43_5334_u32;
        let mut scalar = std::array::from_fn(|lane| ScalarLane {
            regs: std::array::from_fn(|register| next_u8(&mut random).wrapping_add((lane + register) as u8) & 0x0F),
            acc: next_u8(&mut random) & 0x0F,
            carry: next_u8(&mut random) & 1 != 0,
            pc: if lane == 0 {
                PROGRAM_COUNTER_MASK
            } else {
                u16::from(next_u8(&mut random)) & PROGRAM_COUNTER_MASK
            },
            halted: lane == 3,
        });
        let mut cluster = CpuClusterSimd::new();

        cluster.acc = Simd::from_array(std::array::from_fn(|lane| scalar[lane].acc));
        cluster.carry = Mask::from_array(std::array::from_fn(|lane| scalar[lane].carry));
        cluster.pc = Simd::from_array(std::array::from_fn(|lane| scalar[lane].pc));
        cluster.halted = Mask::from_array(std::array::from_fn(|lane| scalar[lane].halted));
        for register in 0..16 {
            cluster.regs[register] = Simd::from_array(std::array::from_fn(|lane| scalar[lane].regs[register]));
        }

        let rom_storage: [Vec<u8>; LANES] = std::array::from_fn(|lane| {
            if lane == 7 {
                return Vec::new();
            }
            let mut rom = Vec::with_capacity(5 + lane);
            for offset in 0..(5 + lane) {
                let opcode = if offset % 2 == 0 {
                    0x80 | (next_u8(&mut random) & 0x0F)
                } else {
                    next_u8(&mut random)
                };
                rom.push(opcode);
            }
            rom
        });
        let roms: [&[u8]; LANES] = std::array::from_fn(|lane| rom_storage[lane].as_slice());

        for _ in 0..64 {
            cluster.step(&roms);
            for lane in 0..LANES {
                scalar[lane].step(roms[lane]);
            }
            assert_cluster_matches_scalar(&cluster, &scalar);
        }
    }

    #[test]
    fn empty_rom_is_a_nop_that_advances_only_active_lanes() {
        let mut cluster = CpuClusterSimd::new();
        cluster.pc = Simd::from_array([0; LANES]);
        cluster.halted = Mask::from_array([false, true, false, false, false, false, false, false]);
        let empty: [&[u8]; LANES] = [&[]; LANES];

        cluster.step(&empty);

        assert_eq!(cluster.pc.to_array(), [1, 0, 1, 1, 1, 1, 1, 1]);
        assert_eq!(cluster.acc.to_array(), [0; LANES]);
        assert_eq!(cluster.carry.to_array(), [false; LANES]);
    }
}
