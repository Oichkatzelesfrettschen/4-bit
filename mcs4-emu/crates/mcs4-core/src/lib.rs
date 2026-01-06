//! MCS-4/MCS-40 Core Simulation Primitives
//!
//! This crate provides the fundamental building blocks for gate-level
//! and transistor-level simulation of Intel 4004/4040 microcomputer systems.

pub mod gate;
pub mod signal;
pub mod simulator;
pub mod timing;
pub mod transistor;
pub mod wire;

pub use gate::{And2, Gate, GateType, Inverter, Nand2, Nand3, Nor2, Nor3, Or2};
pub use signal::{Signal, SignalId, SignalLevel};
pub use simulator::{Event, Simulator, SimulatorConfig};
pub use timing::{Delay, Time, MICROSECOND, NANOSECOND, PICOSECOND};
pub use wire::{Fanout, Net, Wire};

/// Prelude for common imports
pub mod prelude {
    pub use crate::{gate::*, signal::*, simulator::*, timing::*, wire::*};
}
