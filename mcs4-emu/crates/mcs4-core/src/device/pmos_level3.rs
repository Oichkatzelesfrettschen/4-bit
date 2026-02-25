//! Unified Level 3 pMOS model combining all Alpha-stream enhancements.
//!
//! Packages the Level 1 model (body effect, subthreshold, DIBL, velocity
//! saturation) with region-dependent Meyer capacitances and junction
//! parasitics into a single device model suitable for transient simulation.
//!
//! When used with default ProcessParams, the DC behavior is identical to
//! PmosLevel1 (same Ids, gm, gds). The enhancement is in capacitance:
//! instead of a single static Cgate, the Level 3 model computes
//! Cgs/Cgd/Cgb as functions of the operating region and terminal voltages.

use super::cap_model::{MeyerCapacitances, MeyerParams};
use super::parasitic::TransistorParasitics;
use super::pmos_level1::PmosLevel1;
use super::{DeviceModel, MosRegion};
use crate::process::ProcessParams;

/// Overlap length for gate-source and gate-drain overlap capacitance (m).
/// Lateral diffusion under the gate in the 10um process: ~1.5um.
const OVERLAP_LENGTH: f64 = 1.5e-6;

/// Unified Level 3 pMOS model instance.
///
/// Combines:
/// - DC model: PmosLevel1 (body effect, subthreshold, DIBL, velocity saturation)
/// - Gate capacitances: MeyerParams (region-dependent Cgs/Cgd/Cgb)
/// - Junction capacitances: TransistorParasitics (Cdb, Csb from junction model)
///
/// The `DeviceModel` trait implementation delegates DC behavior to PmosLevel1.
/// Use `capacitances_at()` for the full operating-point-dependent capacitance
/// breakdown needed by the transient solver.
#[derive(Clone, Debug)]
pub struct PmosLevel3 {
    /// DC model (Ids, gm, gds, region detection).
    pub dc: PmosLevel1,
    /// Region-dependent gate capacitance parameters (Meyer model).
    pub meyer: MeyerParams,
    /// Junction and static parasitic capacitances.
    pub parasitics: TransistorParasitics,
}

/// Complete capacitance state at a given operating point.
///
/// Combines Meyer gate capacitances with junction capacitances
/// for a full picture of all capacitive coupling in the device.
#[derive(Clone, Copy, Debug)]
pub struct DeviceCapacitances {
    /// Gate-source capacitance (F): Meyer intrinsic + overlap.
    pub cgs: f64,
    /// Gate-drain capacitance (F): Meyer intrinsic + overlap.
    pub cgd: f64,
    /// Gate-bulk capacitance (F): Meyer model, large in cutoff.
    pub cgb: f64,
    /// Drain-bulk junction capacitance (F): from parasitic model.
    pub cdb: f64,
    /// Source-bulk junction capacitance (F): from parasitic model.
    pub csb: f64,
}

impl DeviceCapacitances {
    /// Total capacitance at the gate terminal.
    pub fn total_gate(&self) -> f64 {
        self.cgs + self.cgd + self.cgb
    }

    /// Total capacitance at the drain terminal.
    pub fn total_drain(&self) -> f64 {
        self.cgd + self.cdb
    }

    /// Total capacitance at the source terminal.
    pub fn total_source(&self) -> f64 {
        self.cgs + self.csb
    }
}

impl PmosLevel3 {
    /// Create a Level 3 model from process parameters and geometry.
    ///
    /// DC behavior is identical to `PmosLevel1::new()`. Capacitance
    /// model adds region-dependent Meyer gate caps and junction parasitics.
    pub fn new(process: &ProcessParams, w: f64, l: f64) -> Self {
        let dc = PmosLevel1::new(process, w, l);

        let meyer = MeyerParams::new(
            process.t_ox,
            w,
            l,
            dc.vth_mag,
            OVERLAP_LENGTH,
        );

        let parasitics = TransistorParasitics::from_geometry(process, w, l);

        Self { dc, meyer, parasitics }
    }

    /// Create from an existing Level 1 model and process parameters.
    ///
    /// Useful when upgrading a simulation from Level 1 to Level 3
    /// without recomputing DC parameters.
    pub fn from_level1(dc: PmosLevel1, process: &ProcessParams) -> Self {
        let meyer = MeyerParams::new(
            process.t_ox,
            dc.w,
            dc.l,
            dc.vth_mag,
            OVERLAP_LENGTH,
        );

        let parasitics = TransistorParasitics::from_geometry(process, dc.w, dc.l);

        Self { dc, meyer, parasitics }
    }

    /// Compute all capacitances at a given operating point.
    ///
    /// Uses source-referenced pMOS voltages:
    /// - `vsg` >= 0 when gate is more negative than source (device turning on)
    /// - `vsd` >= 0 when drain is more negative than source (current flowing)
    pub fn capacitances_at(&self, vsg: f64, vsd: f64) -> DeviceCapacitances {
        let meyer_caps = self.meyer.capacitances(vsg, vsd);

        DeviceCapacitances {
            cgs: meyer_caps.cgs,
            cgd: meyer_caps.cgd,
            cgb: meyer_caps.cgb,
            cdb: self.parasitics.cdb,
            csb: self.parasitics.csb,
        }
    }

