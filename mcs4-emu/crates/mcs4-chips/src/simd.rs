#![cfg(feature = "simd")]
//! Portable-SIMD cluster execution stubs (nightly std::simd)

#![allow(dead_code)]

use core::simd::{Simd, SimdUint, Mask as SimdMask};

pub struct CpuStateSimd<const LANES: usize> {
    pub acc: Simd<u8, LANES>,
    pub carry: SimdMask<u8, LANES>,
    pub pc: Simd<u16, LANES>,
    pub regs: [Simd<u8, LANES>; 16],
    pub stack: [Simd<u16, LANES>; 7],
    pub sp: Simd<u8, LANES>,
}

impl<const LANES: usize> CpuStateSimd<LANES> {
    pub fn new() -> Self {
        Self {
            acc: Simd::splat(0),
            carry: SimdMask::splat(false),
            pc: Simd::splat(0),
            regs: [Simd::splat(0); 16],
            stack: [Simd::splat(0); 7],
            sp: Simd::splat(0),
        }
    }
}

pub struct CpuSimd<const LANES: usize> {
    pub state: CpuStateSimd<LANES>,
    // Each lane points to its ROM slice
    pub roms: [Option<&'static [u8]>; LANES],
}

impl<const LANES: usize> CpuSimd<LANES> {
    pub fn new() -> Self { Self { state: CpuStateSimd::new(), roms: [None; LANES] } }
    pub fn load_roms(&mut self, roms: [&'static [u8]; LANES]) { for (i, r) in roms.iter().enumerate() { self.roms[i] = Some(r); } }
    pub fn reset_lane(&mut self, lane: usize) { self.state.acc[lane] = 0; self.state.pc[lane] = 0; self.state.sp[lane] = 0; }
    pub fn step(&mut self) { /* TODO: vectorized fetch/decode/execute */ }
}
