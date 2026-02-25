//! Region-dependent Meyer capacitance model with Ward-Dutton charge partition.
//!
//! The Meyer model makes gate capacitances functions of the operating region:
//! - **Cutoff**: Channel does not exist; gate couples entirely to bulk
//!   (Cgb = Cox*W*L, Cgs = Cov, Cgd = Cov)
//! - **Saturation**: Channel pinched off at drain; gate couples to source
//!   (Cgs = Cov + 2/3 * Cox*W*L, Cgd = Cov, Cgb ~ 0)
//! - **Triode**: Full channel; gate charge shared between source and drain
//!   (Cgs = Cov + Cox*W*L * f(Vgs,Vds), Cgd = Cov + Cox*W*L * g(Vgs,Vds))
//!
//! The Ward-Dutton charge partition ensures charge conservation at region
//! boundaries by computing total charge Q as a function of terminal voltages,
//! then deriving capacitances as partial derivatives C = dQ/dV.

use crate::device::MosRegion;
use crate::process::oxide;

/// Meyer model capacitances for a single operating point.
#[derive(Clone, Copy, Debug)]
pub struct MeyerCapacitances {
    /// Gate-source capacitance (F).
    pub cgs: f64,
    /// Gate-drain capacitance (F).
    pub cgd: f64,
    /// Gate-bulk capacitance (F).
    pub cgb: f64,
}

impl MeyerCapacitances {
    /// Total gate capacitance.
    pub fn total_gate(&self) -> f64 {
        self.cgs + self.cgd + self.cgb
    }
}

/// Parameters for the Meyer capacitance model.
#[derive(Clone, Debug)]
pub struct MeyerParams {
    /// Gate oxide capacitance per unit area (F/m^2).
    pub cox: f64,
    /// Gate width (m).
    pub w: f64,
    /// Gate length (m).
    pub l: f64,
    /// Gate-source overlap capacitance (F).
    pub cov_gs: f64,
    /// Gate-drain overlap capacitance (F).
    pub cov_gd: f64,
    /// Threshold voltage magnitude (V, positive).
    pub vth_mag: f64,
}

impl MeyerParams {
    /// Create from process parameters and transistor geometry.
    ///
    /// # Arguments
    /// * `t_ox` - gate oxide thickness (m)
    /// * `w` - gate width (m)
    /// * `l` - gate length (m)
    /// * `vth_mag` - threshold voltage magnitude (V, positive)
    /// * `overlap_length` - lateral diffusion under gate (m), typically 1-2um
    pub fn new(t_ox: f64, w: f64, l: f64, vth_mag: f64, overlap_length: f64) -> Self {
        let cox = oxide::cox(t_ox);
        let cov = cox * w * overlap_length;
        Self {
            cox,
            w,
            l,
            cov_gs: cov,
            cov_gd: cov,
            vth_mag,
        }
    }

    /// Intrinsic gate capacitance (no overlap): Cox * W * L.
    pub fn c_intrinsic(&self) -> f64 {
        self.cox * self.w * self.l
    }

