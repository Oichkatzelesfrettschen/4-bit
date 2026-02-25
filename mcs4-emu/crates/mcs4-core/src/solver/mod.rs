//! SPICE-class circuit simulation engine.
//!
//! Provides Modified Nodal Analysis (MNA) matrix assembly, Newton-Raphson
//! DC operating point solver, backward Euler transient stepping, and
//! convergence aids. Designed to simulate the 37-3500 transistor circuits
//! of the MCS-4 chip family.
//!
//! # Architecture
//!
//! The solver operates on a [`CircuitGraph`](crate::circuit::CircuitGraph)
//! populated from extracted netlists. It creates device model instances
//! for each transistor, assembles the MNA system, and iterates to find
//! the DC operating point using Newton-Raphson with convergence aids.

pub mod ac;
pub mod convergence;
pub mod dc_op;
pub mod event_scheduler;
pub mod matrix;
pub mod noise;
pub mod sensitivity;
pub mod sparse_matrix;
pub mod stimulus;
pub mod transient;

pub use ac::{AcConfig, AcResult, AcSolver};
pub use dc_op::{DcOpResult, DcSolver, SolverBackend, SPARSE_THRESHOLD};
pub use matrix::MnaSystem;
pub use noise::{NoiseConfig, NoiseResult, TransistorNoise};
pub use sensitivity::{ParamSensitivityResult, SensitivityParam, SensitivityResult};
pub use sparse_matrix::SparseMnaSystem;
pub use stimulus::{PulseSource, PwlSource, Stimulus, Waveform};
pub use transient::{TransientConfig, TransientResult, TransientSolver, WaveformPoint};

/// Solver configuration parameters.
#[derive(Clone, Debug)]
pub struct SolverConfig {
    /// Maximum Newton-Raphson iterations per operating point.
    pub max_nr_iterations: usize,
    /// Voltage convergence tolerance (V).
    pub v_tol: f64,
    /// Current convergence tolerance (A).
    pub i_tol: f64,
    /// Minimum conductance added for convergence (gmin, S).
    pub gmin: f64,
    /// Damping factor for Newton updates (0..1, 1 = no damping).
    pub damping: f64,
    /// Enable source stepping for initial convergence.
    pub source_stepping: bool,
    /// Number of source-stepping ramp steps.
    pub source_steps: usize,
    /// Matrix backend selection (dense/sparse/auto).
    pub backend: SolverBackend,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            max_nr_iterations: 100,
            v_tol: 1e-6,
            i_tol: 1e-9,
            gmin: 1e-12,
            damping: 1.0,
            source_stepping: false,
            source_steps: 10,
            backend: SolverBackend::Auto,
        }
    }
}

impl SolverConfig {
    /// Configuration tuned for small circuits (< 50 transistors).
    pub fn small_circuit() -> Self {
        Self {
            max_nr_iterations: 150,
            gmin: 1e-12,
            damping: 0.5,
            source_stepping: true,
            source_steps: 5,
            ..Self::default()
        }
    }

    /// Configuration with aggressive convergence aids for difficult circuits.
    pub fn robust() -> Self {
        Self {
            max_nr_iterations: 200,
            gmin: 1e-9,
            damping: 0.5,
            source_stepping: true,
            source_steps: 20,
            ..Self::default()
        }
    }
}
