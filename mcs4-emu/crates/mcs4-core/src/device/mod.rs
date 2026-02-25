//! Transistor device models for the Intel 10um pMOS process.
//!
//! Provides physics-based device models suitable for SPICE-class circuit
//! simulation. The primary model is Shichman-Hodges (SPICE Level 1),
//! which is appropriate for the long-channel (10um) devices in the MCS-4
//! chip family.
//!
//! # Device types
//!
//! - **Enhancement-mode pMOS**: Used for logic switching. Gate must be driven
//!   below threshold to conduct (Vgs < Vth, with Vth negative).
//! - **Depletion-mode pMOS**: Used as active loads. Gate tied to source,
//!   always conducting. Positive threshold voltage.

pub mod depletion_load;
pub mod parasitic;
pub mod pmos_level1;

pub use depletion_load::DepletionLoadModel;
pub use parasitic::TransistorParasitics;
pub use pmos_level1::PmosLevel1;

/// Operating region of a MOSFET.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MosRegion {
    /// Gate-source voltage below threshold; essentially no current flows.
    Cutoff,
    /// Transistor is in the linear (ohmic/triode) region.
    /// Channel exists from source to drain.
    Triode,
    /// Channel is pinched off at drain end. Current is approximately
    /// constant with respect to Vds.
    Saturation,
}

/// Common interface for transistor device models.
///
/// All voltages use the standard MOSFET convention:
/// - Vgs = Vg - Vs (gate-source)
/// - Vds = Vd - Vs (drain-source)
///
/// For pMOS in the MCS-4 process:
/// - Source is typically at VSS (0V) or an internal node
/// - Drain is typically at VDD (-15V) or the output node
/// - Vgs and Vds are typically negative when the device is ON
pub trait DeviceModel {
    /// Drain-source current (A). Positive for conventional current
    /// flowing from source to drain in pMOS (holes).
    fn ids(&self, vgs: f64, vds: f64) -> f64;

    /// Transconductance gm = dIds/dVgs (S).
    fn gm(&self, vgs: f64, vds: f64) -> f64;

    /// Output conductance gds = dIds/dVds (S).
    fn gds(&self, vgs: f64, vds: f64) -> f64;

    /// Total gate capacitance (F).
    fn cgate(&self) -> f64;

    /// Determine the operating region.
    fn region(&self, vgs: f64, vds: f64) -> MosRegion;
}
