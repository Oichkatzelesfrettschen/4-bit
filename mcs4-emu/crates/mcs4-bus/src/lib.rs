//! MCS-4/MCS-40 Bus Infrastructure
//!
//! This crate implements the bus protocol for Intel's 4-bit microcomputer systems:
//! - 4-bit bidirectional data bus (D0-D3)
//! - SYNC signal for cycle synchronization
//! - CM-ROM (Chip Memory - ROM select)
//! - CM-RAM (Chip Memory - RAM select)
//! - Two-phase clock (PHI1, PHI2)
//!
//! # Modules
//!
//! - [`clock`] - Two-phase clock generator configuration
//! - [`control`] - Control signals (chip select, sync)
//! - [`cycle`] - Bus cycle state machine (A1-A3, M1-M2, X1-X3)
//! - [`data_bus`] - 4-bit bidirectional data bus
#![allow(missing_docs)]

pub mod clock;
pub mod control;
pub mod cycle;
pub mod data_bus;

pub use clock::{ClockConfig, TwoPhaseClock};
pub use control::{ChipSelect, ControlSignals};
pub use cycle::{BusCycle, CycleState, MachineState};
pub use data_bus::DataBus;

/// Prelude for common imports
pub mod prelude {
    pub use crate::{clock::*, control::*, cycle::*, data_bus::*};
}
