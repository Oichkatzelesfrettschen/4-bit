//! MCS-4/MCS-40 GUI Application
//!
//! Provides the graphical interface for the emulator, including register
//! displays, memory viewers, signal trace panels, and disassembly views.
#![allow(missing_docs)]

pub mod panels;
pub mod signal_trace;

pub mod app;
pub use app::Mcs4App;
