#![allow(missing_docs)]

//! MCS-4 System Integration
//!
//! This crate wires together the bus, chip, and core crates into complete
//! MCS-4 and MCS-40 systems with memory maps, I/O, and test fixtures.

pub mod cluster;
pub mod fixture;
pub mod mcs4;
pub mod mcs40;

// `src/simd_cluster.rs` is intentionally NOT declared as a module here. It
// holds 2k LOC of in-progress 16-lane SIMD work that has rotted (147 build
// errors under `--features simd_cluster` as of 2026-04-30, primarily the
// missing `#![feature(portable_simd)]` crate attribute and other drift).
// Resurrecting it is tracked under debt phase D1.4.3 / D2.2 in
// `~/.claude/plans/elucidate-and-build-out-merry-gadget.md`. Until that
// happens, the `simd_cluster` Cargo feature is inert.

pub use fixture::{load_hex_bytes, parse_hex_bytes, FixtureError};
pub use mcs4::Mcs4System;
pub use mcs40::Mcs40System;
use mcs4_chips::{i4001::I4001, i4002::I4002};
use mcs4_core::FidelityManager;

/// Memory-map a ROM file as read-only bytes.
///
/// Convenience wrapper around `memmap2::Mmap::map` used by both `Mcs4System`
/// and `Mcs40System` to load ROM images.
///
/// # Safety
///
/// `memmap2::Mmap::map` is `unsafe` because the kernel can change the
/// underlying file's contents while the mapping is alive (e.g., another
/// process truncating or writing to the same path). For ROM-image loading we
/// treat the file as content-addressed: callers point at static, read-only
/// fixtures under `mcs4-emu/crates/mcs4-system/fixtures/` (or test-only
/// tempfiles whose handles are kept alive by the caller). Under those
/// invariants the mapped memory is stable for the lifetime of the returned
/// `Mmap`. If you intend to consume mutable or writer-shared files, use
/// `std::fs::read` instead.
pub fn load_rom_mmap(path: impl AsRef<std::path::Path>) -> std::io::Result<memmap2::Mmap> {
    let file = std::fs::File::open(path)?;
    // SAFETY: see function-level docs. Caller-provided ROM images are
    // expected to be immutable for the lifetime of the returned mapping.
    unsafe { memmap2::Mmap::map(&file) }
}

/// Common interface for MCS-4 and MCS-40 systems
pub trait System {
    /// Step one bus phase
    fn step(&mut self);

    /// Advance analog solvers by given time step
    fn step_analog(&mut self, dt: f64);

    /// Get the fidelity manager
    fn fidelity(&self) -> &FidelityManager;
    fn fidelity_mut(&mut self) -> &mut FidelityManager;

    /// Run for N machine cycles
    fn run_cycles(&mut self, cycles: usize) {
        for _ in 0..(cycles * 8) {
            self.step();
        }
    }

    /// Reset the system
    fn reset(&mut self);

    /// Set the program counter
    fn set_pc(&mut self, addr: u16);

    /// Get current program counter
    fn pc(&self) -> u16;

    /// Get accumulator value
    fn accumulator(&self) -> u8;

    /// Load program into ROM
    fn load_rom(&mut self, data: &[u8]);

    /// Read ROM at address
    fn read_rom(&self, addr: u16) -> Option<u8>;

    /// Access ROM chips
    fn roms(&self) -> &[I4001];
    fn roms_mut(&mut self) -> &mut [I4001];

    /// Access RAM chips
    fn rams(&self) -> &[I4002];
    fn rams_mut(&mut self) -> &mut [I4002];
}

// SIMD cluster execution...

#[cfg(test)]
mod wiring_tests {
    use std::io::Write;

    use bumpalo::Bump;
    use memmap2::MmapOptions;

    #[test]
    fn mmap_and_bumpalo_work() {
        let mut f = tempfile::tempfile().expect("tempfile");
        f.write_all(&[0u8; 64]).expect("write temp data");
        let mmap = unsafe { MmapOptions::new().len(64).map(&f).expect("mmap temp file") };
        assert_eq!(mmap.len(), 64);
        let bump = Bump::new();
        let s = bump.alloc_slice_fill_copy(10, 7u8);
        assert_eq!(s.len(), 10);
    }
}
