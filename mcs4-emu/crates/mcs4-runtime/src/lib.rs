//! Frontend-neutral MCS-4/MCS-40 simulation runtime.
//!
//! One worker owns the mutable emulator, the Intellec console, and the
//! peripherals that observe it. Frontends -- egui today, a 3D world later --
//! drive it through [`SimulationCommand`] and render the immutable
//! [`SimulationEvent`] stream and data snapshots. This crate depends only on the
//! lower emulation layers (`mcs4-system`, `mcs4-intellec`, `mcs4-periph`) and no
//! presentation framework, so nothing here reaches up into a UI.

pub mod dto;
pub mod scenario;
pub mod session;

pub use dto::{CpuSnapshot, MemoryRegion, MemorySnapshot, StackSnapshot};
pub use scenario::{rom_image_from_hex, Scenario, ROM_IMAGE_BYTES, SCENARIOS};
pub use session::{
    IntellecConsoleSnapshot, MachineSnapshot, SevenSegView, SimulationCommand, SimulationEvent, SimulationSession,
    SimulationSessionError, MAX_RUN_PHASES,
};