    /// Compute Meyer capacitances at a given operating point.
    ///
    /// Uses source-referenced pMOS voltages:
    /// - `vsg` >= 0 when gate is more negative than source (device turning on)
    /// - `vsd` >= 0 when drain is more negative than source (current flowing)
    ///
    /// The Meyer model with Ward-Dutton partition:
    /// - Cutoff: all charge on bulk
    /// - Saturation: 2/3 charge on source
    /// - Triode: charge distributed between source and drain via smooth partition
    pub fn capacitances(&self, vsg: f64, vsd: f64) -> MeyerCapacitances {
        let c_int = self.c_intrinsic();
        let vov = vsg - self.vth_mag; // Overdrive (positive when ON)

        if vov <= 0.0 {
            // Cutoff: no channel, gate couples to bulk
            MeyerCapacitances {
                cgs: self.cov_gs,
                cgd: self.cov_gd,
                cgb: c_int,
            }
        } else if vsd >= vov {
            // Saturation: channel pinched off at drain
            // 2/3 of channel charge associates with source terminal
            MeyerCapacitances {
                cgs: self.cov_gs + (2.0 / 3.0) * c_int,
                cgd: self.cov_gd,
                cgb: 0.0,
            }
        } else {
            // Triode: full channel from source to drain
            // Ward-Dutton charge partition for charge-conserving transitions.
            //
            // Total inversion charge: Qi = Cox*W*L * (Vov - Vsd/2)
            // Ward-Dutton splits Qi between source and drain terminals:
            //   Cgs = (2/3)*Cox*W*L * [1 - ((Vov - Vsd)/(2*Vov - Vsd))^2]
            //   Cgd = (2/3)*Cox*W*L * [1 - (Vov/(2*Vov - Vsd))^2]
            //
            // The 2/3 prefactor ensures charge conservation:
            //   At Vsd=0: Cgs = Cgd = 0.5*Cox*W*L  (symmetric, sum = Cint)
            //   At Vsd=Vov: Cgs -> 2/3*Cint, Cgd -> 0  (matches saturation)
            let denom = 2.0 * vov - vsd;
            if denom.abs() < 1e-15 {
                // Degenerate: treat as saturation
                MeyerCapacitances {
                    cgs: self.cov_gs + (2.0 / 3.0) * c_int,
                    cgd: self.cov_gd,
                    cgb: 0.0,
                }
            } else {
                let ratio_s = (vov - vsd) / denom;
                let ratio_d = vov / denom;
                let cgs_int = (2.0 / 3.0) * c_int * (1.0 - ratio_s * ratio_s);
                let cgd_int = (2.0 / 3.0) * c_int * (1.0 - ratio_d * ratio_d);

                MeyerCapacitances {
                    cgs: self.cov_gs + cgs_int,
                    cgd: self.cov_gd + cgd_int,
                    cgb: 0.0,
                }
            }
        }
    }

    /// Compute capacitances from the operating region and voltages.
    ///
    /// Convenience wrapper that takes the MosRegion enum directly.
    pub fn capacitances_for_region(
        &self,
        region: MosRegion,
        vsg: f64,
        vsd: f64,
    ) -> MeyerCapacitances {
        match region {
            MosRegion::Cutoff => MeyerCapacitances {
                cgs: self.cov_gs,
                cgd: self.cov_gd,
                cgb: self.c_intrinsic(),
            },
            MosRegion::Saturation => MeyerCapacitances {
                cgs: self.cov_gs + (2.0 / 3.0) * self.c_intrinsic(),
                cgd: self.cov_gd,
                cgb: 0.0,
            },
            MosRegion::Triode => self.capacitances(vsg, vsd),
        }
    }
}