    /// Compute gate capacitances only (Meyer model) at an operating point.
    pub fn meyer_capacitances(&self, vsg: f64, vsd: f64) -> MeyerCapacitances {
        self.meyer.capacitances(vsg, vsd)
    }

    /// Compute capacitances from absolute terminal voltages.
    ///
    /// Handles source/drain assignment (pMOS source = higher voltage terminal).
    pub fn capacitances_absolute(&self, vg: f64, vs: f64, vd: f64) -> DeviceCapacitances {
        let (vsg, vsd) = PmosLevel1::orient_voltages(vg, vs, vd);
        self.capacitances_at(vsg, vsd)
    }
}

impl DeviceModel for PmosLevel3 {
    fn ids(&self, vgs: f64, vds: f64) -> f64 {
        self.dc.ids(vgs, vds)
    }

    fn gm(&self, vgs: f64, vds: f64) -> f64 {
        self.dc.gm(vgs, vds)
    }

    fn gds(&self, vgs: f64, vds: f64) -> f64 {
        self.dc.gds(vgs, vds)
    }

    fn cgate(&self) -> f64 {
        // Return the intrinsic gate capacitance (Cox*W*L) as the nominal value.
        // For operating-point-dependent capacitance, use capacitances_at().
        self.meyer.c_intrinsic() + self.meyer.cov_gs + self.meyer.cov_gd
    }

