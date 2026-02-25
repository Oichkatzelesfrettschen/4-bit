//! Parasitic capacitance models for pMOS transistors.
//!
//! Computes gate-source, gate-drain, drain-bulk, and source-bulk
//! capacitances from transistor geometry and process parameters.
//! These capacitances are essential for accurate transient simulation.

use crate::process::ProcessParams;
use crate::process::junction;
use crate::process::oxide;

/// Parasitic capacitances for a single transistor instance.
///
/// All values in Farads.
#[derive(Clone, Debug)]
pub struct TransistorParasitics {
    /// Gate-source overlap capacitance (F).
    pub cgs: f64,
    /// Gate-drain overlap capacitance (F).
    pub cgd: f64,
    /// Drain-bulk junction capacitance at zero bias (F).
    pub cdb: f64,
    /// Source-bulk junction capacitance at zero bias (F).
    pub csb: f64,
    /// Gate-bulk (channel) capacitance (F).
    pub cgb: f64,
}

/// Typical gate overlap per unit width (m).
/// For the 10um process, lateral diffusion ~ 1-2um under the gate.
const OVERLAP_LENGTH: f64 = 1.5e-6;

impl TransistorParasitics {
    /// Compute parasitic capacitances from geometry and process.
    ///
    /// # Arguments
    /// * `process` - semiconductor process parameters
    /// * `w` - gate width (m)
    /// * `l` - gate length (m)
    pub fn from_geometry(process: &ProcessParams, w: f64, l: f64) -> Self {
        let cox = oxide::cox(process.t_ox);

        // Gate overlap capacitances (gate-source and gate-drain)
        // C_ov = Cox * W * L_overlap
        let c_overlap = cox * w * OVERLAP_LENGTH;

        // Junction capacitances for source and drain
        // Assume source/drain diffusion area = W * (5 * L_min) approximately
        let diff_length = 5.0 * process.l_min;
        let area = w * diff_length;
        let cj0 = junction::cj0(process.n_sd, process.n_sub, process.temp);
        let c_junc = cj0 * area;

        // Add sidewall contribution
        let perimeter = 2.0 * (w + diff_length);
        let cjsw = junction::cjsw(cj0, process.x_j);
        let c_sw = cjsw * perimeter;

        let cdb = c_junc + c_sw;
        let csb = c_junc + c_sw;

        // Gate-bulk capacitance (in cutoff, full gate area couples to bulk)
        // In inversion this redistributes to Cgs/Cgd, but we store the
        // total for use by the solver.
        let cgb = cox * w * l;

        Self {
            cgs: c_overlap,
            cgd: c_overlap,
            cdb,
            csb,
            cgb,
        }
    }

    /// Total capacitance seen at the gate node (F).
    pub fn total_gate_cap(&self) -> f64 {
        self.cgs + self.cgd + self.cgb
    }

    /// Total capacitance seen at the drain node (F).
    pub fn total_drain_cap(&self) -> f64 {
        self.cgd + self.cdb
    }

    /// Total capacitance seen at the source node (F).
    pub fn total_source_cap(&self) -> f64 {
        self.cgs + self.csb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_parasitics() -> TransistorParasitics {
        let p = ProcessParams::default();
        TransistorParasitics::from_geometry(&p, 10e-6, 10e-6)
    }

    #[test]
    fn all_capacitances_positive() {
        let c = default_parasitics();
        assert!(c.cgs > 0.0, "cgs = {:.3e}", c.cgs);
        assert!(c.cgd > 0.0, "cgd = {:.3e}", c.cgd);
        assert!(c.cdb > 0.0, "cdb = {:.3e}", c.cdb);
        assert!(c.csb > 0.0, "csb = {:.3e}", c.csb);
        assert!(c.cgb > 0.0, "cgb = {:.3e}", c.cgb);
    }

    #[test]
    fn overlap_caps_equal() {
        let c = default_parasitics();
        assert!((c.cgs - c.cgd).abs() < 1e-20,
            "Symmetric device should have Cgs = Cgd");
    }

    #[test]
    fn junction_caps_equal_for_symmetric_device() {
        let c = default_parasitics();
        assert!((c.cdb - c.csb).abs() < 1e-20,
            "Symmetric device should have Cdb = Csb");
    }

    #[test]
    fn wider_device_more_capacitance() {
        let p = ProcessParams::default();
        let c_narrow = TransistorParasitics::from_geometry(&p, 10e-6, 10e-6);
        let c_wide = TransistorParasitics::from_geometry(&p, 20e-6, 10e-6);

        assert!(c_wide.cgs > c_narrow.cgs);
        assert!(c_wide.cdb > c_narrow.cdb);
        assert!(c_wide.cgb > c_narrow.cgb);
    }

    #[test]
    fn total_gate_cap_sums_correctly() {
        let c = default_parasitics();
        let expected = c.cgs + c.cgd + c.cgb;
        assert!((c.total_gate_cap() - expected).abs() < 1e-25);
    }

    #[test]
    fn capacitances_in_femtofarad_range() {
        let c = default_parasitics();
        // For 10um transistor, capacitances should be in the fF range (1e-15)
        let cg = c.total_gate_cap();
        assert!(cg > 1e-16 && cg < 1e-12,
            "Total gate cap = {:.3e} F, expected fF range", cg);
    }
}
