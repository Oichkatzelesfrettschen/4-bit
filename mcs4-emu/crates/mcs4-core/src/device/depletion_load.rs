//! Depletion-mode pMOS load transistor model.
//!
//! In the Intel 10um pMOS process, depletion-mode transistors are used as
//! active loads. Their gate is connected to their source, so they are always
//! conducting (Vgs = 0, but Vth > 0 means channel exists at zero gate bias).
//!
//! The depletion load acts as a current source/high-impedance load,
//! providing much better performance than resistive loads while requiring
//! less die area.

use super::{DeviceModel, MosRegion};
use crate::process::ProcessParams;

/// Depletion-mode pMOS load model.
///
/// Operates with gate tied to source (Vgs = 0 always). Since Vth is
/// positive for depletion mode, the device is always ON when Vgs = 0.
#[derive(Clone, Debug)]
pub struct DepletionLoadModel {
    /// Transconductance parameter: beta = mu * Cox * W/L (A/V^2).
    pub beta: f64,
    /// Threshold voltage magnitude (positive for depletion mode).
    pub vth_dep: f64,
    /// Channel-length modulation parameter (1/V).
    pub lambda: f64,
    /// Total gate capacitance (F).
    pub cg: f64,
    /// Gate width (m).
    pub w: f64,
    /// Gate length (m).
    pub l: f64,
}

impl DepletionLoadModel {
    /// Create a depletion-mode load from process parameters.
    pub fn new(process: &ProcessParams, w: f64, l: f64) -> Self {
        Self {
            beta: process.beta(w, l),
            vth_dep: process.vth_depletion,
            lambda: process.lambda,
            cg: process.cgate(w, l),
            w,
            l,
        }
    }

    /// Create with explicit parameters.
    pub fn with_params(beta: f64, vth_dep: f64, lambda: f64, cg: f64, w: f64, l: f64) -> Self {
        Self {
            beta,
            vth_dep,
            lambda,
            cg,
            w,
            l,
        }
    }

    /// Current through the load for given Vsd (source-drain voltage drop).
    ///
    /// With gate tied to source: Vsg = 0, so overdrive = Vth_dep.
    /// Triode when Vsd < Vth_dep, saturation when Vsd >= Vth_dep.
    pub fn ids_load(&self, vsd: f64) -> f64 {
        if vsd <= 0.0 {
            0.0
        } else if vsd < self.vth_dep {
            // Triode: Ids = beta * (Vth_dep * Vsd - 0.5 * Vsd^2) * (1 + lambda * Vsd)
            self.beta * (self.vth_dep * vsd - 0.5 * vsd * vsd) * (1.0 + self.lambda * vsd)
        } else {
            // Saturation: Ids = 0.5 * beta * Vth_dep^2 * (1 + lambda * Vsd)
            0.5 * self.beta * self.vth_dep * self.vth_dep * (1.0 + self.lambda * vsd)
        }
    }

    /// Output conductance of the load.
    pub fn gds_load(&self, vsd: f64) -> f64 {
        if vsd <= 0.0 {
            // At Vsd=0, gds = beta * Vth_dep (initial slope)
            self.beta * self.vth_dep
        } else if vsd < self.vth_dep {
            // Triode: dIds/dVsd
            self.beta * (self.vth_dep - vsd) * (1.0 + self.lambda * vsd)
                + self.beta * self.lambda * (self.vth_dep * vsd - 0.5 * vsd * vsd)
        } else {
            // Saturation: gds from CLM only
            0.5 * self.beta * self.vth_dep * self.vth_dep * self.lambda
        }
    }
}

impl DeviceModel for DepletionLoadModel {
    fn ids(&self, _vgs: f64, vds: f64) -> f64 {
        // Gate tied to source, so Vgs=0 always. Only Vds matters.
        // Vsd = -Vds for pMOS convention
        self.ids_load(-vds)
    }

    fn gm(&self, _vgs: f64, _vds: f64) -> f64 {
        // Gate is tied to source, so gm is effectively zero
        // (no external control of gate voltage)
        0.0
    }

    fn gds(&self, _vgs: f64, vds: f64) -> f64 {
        self.gds_load(-vds)
    }

    fn cgate(&self) -> f64 {
        self.cg
    }

    fn region(&self, _vgs: f64, vds: f64) -> MosRegion {
        let vsd = -vds;
        if vsd <= 0.0 {
            MosRegion::Cutoff
        } else if vsd < self.vth_dep {
            MosRegion::Triode
        } else {
            MosRegion::Saturation
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_default_load() -> DepletionLoadModel {
        let p = ProcessParams::default();
        DepletionLoadModel::new(&p, 10e-6, 20e-6) // W/L = 0.5 typical for load
    }

    #[test]
    fn always_conducting_at_positive_vsd() {
        let m = make_default_load();
        let ids = m.ids_load(5.0);
        assert!(ids > 0.0, "Depletion load should always conduct, Ids = {:.3e}", ids);
    }

    #[test]
    fn no_current_at_zero_vsd() {
        let m = make_default_load();
        let ids = m.ids_load(0.0);
        assert!((ids - 0.0).abs() < 1e-15);
    }

    #[test]
    fn triode_at_low_vsd() {
        let m = make_default_load();
        // Vsd = 0.5V < Vth_dep = 1.0V -> triode
        assert_eq!(m.region(0.0, -0.5), MosRegion::Triode);
    }

    #[test]
    fn saturation_at_high_vsd() {
        let m = make_default_load();
        // Vsd = 5V > Vth_dep = 1.0V -> saturation
        assert_eq!(m.region(0.0, -5.0), MosRegion::Saturation);
    }

    #[test]
    fn current_saturates() {
        let m = make_default_load();
        let ids_5v = m.ids_load(5.0);
        let ids_15v = m.ids_load(15.0);
        // In saturation, current should be roughly constant (within CLM)
        let ratio = ids_15v / ids_5v;
        assert!(ratio < 2.0, "Current should roughly saturate, ratio = {:.2}", ratio);
    }

    #[test]
    fn gm_is_zero() {
        let m = make_default_load();
        assert!(
            (m.gm(0.0, -10.0) - 0.0).abs() < 1e-15,
            "Depletion load gm should be zero (gate tied to source)"
        );
    }

    #[test]
    fn gds_positive_in_saturation() {
        let m = make_default_load();
        let gds = m.gds_load(10.0);
        assert!(gds > 0.0, "gds in saturation should be positive from CLM");
    }

    #[test]
    fn gds_larger_in_triode_than_saturation() {
        let m = make_default_load();
        let gds_triode = m.gds_load(0.5);
        let gds_sat = m.gds_load(10.0);
        assert!(
            gds_triode > gds_sat,
            "gds in triode ({:.3e}) should exceed saturation ({:.3e})",
            gds_triode,
            gds_sat
        );
    }
}