    fn region(&self, vgs: f64, vds: f64) -> MosRegion {
        self.dc.region(vgs, vds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_process() -> ProcessParams {
        ProcessParams::default()
    }

    fn default_level3() -> PmosLevel3 {
        PmosLevel3::new(&default_process(), 10e-6, 10e-6)
    }

    // -- DC behavior matches Level 1 --

    #[test]
    fn dc_matches_level1() {
        let p = default_process();
        let l3 = PmosLevel3::new(&p, 10e-6, 10e-6);
        let l1 = PmosLevel1::new(&p, 10e-6, 10e-6);

        // Saturation current
        let ids_l3 = l3.ids(-10.0, -15.0);
        let ids_l1 = l1.ids(-10.0, -15.0);
        assert!(
            (ids_l3 - ids_l1).abs() < 1e-20,
            "Level 3 Ids should match Level 1: L3={:.3e}, L1={:.3e}", ids_l3, ids_l1
        );

        // Transconductance
        let gm_l3 = l3.gm(-10.0, -15.0);
        let gm_l1 = l1.gm(-10.0, -15.0);
        assert!(
            (gm_l3 - gm_l1).abs() < 1e-20,
            "Level 3 gm should match Level 1"
        );

        // Output conductance
        let gds_l3 = l3.gds(-10.0, -15.0);
        let gds_l1 = l1.gds(-10.0, -15.0);
        assert!(
            (gds_l3 - gds_l1).abs() < 1e-20,
            "Level 3 gds should match Level 1"
        );

        // Region
        assert_eq!(l3.region(-10.0, -15.0), l1.region(-10.0, -15.0));
        assert_eq!(l3.region(-10.0, -1.0), l1.region(-10.0, -1.0));
        assert_eq!(l3.region(0.0, -15.0), l1.region(0.0, -15.0));
    }

    #[test]
    fn dc_matches_level1_cutoff() {
        let p = default_process();
        let l3 = PmosLevel3::new(&p, 10e-6, 10e-6);
        let l1 = PmosLevel1::new(&p, 10e-6, 10e-6);

        let ids_l3 = l3.ids(0.0, -15.0);
        let ids_l1 = l1.ids(0.0, -15.0);
        assert!(
            (ids_l3 - ids_l1).abs() < 1e-25,
            "Cutoff current should match: L3={:.3e}, L1={:.3e}", ids_l3, ids_l1
        );
    }

    // -- Region-dependent capacitances --

    #[test]
    fn cutoff_capacitance_dominated_by_cgb() {
        let l3 = default_level3();
        // Vsg=0, Vsd=5: deeply OFF
        let caps = l3.capacitances_at(0.0, 5.0);

        assert!(caps.cgb > caps.cgs,
            "Cgb should dominate in cutoff: Cgb={:.3e}, Cgs={:.3e}", caps.cgb, caps.cgs);
        assert!(caps.cgb > caps.cgd,
            "Cgb should dominate in cutoff: Cgb={:.3e}, Cgd={:.3e}", caps.cgb, caps.cgd);
    }

    #[test]
    fn saturation_capacitance_two_thirds_to_source() {
        let l3 = default_level3();
        // Vsg=10, Vsd=10 (Vov=7, Vsd>Vov => saturation)
        let caps = l3.capacitances_at(10.0, 10.0);

        let c_int = l3.meyer.c_intrinsic();
        let expected_cgs = l3.meyer.cov_gs + (2.0 / 3.0) * c_int;

        assert!(
            (caps.cgs - expected_cgs).abs() < 1e-20,
            "Saturation Cgs should be Cov + 2/3*Cint"
        );
        assert!(caps.cgb.abs() < 1e-20, "Cgb should be zero in saturation");
    }

    #[test]
    fn triode_capacitance_symmetric_at_vsd_zero() {
        let l3 = default_level3();
        // Vsg=5 (Vov=2), Vsd=0 (symmetric triode)
        let caps = l3.capacitances_at(5.0, 0.0);

        assert!(
            (caps.cgs - caps.cgd).abs() / caps.cgs < 0.01,
            "Cgs and Cgd should be equal at Vsd=0: Cgs={:.3e}, Cgd={:.3e}",
            caps.cgs, caps.cgd
        );
    }

    // -- Junction capacitances --

    #[test]
    fn junction_caps_always_present() {
        let l3 = default_level3();

        // Check in all regions
        for &(vsg, vsd) in &[(0.0, 5.0), (10.0, 10.0), (5.0, 1.0)] {
            let caps = l3.capacitances_at(vsg, vsd);
            assert!(caps.cdb > 0.0, "Cdb should be positive at Vsg={}, Vsd={}", vsg, vsd);
            assert!(caps.csb > 0.0, "Csb should be positive at Vsg={}, Vsd={}", vsg, vsd);
        }
    }

    #[test]
    fn total_drain_cap_includes_junction() {
        let l3 = default_level3();
        let caps = l3.capacitances_at(10.0, 5.0);

        assert!(
            caps.total_drain() > caps.cgd,
            "Total drain cap should include junction: total={:.3e}, Cgd={:.3e}",
            caps.total_drain(), caps.cgd
        );
    }

    // -- Absolute voltage interface --

    #[test]
    fn capacitances_absolute_matches_oriented() {
        let l3 = default_level3();

        // Gate=-10V, Source=0V, Drain=-15V => Vsg=10, Vsd=15
        let caps_abs = l3.capacitances_absolute(-10.0, 0.0, -15.0);
        let caps_ori = l3.capacitances_at(10.0, 15.0);

        assert!((caps_abs.cgs - caps_ori.cgs).abs() < 1e-20);
        assert!((caps_abs.cgd - caps_ori.cgd).abs() < 1e-20);
        assert!((caps_abs.cgb - caps_ori.cgb).abs() < 1e-20);
    }

    // -- Construction from Level 1 --

    #[test]
    fn from_level1_preserves_dc() {
        let p = default_process();
        let l1 = PmosLevel1::new(&p, 10e-6, 10e-6);
        let l3_direct = PmosLevel3::new(&p, 10e-6, 10e-6);
        let l3_from = PmosLevel3::from_level1(l1, &p);

        let ids_direct = l3_direct.ids(-10.0, -15.0);
        let ids_from = l3_from.ids(-10.0, -15.0);
        assert!(
            (ids_direct - ids_from).abs() < 1e-20,
            "from_level1 should give same DC: direct={:.3e}, from={:.3e}",
            ids_direct, ids_from
        );
    }

    // -- DeviceModel trait --

    #[test]
    fn cgate_trait_returns_nominal() {
        let l3 = default_level3();
        let cg = l3.cgate();
        assert!(cg > 0.0, "cgate() should be positive");

        // Should be approximately Cox*W*L + 2*Cov
        let c_int = l3.meyer.c_intrinsic();
        let c_total = c_int + l3.meyer.cov_gs + l3.meyer.cov_gd;
        assert!(
            (cg - c_total).abs() < 1e-20,
            "cgate() should be Cint + Cov_gs + Cov_gd"
        );
    }

    // -- Geometry scaling --

    #[test]
    fn wider_device_more_capacitance() {
        let p = default_process();
        let narrow = PmosLevel3::new(&p, 10e-6, 10e-6);
        let wide = PmosLevel3::new(&p, 20e-6, 10e-6);

        let caps_n = narrow.capacitances_at(10.0, 5.0);
        let caps_w = wide.capacitances_at(10.0, 5.0);

        assert!(caps_w.cgs > caps_n.cgs, "Wider device should have more Cgs");
        assert!(caps_w.cgd > caps_n.cgd, "Wider device should have more Cgd");
        assert!(caps_w.cdb > caps_n.cdb, "Wider device should have more Cdb");
    }

    #[test]
    fn level3_all_regions_traversable() {
        let l3 = default_level3();

        // Cutoff
        assert_eq!(l3.region(0.0, -15.0), MosRegion::Cutoff);
        let caps = l3.capacitances_at(0.0, 15.0);
        assert!(caps.cgb > 0.0);

        // Triode
        assert_eq!(l3.region(-10.0, -1.0), MosRegion::Triode);
        let caps = l3.capacitances_at(10.0, 1.0);
        assert!(caps.cgs > caps.cgb);

        // Saturation
        assert_eq!(l3.region(-10.0, -15.0), MosRegion::Saturation);
        let caps = l3.capacitances_at(10.0, 15.0);
        assert!(caps.cgb.abs() < 1e-20);
    }
}
