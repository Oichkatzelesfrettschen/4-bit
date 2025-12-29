//! FPGA Synthesis Support for MCS-4/MCS-40
//!
//! This crate provides tools to export the gate-level design
//! to Verilog for FPGA synthesis.

pub mod verilog;

pub use verilog::VerilogExporter;
