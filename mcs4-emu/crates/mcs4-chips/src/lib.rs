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
//!
//! ## MCS-40 Family (4040-based)
//! - [`i4040`] - Enhanced 4-bit CPU with interrupts
//! - [`i4101`] - 256x4 static RAM
//! - [`i4201`] - Clock generator
//! - [`i4289`] - Standard memory interface
//! - [`i4308`] - 1Kx8 ROM

pub mod i4004;
pub mod i4040;
pub mod i4001;
pub mod i4002;
pub mod i4003;

// MCS-40 specific chips
pub mod i4101;
pub mod i4201;
pub mod i4289;
pub mod i4308;

/// Common trait for all chips
pub trait Chip: Send + Sync {
    /// Chip name (e.g., "4004", "4001")
    fn name(&self) -> &'static str;

    /// Reset chip to initial state
    fn reset(&mut self);

    /// Process one clock cycle
    fn tick(&mut self, phase: mcs4_bus::BusCycle);
}
