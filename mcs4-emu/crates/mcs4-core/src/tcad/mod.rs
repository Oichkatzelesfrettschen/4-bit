//! TCAD-class semiconductor device physics engine.
//!
//! Provides numerical solutions to semiconductor equations on spatial
//! meshes, bridging between material properties (process parameters)
//! and compact device models. These solvers compute device parameters
//! from first principles rather than using curve-fit equations.
//!
//! # Modules
//!
//! - [`mesh`] - 1D non-uniform spatial mesh with adaptive refinement
//! - [`carrier`] - Carrier statistics (Boltzmann, Fermi-Dirac)
//! - [`poisson`] - 1D Poisson-Boltzmann electrostatic solver

pub mod bridge;
pub mod carrier;
pub mod channel;
pub mod drift_diffusion;
pub mod mesh;
pub mod poisson;

pub use bridge::{DeviceResult, TCADBridge};
pub use carrier::{CarrierStats, ChargeProfile};
pub use channel::{ChannelConfig, ChannelMesh};
pub use drift_diffusion::{DriftDiffusionConfig, DriftDiffusionResult, DriftDiffusionSolver};
pub use mesh::Mesh1D;
pub use poisson::{PoissonConfig, PoissonResult, PoissonSolver};
