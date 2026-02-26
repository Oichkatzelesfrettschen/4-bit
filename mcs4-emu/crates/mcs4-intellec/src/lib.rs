//! Intellec-4 development system emulation.
//!
//! The Intellec-4 was Intel's official development system for the MCS-4
//! chip family. It provided a front panel with toggle switches and LEDs
//! for entering programs, single-stepping, and examining memory.
//!
//! This crate will emulate the Intellec-4 hardware:
//! - Front panel (address/data switches, run/stop/step controls)
//! - Monitor ROM (bootstrap program)
//! - PROM programmer interface
//! - Teletype (ASR-33) serial I/O
//!
//! # Status
//!
//! Phase 5 scaffold. Not yet implemented.