/// Compute total node capacitance contributions from Meyer model.
///
/// Returns (cap_at_gate, cap_at_source, cap_at_drain) -- the lumped
/// capacitance each terminal sees from this transistor's gate capacitances.
/// Does NOT include junction capacitances (Cdb, Csb) -- those come from
/// the parasitic module.
pub fn node_capacitances(caps: &MeyerCapacitances) -> (f64, f64, f64) {
    let gate_cap = caps.cgs + caps.cgd + caps.cgb;
    let source_cap = caps.cgs;
    let drain_cap = caps.cgd;
    (gate_cap, source_cap, drain_cap)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_meyer() -> MeyerParams {
        // 10um process: tox=120nm, W=10um, L=10um, Vth=3V, overlap=1.5um
        MeyerParams::new(120e-9, 10e-6, 10e-6, 3.0, 1.5e-6)
    }

    #[test]
    fn intrinsic_capacitance_positive() {
        let p = default_meyer();
        assert!(p.c_intrinsic() > 0.0);
        // Cox = eps_ox/tox = 3.45e-11/120e-9 = 2.878e-4 F/m^2
        // C_int = Cox * W * L = 2.878e-4 * 10e-6 * 10e-6 = 2.878e-14 F (~29 fF)
        let c_int = p.c_intrinsic();
        assert!(
            c_int > 1e-15 && c_int < 1e-12,
            "C_int = {:.3e}, expected tens-of-fF range", c_int
        );
    }

    #[test]
    fn cutoff_all_capacitance_to_bulk() {
        let p = default_meyer();
        // Vsg = 0 (OFF), Vsd = 5V
        let caps = p.capacitances(0.0, 5.0);

        assert!(caps.cgb > 0.0, "Cgb should be large in cutoff");
        assert!(
            (caps.cgs - p.cov_gs).abs() < 1e-20,
            "Cgs should be just overlap in cutoff"
        );
        assert!(
            (caps.cgd - p.cov_gd).abs() < 1e-20,
            "Cgd should be just overlap in cutoff"
        );
        assert!(
            (caps.cgb - p.c_intrinsic()).abs() < 1e-20,
            "Cgb should be full intrinsic in cutoff"
        );
    }

    #[test]
    fn saturation_two_thirds_to_source() {
        let p = default_meyer();
        // Vsg = 10V (well above Vth=3V), Vsd = 10V (in saturation: Vsd > Vov=7)
        let caps = p.capacitances(10.0, 10.0);

        let expected_cgs = p.cov_gs + (2.0 / 3.0) * p.c_intrinsic();
        assert!(
            (caps.cgs - expected_cgs).abs() < 1e-20,
            "Cgs should be Cov + 2/3*Cint in saturation"
        );
        assert!(
            (caps.cgd - p.cov_gd).abs() < 1e-20,
            "Cgd should be just overlap in saturation"
        );
        assert!(
            caps.cgb.abs() < 1e-20,
            "Cgb should be zero in saturation"
        );
    }

    #[test]
    fn triode_symmetric_at_vsd_zero() {
        let p = default_meyer();
        // Vsg = 5V (Vov = 2V), Vsd = 0 (symmetric)
        let caps = p.capacitances(5.0, 0.0);

        // At Vsd=0 with Ward-Dutton 2/3 factor:
        //   ratio_s = ratio_d = Vov/(2*Vov) = 0.5
        //   Cgs_int = Cgd_int = (2/3)*Cint*(1 - 0.25) = 0.5*Cint
        //   Total intrinsic = Cint (charge conserved)
        let expected = p.cov_gs + 0.5 * p.c_intrinsic();
        assert!(
            (caps.cgs - expected).abs() / expected < 0.01,
            "Cgs at Vsd=0 should be Cov + 0.5*Cint: got {:.3e}, expected {:.3e}",
            caps.cgs, expected
        );

        // By symmetry, Cgs = Cgd when Vsd=0 (overlap caps are equal)
        assert!(
            (caps.cgs - caps.cgd).abs() / caps.cgs < 0.01,
            "Cgs and Cgd should be nearly equal at Vsd=0"
        );
    }

    #[test]
    fn triode_approaches_saturation_at_boundary() {
        let p = default_meyer();
        // Vov = 5V, Vsd approaching Vov
        let vov = 5.0;
        let vsg = vov + p.vth_mag; // = 8V
        let vsd = vov - 0.001; // just below saturation

        let caps_triode = p.capacitances(vsg, vsd);

        // Should approach saturation values: Cgs -> 2/3 * Cint + Cov
        let sat_cgs = p.cov_gs + (2.0 / 3.0) * p.c_intrinsic();
        assert!(
            (caps_triode.cgs - sat_cgs).abs() / sat_cgs < 0.02,
            "Cgs near saturation boundary should approach 2/3*Cint"
        );

        // Cgd should approach overlap only
        assert!(
            (caps_triode.cgd - p.cov_gd).abs() / p.c_intrinsic() < 0.02,
            "Cgd near saturation boundary should approach Cov"
        );
    }

    #[test]
    fn total_gate_cap_continuous_across_regions() {
        let p = default_meyer();
        let vsg = 10.0; // well above threshold, Vov = 7V

        // Sweep Vsd from 0 to 15V (crosses triode->saturation at Vsd=Vov=7V)
        let n_steps = 1000;
        let vsd_max = 15.0;
        let mut prev_total = 0.0;
        let mut max_jump = 0.0_f64;
        for i in 0..n_steps {
            let vsd = vsd_max * (i as f64) / (n_steps as f64);
            let caps = p.capacitances(vsg, vsd);
            let total = caps.total_gate();

            if i > 0 {
                max_jump = max_jump.max((total - prev_total).abs());
            }
            prev_total = total;
        }

        // Ward-Dutton partition ensures smooth transitions. With fine stepping
        // the max jump per step should be small relative to intrinsic capacitance.
        assert!(
            max_jump / p.c_intrinsic() < 0.05,
            "Total gate cap should be continuous, max jump = {:.3e}, Cint = {:.3e}",
            max_jump, p.c_intrinsic()
        );
    }

    #[test]
    fn capacitances_for_region_matches_auto() {
        let p = default_meyer();

        // Cutoff
        let auto = p.capacitances(0.0, 5.0);
        let manual = p.capacitances_for_region(MosRegion::Cutoff, 0.0, 5.0);
        assert!((auto.cgs - manual.cgs).abs() < 1e-20);
        assert!((auto.cgd - manual.cgd).abs() < 1e-20);
        assert!((auto.cgb - manual.cgb).abs() < 1e-20);

        // Saturation
        let auto = p.capacitances(10.0, 10.0);
        let manual = p.capacitances_for_region(MosRegion::Saturation, 10.0, 10.0);
        assert!((auto.cgs - manual.cgs).abs() < 1e-20);
    }

    #[test]
    fn wider_device_more_capacitance() {
        let narrow = MeyerParams::new(120e-9, 10e-6, 10e-6, 3.0, 1.5e-6);
        let wide = MeyerParams::new(120e-9, 20e-6, 10e-6, 3.0, 1.5e-6);

        let caps_n = narrow.capacitances(10.0, 5.0);
        let caps_w = wide.capacitances(10.0, 5.0);

        assert!(caps_w.cgs > caps_n.cgs, "Wider device should have more Cgs");
        assert!(caps_w.cgd > caps_n.cgd, "Wider device should have more Cgd");
    }

    #[test]
    fn node_capacitances_sum_correctly() {
        let p = default_meyer();
        let caps = p.capacitances(10.0, 5.0);
        let (gate_c, src_c, drn_c) = node_capacitances(&caps);

        assert!((gate_c - caps.total_gate()).abs() < 1e-20);
        assert!((src_c - caps.cgs).abs() < 1e-20);
        assert!((drn_c - caps.cgd).abs() < 1e-20);
    }

    #[test]
    fn charge_conservation_triode_sweep() {
        // Ward-Dutton charge partition conserves charge: the sum of intrinsic
        // Cgs and Cgd should not exceed Cox*W*L at any operating point.
        // At Vsd=0: sum = Cint (exact). As Vsd -> Vov: sum -> 2/3*Cint.
        let p = default_meyer();
        let c_int = p.c_intrinsic();
        let vov = 5.0;
        let vsg = vov + p.vth_mag;

        for i in 0..50 {
            let vsd = vov * (i as f64) / 50.0;
            let caps = p.capacitances(vsg, vsd);
            let intrinsic_sum = (caps.cgs - p.cov_gs) + (caps.cgd - p.cov_gd);

            assert!(
                intrinsic_sum <= c_int * 1.001, // small tolerance
                "Intrinsic Cgs+Cgd should not exceed Cint: {:.3e} > {:.3e} at Vsd={:.2}",
                intrinsic_sum, c_int, vsd
            );
            assert!(
                intrinsic_sum > 0.0,
                "Intrinsic capacitances should be positive at Vsd={:.2}", vsd
            );
        }
    }
}
