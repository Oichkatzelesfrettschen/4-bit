#![cfg_attr(feature = "simd", feature(portable_simd))]
#![allow(missing_docs)]
//! MCS-4/MCS-40 Chip Implementations
//!
//! This crate provides gate-level implementations of the Intel 4-bit
//! microcomputer chip families.
//!
//! ## MCS-4 Family (4004-based)
//! - [`i4004`] - 4-bit CPU
//! - [`i4001`] - 256x8 ROM with 4-bit I/O
//! - [`i4002`] - 320-bit RAM with 4-bit output
//! - [`i4003`] - 10-bit shift register
//! - [`i4008`] - 12-bit address latch with CM-ROM decode
//! - [`i4009`] - Standard I/O expander
//! - [`i3216`] - 4-bit bidirectional bus driver (non-inverting)
//! - [`i3226`] - 4-bit bidirectional bus driver (inverting)
//!
//! ## MCS-40 Family (4040-based)
//! - [`i4040`] - Enhanced 4-bit CPU with interrupts
//! - [`i4101`] - 256x4 static RAM
//! - [`i4201`] - Clock generator
//! - [`i4207`] - Single-phase crystal clock generator
//! - [`i4209`] - Single-to-two-phase clock converter
//! - [`i4211`] - RC oscillator + two-phase clock generator
//! - [`i4265`] - Programmable general purpose I/O (4x4 bits)
//! - [`i4289`] - Standard memory interface
//! - [`i4308`] - 1Kx8 ROM
//! - [`i4316`] - LCD segment driver
//! - [`i4702`] - 256x8 UV-erasable PROM

pub mod i4001;
pub mod i4002;
pub mod i4003;
pub mod i4004;
pub mod i4040;

// MCS-4 support chips
pub mod i3205;
pub mod i3216;
pub mod i3226;
pub mod i3404;
pub mod i4008;
pub mod i4009;

// External memory chips
pub mod i2101;

// MCS-40 specific chips
pub mod i4101;
pub mod i4201;
pub mod i4289;
pub mod i4308;

// MCS-40 clock generators
pub mod i4207;
pub mod i4209;
pub mod i4211;

// MCS-40 peripheral chips
pub mod i4265;
pub mod i4316;
pub mod i4702;

pub mod disasm;
#[cfg(feature = "simd")]
pub mod simd;

/// Common trait for all chips
pub trait Chip: Send + Sync {
    /// Chip name (e.g., "4004", "4001")
    fn name(&self) -> &'static str;

    /// Reset chip to initial state
    fn reset(&mut self);

    /// Process one clock cycle
    fn tick(&mut self, phase: mcs4_bus::BusCycle);
}
