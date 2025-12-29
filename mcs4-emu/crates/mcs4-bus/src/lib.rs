//! MCS-4/MCS-40 Bus Infrastructure
//!
//! This crate implements the bus protocol for Intel's 4-bit microcomputer systems:
//! - 4-bit bidirectional data bus (D0-D3)
//! - SYNC signal for cycle synchronization
//! - CM-ROM (Chip Memory - ROM select)
//! - CM-RAM (Chip Memory - RAM select)
//! - Two-phase clock (PHI1, PHI2)

pub mod clock;
pub mod data_bus;
pub mod control;
pub mod cycle;

pub use clock::{TwoPhaseClockTwoPhaseClock as TwoPhaseClock, ClockConfig};
pub use data_bus::DataBus;
pub use control::{ControlSignals, ChipSelect};
pub use cycle::{BusCycle, CycleState, MachineState};

/// Prelude for common imports
pub mod prelude {
    pub use crate::clock::*;
    pub use crate::data_bus::*;
    pub use crate::control::*;
    pub use crate::cycle::*;
}
