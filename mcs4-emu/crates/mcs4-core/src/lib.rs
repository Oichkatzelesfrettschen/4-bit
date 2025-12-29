//! MCS-4/MCS-40 Core Simulation Primitives
//!
//! This crate provides the fundamental building blocks for gate-level
//! and transistor-level simulation of Intel 4004/4040 microcomputer systems.

pub mod timing;
pub mod signal;
pub mod gate;
pub mod wire;
pub mod transistor;
pub mod simulator;

pub use timing::{Time, Delay, PICOSECOND, NANOSECOND, MICROSECOND};
pub use signal::{SignalLevel, Signal, SignalId};
pub use gate::{Gate, GateType, Nand2, Nor2, Inverter, Nand3, Nor3, And2, Or2};
pub use wire::{Wire, Net, Fanout};
pub use simulator::{Simulator, Event, SimulatorConfig};

/// Prelude for common imports
pub mod prelude {
    pub use crate::timing::*;
    pub use crate::signal::*;
    pub use crate::gate::*;
    pub use crate::wire::*;
    pub use crate::simulator::*;
}
