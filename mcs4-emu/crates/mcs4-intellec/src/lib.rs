//! Intellec-4 Development System Emulation
//!
//! The Intellec-4 was Intel's official development system for the MCS-4
//! chip family. It provided a front panel with toggle switches and LEDs
//! for entering programs, single-stepping, and examining memory.
//!
//! This crate emulates the Intellec-4 hardware:
//! - [`front_panel`]: Address/data switches, run/stop/step controls, LED indicators
//! - [`monitor`]: Monitor ROM state machine (examine, deposit, go, halt)
//! - [`prom_programmer`]: 4702 EPROM programming interface
//! - [`system`]: Complete system integration wiring all subsystems together

pub mod front_panel;
pub mod monitor;
pub mod prom_programmer;
pub mod system;

pub use front_panel::{FrontPanel, PanelLeds, PanelMode};
pub use monitor::{Monitor, MonitorAction, MonitorCommand};
pub use prom_programmer::{ProgResult, PromProgrammer};
pub use system::IntellecSystem;
