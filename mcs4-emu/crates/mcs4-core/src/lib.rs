//! MCS-4/MCS-40 Core Simulation Primitives
//!
//! This crate provides the fundamental building blocks for gate-level
//! and transistor-level simulation of Intel 4004/4040 microcomputer systems.
//!
//! # Modules
//!
//! - [`process`] - Semiconductor material properties (Intel 10um pMOS SGT)
//! - [`device`] - Transistor models (Shichman-Hodges Level 1)
//! - [`circuit`] - Netlist-to-graph bridge and circuit representation
//! - [`solver`] - SPICE-class DC and transient simulation
//! - [`gate`] - Logic gate primitives (AND, OR, NAND, NOR, INV)
//! - [`signal`] - Signal types and routing
//! - [`simulator`] - Event-driven gate-level simulation engine
//! - [`timing`] - Time representation (picosecond precision)
//! - [`transistor`] - Switch-level transistor abstractions
//! - [`transistor_solver`] - Switch-level iterative solver
//! - [`nodal_solver`] - Nodal analysis with RC networks
//! - [`wire`] - Wire and net connectivity
//! - [`trace_buffer`] - Signal trace capture for waveform display
//! - [`layout_netlist`] - NetlistV1 JSON parser for extracted chip data
//! - [`netlist_v0`] - Legacy NetlistV0 parser
#![allow(missing_docs)]

pub mod circuit;
pub mod device;
pub mod gate;
pub mod layout_netlist;
pub mod netlist_v0;
pub mod nodal_solver;
pub mod process;
pub mod signal;
pub mod simulator;
pub mod solver;
pub mod timing;
pub mod trace_buffer;
pub mod transistor;
pub mod transistor_solver;
pub mod wire;

pub use gate::{And2, Gate, GateType, Inverter, Nand2, Nand3, Nor2, Nor3, Or2};
pub use signal::{Signal, SignalId, SignalLevel};
pub use simulator::{Event, Simulator, SimulatorConfig};
pub use timing::{Delay, Time, MICROSECOND, NANOSECOND, PICOSECOND};
pub use trace_buffer::{SignalSample, TraceBuffer};
pub use transistor_solver::{Circuit, Node, NodeId, TransistorId, TransistorSimulator, TransistorType, VoltageLevel};
pub use wire::{Fanout, Net, Wire};

/// Prelude for common imports
pub mod prelude {
    pub use crate::{gate::*, signal::*, simulator::*, timing::*, wire::*};
}
