//! Shichman-Hodges (SPICE Level 1) pMOS transistor model.
//!
//! The Level 1 model is a square-law model that divides operation into
//! three regions: cutoff, triode (linear), and saturation. It includes
//! channel-length modulation (lambda) for finite output resistance in
//! saturation.
//!
//! For pMOS, all voltages are inverted relative to nMOS convention:
//! - The device conducts when Vsg > |Vth| (equivalently Vgs < Vth < 0)
//! - Current flows from source to drain (holes)
//!
//! We use the "source-referenced" convention internally:
//!   Vsg = Vs - Vg (positive when gate is pulled low)
//!   Vsd = Vs - Vd (positive when drain is at VDD = -15V)

use super::{DeviceModel, MosRegion};
use crate::process::ProcessParams;

/// Shichman-Hodges Level 1 pMOS model instance with body effect
/// and subthreshold conduction extensions.
///
/// Created from process parameters and transistor geometry.
/// When `gamma` is zero (default for backward compatibility),
/// behaves identically to the original Level 1 model.
#[derive(Clone, Debug)]
pub struct PmosLevel1 {
    /// Transconductance parameter: beta = mu * Cox * W/L (A/V^2).
    pub beta: f64,
    /// Zero-bias threshold voltage magnitude |Vth0| (positive value).
    pub vth_mag: f64,
    /// Channel-length modulation parameter (1/V).
    pub lambda: f64,
    /// Total gate capacitance (F).
    pub cg: f64,
    /// Gate width (m).
    pub w: f64,
    /// Gate length (m).
    pub l: f64,
    /// Body effect coefficient (V^0.5). Zero disables body effect.
    pub gamma: f64,
    /// Bulk Fermi potential phi_b = 2*phi_f (V). Used with gamma.
    pub phi_b: f64,
    /// Thermal voltage Vt = kT/q (V). Used for subthreshold.
    pub vt: f64,
    /// Subthreshold ideality factor n. Used for subthreshold.
    pub n_sub: f64,
    /// DIBL coefficient (V/V). Vth reduction per volt of Vds.
    pub eta_dibl: f64,
    /// Hole saturation velocity (cm/s). Used for velocity saturation.
    /// When zero, velocity saturation is disabled (pure square-law).
    pub v_sat: f64,
    /// Low-field mobility (cm^2/V*s). Used to compute Vdsat from v_sat.
    pub mu_0: f64,
}

impl PmosLevel1 {
    /// Create a Level 1 pMOS model from process parameters and geometry.
    ///
    /// Includes body effect and subthreshold parameters derived from process.
    ///
    /// # Arguments
    /// * `process` - semiconductor process parameters
    /// * `w` - gate width (meters)
    /// * `l` - gate length (meters)
    pub fn new(process: &ProcessParams, w: f64, l: f64) -> Self {
        Self {
            beta: process.beta(w, l),
            vth_mag: process.vth_enhancement.abs(),
            lambda: process.lambda,
            cg: process.cgate(w, l),
            w,
            l,
            gamma: process.gamma(),
            phi_b: process.phi_b(),
            vt: process.vt(),
            n_sub: process.subthreshold_n(),
            eta_dibl: process.eta_dibl,
            v_sat: crate::process::silicon::V_SAT_HOLES,
            mu_0: process.mu_0,
        }
    }

    /// Create with explicit parameters (for testing or custom devices).
    /// Body effect and subthreshold are disabled (zero gamma, zero eta).
    pub fn with_params(beta: f64, vth_mag: f64, lambda: f64, cg: f64, w: f64, l: f64) -> Self {
        Self {
            beta, vth_mag, lambda, cg, w, l,
            gamma: 0.0,
            phi_b: 0.0,
            vt: 0.02585,
            n_sub: 1.0,
            eta_dibl: 0.0,
            v_sat: 0.0,  // disabled
            mu_0: 175.0,
        }
    }

    /// Convert external node voltages (Vg, Vs, Vd absolute) to
    /// source-referenced (Vsg, Vsd) with proper pMOS orientation.
    ///
    /// For pMOS, the source is the terminal at higher voltage.
    /// Returns (vsg, vsd) both non-negative when device is conducting.
    pub fn orient_voltages(vg: f64, vs: f64, vd: f64) -> (f64, f64) {
        // For pMOS, source is at higher potential
        if vs >= vd {
            (vs - vg, vs - vd)
        } else {
            // Drain and source swap
            (vd - vg, vd - vs)
        }
    }

    /// Compute Ids from absolute terminal voltages (convenience wrapper).
    /// Assumes bulk is tied to source (zero body effect).
    pub fn ids_absolute(&self, vg: f64, vs: f64, vd: f64) -> f64 {
        let (vsg, vsd) = Self::orient_voltages(vg, vs, vd);
        self.ids_pmos(vsg, vsd, 0.0)
    }

    /// Compute Ids with explicit bulk voltage.
    ///
    /// For pMOS, Vsb is the source-to-bulk voltage. In an n-type
    /// substrate (bulk at VSS=0V), if the source is at an internal node
    /// above VDD, Vsb = Vs - Vb can be non-zero, shifting the threshold.
    pub fn ids_with_bulk(&self, vg: f64, vs: f64, vd: f64, vb: f64) -> f64 {
        let (vsg, vsd) = Self::orient_voltages(vg, vs, vd);
        // For pMOS, the source is at higher potential.
        // Vsb (source-bulk) in pMOS convention: for body effect we need
        // the magnitude of the reverse bias across the source-bulk junction.
        // In pMOS, substrate is n-type (at VSS=0V typically).
        // Source is the terminal at higher potential.
        let v_source = vs.max(vd);
        let vsb = v_source - vb;
        self.ids_pmos(vsg, vsd, vsb)
    }

    /// Effective threshold voltage magnitude with body effect and DIBL.
    ///
    /// |Vth_eff| = |Vth0| + gamma * (sqrt(phi_b + |Vsb|) - sqrt(phi_b)) - eta * |Vsd|
    ///
    /// For pMOS, Vsb is typically non-negative (source at higher potential
    /// than n-type bulk at 0V). Body effect increases |Vth|.
    /// DIBL decreases |Vth| with increasing |Vds|.
    pub fn vth_effective(&self, vsb: f64, vsd: f64) -> f64 {
        let mut vth = self.vth_mag;

        // Body effect: threshold increases with source-bulk reverse bias
        if self.gamma > 0.0 && self.phi_b > 0.0 {
            let vsb_abs = vsb.abs();
            let delta_vth = self.gamma
                * ((self.phi_b + vsb_abs).sqrt() - self.phi_b.sqrt());
            vth += delta_vth;
        }

        // DIBL: threshold decreases with drain-source voltage
        if self.eta_dibl > 0.0 {
            vth -= self.eta_dibl * vsd.abs();
            vth = vth.max(0.0); // prevent negative threshold magnitude
        }

        vth
    }

    /// Effective saturation voltage considering velocity saturation.
    ///
    /// Vdsat = min(Vov, v_sat * L / mu_eff)
    ///
    /// For long-channel devices (10um), the velocity-limited Vdsat is
    /// much larger than Vov, so this has minimal effect. For shorter
    /// channels, it naturally limits the saturation current.
    fn vdsat_effective(&self, vov: f64) -> f64 {
        if self.v_sat <= 0.0 || self.mu_0 <= 0.0 || self.l <= 0.0 {
            return vov;
        }

        // v_sat in cm/s, mu_0 in cm^2/V*s, l in meters
        // E_sat = v_sat / mu_0  (V/cm)
        // Vdsat_vsat = E_sat * L = v_sat * L_cm / mu_0
        let l_cm = self.l * 1e2; // m to cm
        let vdsat_vsat = self.v_sat * l_cm / self.mu_0;

        // Use harmonic mean for smooth transition:
        // 1/Vdsat_eff = 1/Vov + 1/Vdsat_vsat
        // Vdsat_eff = Vov * Vdsat_vsat / (Vov + Vdsat_vsat)
        if vov + vdsat_vsat > 0.0 {
            vov * vdsat_vsat / (vov + vdsat_vsat)
        } else {
            vov
        }
    }

    /// Drain current using pMOS source-referenced convention.
    ///
    /// Vsg = Vs - Vg (positive when gate is low, device tends ON)
    /// Vsd = Vs - Vd (positive when drain is at lower voltage)
    /// Vsb = Vs - Vb (source-bulk voltage, 0.0 for bulk tied to source)
    ///
    /// Returns current magnitude (positive when device conducts).
    /// Includes subthreshold conduction below threshold for smooth
    /// transition, body effect when gamma > 0, and velocity saturation.
    fn ids_pmos(&self, vsg: f64, vsd: f64, vsb: f64) -> f64 {
        let vth_eff = self.vth_effective(vsb, vsd);

        if vsg <= vth_eff {
            // Subthreshold region: exponential current
            self.ids_subthreshold(vsg, vsd, vth_eff)
        } else {
            let vov = vsg - vth_eff;
            let vdsat = self.vdsat_effective(vov);

            if vsd <= vdsat {
                // Triode (linear) region: Vsd < Vdsat
                self.beta * ((vov) * vsd - 0.5 * vsd * vsd)
                    * (1.0 + self.lambda * vsd)
            } else {
                // Saturation: Vsd >= Vdsat
                // Use Vdsat instead of Vov for velocity-saturated current
                0.5 * self.beta * vdsat * vdsat * (1.0 + self.lambda * vsd)
            }
        }
    }

    /// Subthreshold (weak inversion) current.
    ///
    /// Ids_sub = Ids0 * exp((Vsg - |Vth|) / (n * Vt)) * (1 - exp(-Vsd/Vt))
    ///
    /// Ids0 is the current at the threshold boundary, ensuring continuity
    /// with the strong-inversion model.
    fn ids_subthreshold(&self, vsg: f64, vsd: f64, vth_eff: f64) -> f64 {
        if self.vt <= 0.0 || vsd <= 0.0 {
            return 0.0;
        }

        // Current at the threshold boundary (Vsg = Vth, Vsd in saturation):
        // At Vsg = Vth_eff, Vov = 0, but we need a finite anchor.
        // Use the subthreshold swing to compute current that connects
        // smoothly to the strong-inversion saturation current.
        // Ids0 = 0.5 * beta * (n * Vt)^2 (current at Vsg = Vth + n*Vt)
        let n_vt = self.n_sub * self.vt;
        let ids0 = 0.5 * self.beta * n_vt * n_vt;

        // Exponential decay below threshold
        let exp_arg = (vsg - vth_eff) / n_vt;
        // Clamp to prevent underflow/overflow
        let exp_val = exp_arg.clamp(-40.0, 0.0).exp();

        // Drain voltage dependence (ensures zero current at Vsd=0)
        let drain_factor = 1.0 - (-vsd / self.vt).exp();

        ids0 * exp_val * drain_factor.max(0.0)
    }

    /// Transconductance in pMOS convention (S).
    /// Uses Vsb=0 (bulk tied to source) for backward compatibility.
    fn gm_pmos(&self, vsg: f64, vsd: f64) -> f64 {
        self.gm_pmos_body(vsg, vsd, 0.0)
    }

    /// Transconductance with body effect and velocity saturation.
    fn gm_pmos_body(&self, vsg: f64, vsd: f64, vsb: f64) -> f64 {
        let vth_eff = self.vth_effective(vsb, vsd);

        if vsg <= vth_eff {
            // Subthreshold gm = Ids / (n * Vt)
            let ids = self.ids_subthreshold(vsg, vsd, vth_eff);
            if self.vt > 0.0 {
                ids / (self.n_sub * self.vt)
            } else {
                0.0
            }
        } else {
            let vov = vsg - vth_eff;
            let vdsat = self.vdsat_effective(vov);

            if vsd <= vdsat {
                // Triode: gm = beta * Vsd * (1 + lambda * Vsd)
                self.beta * vsd * (1.0 + self.lambda * vsd)
            } else {
                // Saturation: gm = beta * Vdsat * (1 + lambda * Vsd)
                // With velocity saturation, dVdsat/dVov reduces gm
                let dvdsat_dvov = if self.v_sat > 0.0 && self.mu_0 > 0.0 && self.l > 0.0 {
                    let l_cm = self.l * 1e2;
                    let vdsat_vsat = self.v_sat * l_cm / self.mu_0;
                    let denom = vov + vdsat_vsat;
                    if denom > 0.0 {
                        (vdsat_vsat * vdsat_vsat) / (denom * denom)
                    } else {
                        1.0
                    }
                } else {
                    1.0 // no velocity saturation -> dVdsat/dVov = 1
                };
                self.beta * vdsat * dvdsat_dvov * (1.0 + self.lambda * vsd)
            }
        }
    }

    /// Output conductance in pMOS convention (S).
    /// Uses Vsb=0 (bulk tied to source) for backward compatibility.
    fn gds_pmos(&self, vsg: f64, vsd: f64) -> f64 {
        self.gds_pmos_body(vsg, vsd, 0.0)
    }

    /// Output conductance with body effect and velocity saturation.
    fn gds_pmos_body(&self, vsg: f64, vsd: f64, vsb: f64) -> f64 {
        let vth_eff = self.vth_effective(vsb, vsd);

        if vsg <= vth_eff {
            // Subthreshold: gds from drain factor derivative
            if self.vt > 0.0 && vsd > 0.0 {
                let drain_deriv = (-vsd / self.vt).exp() / self.vt;
                let n_vt = self.n_sub * self.vt;
                let ids0 = 0.5 * self.beta * n_vt * n_vt;
                let exp_arg = ((vsg - vth_eff) / n_vt).clamp(-40.0, 0.0);
                ids0 * exp_arg.exp() * drain_deriv
            } else {
                0.0
            }
        } else {
            let vov = vsg - vth_eff;
            let vdsat = self.vdsat_effective(vov);

            if vsd <= vdsat {
                // Triode: gds = beta * (Vov - Vsd) * (1 + lambda*Vsd)
                //              + beta * lambda * (Vov*Vsd - 0.5*Vsd^2)
                self.beta * (vov - vsd) * (1.0 + self.lambda * vsd)
                    + self.beta * self.lambda * (vov * vsd - 0.5 * vsd * vsd)
            } else {
                // Saturation: gds = 0.5 * beta * Vdsat^2 * lambda
                0.5 * self.beta * vdsat * vdsat * self.lambda
            }
        }
    }

    fn region_pmos(&self, vsg: f64, vsd: f64) -> MosRegion {
        self.region_pmos_body(vsg, vsd, 0.0)
    }

    fn region_pmos_body(&self, vsg: f64, vsd: f64, vsb: f64) -> MosRegion {
        let vth_eff = self.vth_effective(vsb, vsd);
        if vsg <= vth_eff {
            MosRegion::Cutoff
        } else {
            let vov = vsg - vth_eff;
            let vdsat = self.vdsat_effective(vov);
            if vsd <= vdsat {
                MosRegion::Triode
            } else {
                MosRegion::Saturation
            }
        }
    }
}

impl DeviceModel for PmosLevel1 {
    fn ids(&self, vgs: f64, vds: f64) -> f64 {
        // Convert from Vgs/Vds to Vsg/Vsd (negate both)
        // Vsb=0 (bulk tied to source) for DeviceModel trait compatibility
        let vsg = -vgs;
        let vsd = -vds;
        self.ids_pmos(vsg, vsd, 0.0)
    }

    fn gm(&self, vgs: f64, vds: f64) -> f64 {
        let vsg = -vgs;
        let vsd = -vds;
        self.gm_pmos(vsg, vsd)
    }

    fn gds(&self, vgs: f64, vds: f64) -> f64 {
        let vsg = -vgs;
        let vsd = -vds;
        self.gds_pmos(vsg, vsd)
    }

    fn cgate(&self) -> f64 {
        self.cg
    }

    fn region(&self, vgs: f64, vds: f64) -> MosRegion {
        let vsg = -vgs;
        let vsd = -vds;
        self.region_pmos(vsg, vsd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_default_transistor() -> PmosLevel1 {
        let p = ProcessParams::default();
        PmosLevel1::new(&p, 10e-6, 10e-6)
    }

    #[test]
    fn cutoff_when_gate_at_source() {
        let m = make_default_transistor();
        // Vgs = 0 (gate = source), Vds = -15V
        let ids = m.ids(0.0, -15.0);
        // With subthreshold model, current is not exactly zero but negligibly small
        // exp((0 - 3) / (n * 0.026)) ~ exp(-89) ~ 0
        assert!(ids.abs() < 1e-12,
            "Should be negligible in deep cutoff, Ids = {:.3e}", ids);
        assert_eq!(m.region(0.0, -15.0), MosRegion::Cutoff);
    }

    #[test]
    fn conducts_when_gate_driven_low() {
        let m = make_default_transistor();
        // Vgs = -10V (gate well below source), Vds = -10V
        let ids = m.ids(-10.0, -10.0);
        assert!(ids > 0.0,
            "Should conduct, Ids = {:.3e}", ids);
    }

    #[test]
    fn saturation_region_identified() {
        let m = make_default_transistor();
        // Vgs = -10V, Vds = -15V -> Vsg=10, Vsd=15, Vov=10-3=7, Vsd>Vov -> sat
        assert_eq!(m.region(-10.0, -15.0), MosRegion::Saturation);
    }

    #[test]
    fn triode_region_identified() {
        let m = make_default_transistor();
        // Vgs = -10V, Vds = -1V -> Vsg=10, Vsd=1, Vov=7, Vsd<Vov -> triode
        assert_eq!(m.region(-10.0, -1.0), MosRegion::Triode);
    }

    #[test]
    fn ids_increases_with_gate_overdrive() {
        let m = make_default_transistor();
        let ids_low = m.ids(-5.0, -10.0);  // Vsg = 5, Vov = 2
        let ids_high = m.ids(-10.0, -10.0); // Vsg = 10, Vov = 7
        assert!(ids_high > ids_low,
            "More overdrive should give more current: {:.3e} vs {:.3e}",
            ids_low, ids_high);
    }

    #[test]
    fn ids_square_law_in_saturation() {
        // In saturation: Ids ~ (Vsg - Vth)^2
        // If we double overdrive, current should quadruple (approx)
        let m = PmosLevel1::with_params(1e-5, 3.0, 0.0, 1e-14, 10e-6, 10e-6);
        let ids_vov2 = m.ids_pmos(5.0, 15.0, 0.0);  // Vov = 2
        let ids_vov4 = m.ids_pmos(7.0, 15.0, 0.0);  // Vov = 4
        let ratio = ids_vov4 / ids_vov2;
        // (4/2)^2 = 4
        assert!((ratio - 4.0).abs() < 0.1,
            "Square-law ratio = {:.2}, expected 4.0", ratio);
    }

    #[test]
    fn gm_negligible_in_deep_cutoff() {
        let m = make_default_transistor();
        // With subthreshold model, gm is not exactly zero but negligibly small
        // At Vgs=0, Vsg=0, device is deeply below threshold (Vth~3V)
        let gm = m.gm(0.0, -15.0);
        assert!(gm.abs() < 1e-12,
            "gm should be negligible in deep cutoff, got {:.3e}", gm);
    }

    #[test]
    fn gm_positive_in_saturation() {
        let m = make_default_transistor();
        let gm = m.gm(-10.0, -15.0);
        assert!(gm > 0.0, "gm should be positive, got {:.3e}", gm);
    }

    #[test]
    fn gds_gives_finite_output_resistance_in_saturation() {
        let m = make_default_transistor();
        let gds = m.gds(-10.0, -15.0);
        // With lambda > 0, gds should be small but positive
        assert!(gds > 0.0, "gds should be positive (finite Rds), got {:.3e}", gds);
        // Output resistance ro = 1/gds should be large
        let ro = 1.0 / gds;
        assert!(ro > 1e3, "ro = {:.0} ohm, should be large", ro);
    }

    #[test]
    fn gds_zero_when_lambda_zero() {
        let m = PmosLevel1::with_params(1e-5, 3.0, 0.0, 1e-14, 10e-6, 10e-6);
        let gds = m.gds_pmos(10.0, 15.0); // saturation, Vsb=0
        assert!((gds - 0.0).abs() < 1e-15,
            "gds should be zero without CLM");
    }

    #[test]
    fn wider_transistor_more_current() {
        let p = ProcessParams::default();
        let narrow = PmosLevel1::new(&p, 10e-6, 10e-6);
        let wide = PmosLevel1::new(&p, 20e-6, 10e-6);

        let ids_narrow = narrow.ids(-10.0, -15.0);
        let ids_wide = wide.ids(-10.0, -15.0);

        let ratio = ids_wide / ids_narrow;
        assert!((ratio - 2.0).abs() < 0.1,
            "2x wider should give 2x current, ratio = {:.2}", ratio);
    }

    #[test]
    fn orient_voltages_selects_higher_as_source() {
        // Source at 0V, drain at -15V
        let (vsg, vsd) = PmosLevel1::orient_voltages(-10.0, 0.0, -15.0);
        assert!((vsg - 10.0).abs() < 1e-10); // Vs(0) - Vg(-10) = 10
        assert!((vsd - 15.0).abs() < 1e-10); // Vs(0) - Vd(-15) = 15

        // Swapped terminals: should auto-correct
        let (vsg2, vsd2) = PmosLevel1::orient_voltages(-10.0, -15.0, 0.0);
        assert!((vsg2 - 10.0).abs() < 1e-10);
        assert!((vsd2 - 15.0).abs() < 1e-10);
    }

    #[test]
    fn ids_absolute_matches_oriented() {
        let m = make_default_transistor();
        // Gate at -10V, Source at 0V, Drain at -15V
        let ids_abs = m.ids_absolute(-10.0, 0.0, -15.0);
        let (vsg, vsd) = PmosLevel1::orient_voltages(-10.0, 0.0, -15.0);
        let ids_oriented = m.ids_pmos(vsg, vsd, 0.0);
        assert!((ids_abs - ids_oriented).abs() < 1e-15);
    }

    #[test]
    fn cgate_positive() {
        let m = make_default_transistor();
        assert!(m.cgate() > 0.0);
    }

    // -- Body effect tests (Alpha-1) --

    #[test]
    fn vth_increases_with_source_bulk_bias() {
        let m = make_default_transistor();
        let vth_0 = m.vth_effective(0.0, 0.0);
        let vth_5 = m.vth_effective(5.0, 0.0);
        let vth_10 = m.vth_effective(10.0, 0.0);

        // Body effect: |Vth| increases with |Vsb|
        assert!(vth_5 > vth_0,
            "Vth should increase with Vsb: Vth(0)={:.3}, Vth(5)={:.3}", vth_0, vth_5);
        assert!(vth_10 > vth_5,
            "Vth should increase monotonically: Vth(5)={:.3}, Vth(10)={:.3}", vth_5, vth_10);

        // Magnitude: delta_vth ~ gamma * (sqrt(phi_b + Vsb) - sqrt(phi_b))
        // For gamma~0.37, phi_b~0.54, Vsb=10: delta ~ 0.37*(sqrt(10.54)-sqrt(0.54)) ~ 0.93V
        let delta = vth_10 - vth_0;
        assert!(delta > 0.3 && delta < 2.0,
            "Body effect delta = {:.3}V for Vsb=10V", delta);
    }

    #[test]
    fn vth_zero_body_effect_when_gamma_zero() {
        let m = PmosLevel1::with_params(1e-5, 3.0, 0.02, 1e-14, 10e-6, 10e-6);
        // gamma=0 by default in with_params
        let vth_0 = m.vth_effective(0.0, 0.0);
        let vth_10 = m.vth_effective(10.0, 0.0);
        assert!((vth_0 - vth_10).abs() < 1e-15,
            "No body effect when gamma=0");
    }

    #[test]
    fn body_effect_reduces_current() {
        let m = make_default_transistor();
        // Same gate overdrive, but non-zero Vsb increases Vth -> less current
        let ids_0 = m.ids_pmos(10.0, 10.0, 0.0);
        let ids_5 = m.ids_pmos(10.0, 10.0, 5.0);
        assert!(ids_0 > ids_5,
            "Body effect should reduce Ids: {:.3e} > {:.3e}", ids_0, ids_5);
    }

    // -- Subthreshold tests (Alpha-1) --

    #[test]
    fn subthreshold_current_small_but_nonzero() {
        let m = make_default_transistor();
        // Vsg slightly below threshold: Vsg = 2.5, Vth ~ 3.0
        let ids = m.ids_pmos(2.5, 10.0, 0.0);
        assert!(ids > 0.0, "Subthreshold current should be > 0");
        // But much smaller than above-threshold
        let ids_above = m.ids_pmos(5.0, 10.0, 0.0);
        assert!(ids < ids_above * 0.01,
            "Subthreshold current ({:.3e}) should be << above-threshold ({:.3e})",
            ids, ids_above);
    }

    #[test]
    fn subthreshold_exponential_slope() {
        let m = make_default_transistor();
        // Check that current changes by ~10x per 60-80mV (at room temp)
        // The subthreshold swing S = n * Vt * ln(10) ~ 1.3 * 26mV * 2.3 ~ 78mV/decade
        let vsg1 = 2.0;
        let vsg2 = 2.0 + 0.080; // 80mV above
        let ids1 = m.ids_pmos(vsg1, 10.0, 0.0);
        let ids2 = m.ids_pmos(vsg2, 10.0, 0.0);

        if ids1 > 1e-30 {
            let ratio = ids2 / ids1;
            // Should be roughly one decade per subthreshold swing
            assert!(ratio > 3.0 && ratio < 100.0,
                "Subthreshold slope ratio = {:.1} for 80mV step", ratio);
        }
    }

    #[test]
    fn subthreshold_zero_at_zero_vsd() {
        let m = make_default_transistor();
        // At Vsd=0, drain factor = 1 - exp(0) = 0, so no current
        let ids = m.ids_pmos(2.0, 0.0, 0.0);
        assert!(ids.abs() < 1e-20,
            "Subthreshold current should be zero at Vsd=0, got {:.3e}", ids);
    }

    #[test]
    fn subthreshold_continuity_at_threshold() {
        // Current should be roughly continuous at the Vsg = Vth boundary
        let m = make_default_transistor();
        let vth = m.vth_effective(0.0, 0.0);
        let eps = 0.01; // 10mV below and above threshold

        let ids_below = m.ids_pmos(vth - eps, 10.0, 0.0);
        let ids_above = m.ids_pmos(vth + eps, 10.0, 0.0);

        // Should be within ~10x of each other at 10mV from threshold
        if ids_below > 1e-30 {
            let ratio = ids_above / ids_below;
            assert!(ratio > 0.1 && ratio < 100.0,
                "Current should be roughly continuous at Vth: below={:.3e}, above={:.3e}, ratio={:.1}",
                ids_below, ids_above, ratio);
        }
    }

    // -- DIBL test (Alpha-1, structural) --

    #[test]
    fn dibl_reduces_threshold_with_vds() {
        let m = make_default_transistor();
        let vth_low_vsd = m.vth_effective(0.0, 1.0);
        let vth_high_vsd = m.vth_effective(0.0, 15.0);

        // DIBL reduces threshold with increasing |Vds|
        assert!(vth_high_vsd < vth_low_vsd,
            "DIBL should reduce Vth: Vth(Vsd=1)={:.4}, Vth(Vsd=15)={:.4}",
            vth_low_vsd, vth_high_vsd);

        // Small effect for 10um process (eta ~ 0.005)
        let delta = vth_low_vsd - vth_high_vsd;
        assert!(delta < 0.5, "DIBL effect should be small for 10um: delta={:.3}V", delta);
    }

    // -- Backward compatibility tests --

    #[test]
    fn with_params_matches_original_behavior() {
        // with_params creates zero-gamma, zero-eta, zero-vsat model (original Level 1)
        let m = PmosLevel1::with_params(1e-5, 3.0, 0.02, 1e-14, 10e-6, 10e-6);

        // At Vsg=0 (deep cutoff), subthreshold should return 0 since
        // the exponential is clamped so deep it's essentially zero
        let ids_cutoff = m.ids_pmos(0.0, 15.0, 0.0);
        assert!(ids_cutoff.abs() < 1e-12);

        // Above threshold should match square law exactly (no vsat)
        let ids = m.ids_pmos(5.0, 15.0, 0.0); // Vov=2, saturation
        let expected = 0.5 * 1e-5 * 4.0 * (1.0 + 0.02 * 15.0);
        assert!((ids - expected).abs() / expected < 1e-6,
            "ids={:.6e}, expected={:.6e}", ids, expected);
    }

    // -- Velocity saturation tests (Alpha-2) --

    #[test]
    fn vdsat_equals_vov_when_vsat_disabled() {
        let m = PmosLevel1::with_params(1e-5, 3.0, 0.02, 1e-14, 10e-6, 10e-6);
        // v_sat = 0 in with_params, so vdsat should equal vov
        let vdsat = m.vdsat_effective(5.0);
        assert!((vdsat - 5.0).abs() < 1e-10,
            "Vdsat should equal Vov when v_sat=0, got {:.3}", vdsat);
    }

    #[test]
    fn vdsat_limited_by_velocity_saturation() {
        let m = make_default_transistor(); // has v_sat = 8e6 cm/s
        // For 10um channel, E_sat = v_sat/mu = 8e6/175 ~ 45714 V/cm
        // Vdsat_vsat = E_sat * L = 45714 * 10e-4 = 45.7V
        // With Vov=7V: Vdsat_eff = 7 * 45.7 / (7 + 45.7) ~ 6.07V
        let vdsat = m.vdsat_effective(7.0);
        // Should be slightly less than Vov=7 due to velocity saturation
        assert!(vdsat < 7.0,
            "Vdsat ({:.3}) should be less than Vov (7.0) with v_sat enabled", vdsat);
        // But not drastically different for 10um channel (long channel)
        assert!(vdsat > 5.0,
            "Vdsat ({:.3}) should still be close to Vov for 10um", vdsat);
    }

    #[test]
    fn velocity_saturation_reduces_saturation_current() {
        let p = ProcessParams::default();
        let m_with_vsat = PmosLevel1::new(&p, 10e-6, 10e-6);

        // Create model without velocity saturation
        let mut m_no_vsat = m_with_vsat.clone();
        m_no_vsat.v_sat = 0.0;

        let ids_with = m_with_vsat.ids_pmos(10.0, 15.0, 0.0);
        let ids_without = m_no_vsat.ids_pmos(10.0, 15.0, 0.0);

        // With velocity saturation, current should be slightly less
        assert!(ids_with <= ids_without,
            "Vsat should reduce Ids: with={:.3e}, without={:.3e}",
            ids_with, ids_without);
    }

    #[test]
    fn velocity_saturation_larger_effect_short_channel() {
        let p = ProcessParams::default();

        // Long channel (10um) - small effect
        let m_long = PmosLevel1::new(&p, 10e-6, 10e-6);
        let vdsat_long = m_long.vdsat_effective(7.0);
        let reduction_long = 1.0 - vdsat_long / 7.0;

        // Short channel (1um) - larger effect
        let m_short = PmosLevel1::new(&p, 10e-6, 1e-6);
        let vdsat_short = m_short.vdsat_effective(7.0);
        let reduction_short = 1.0 - vdsat_short / 7.0;

        assert!(reduction_short > reduction_long,
            "Shorter channel should show more v_sat reduction: short={:.3}, long={:.3}",
            reduction_short, reduction_long);
    }

    // -- DIBL additional tests (Alpha-2) --

    #[test]
    fn dibl_increases_current_at_high_vds() {
        let m = make_default_transistor();
        // Near threshold: Vsg=3.5 (barely on)
        let ids_low_vds = m.ids_pmos(3.5, 1.0, 0.0);
        let ids_high_vds = m.ids_pmos(3.5, 15.0, 0.0);

        // DIBL lowers Vth at high Vds, so more current
        assert!(ids_high_vds > ids_low_vds,
            "DIBL should increase Ids at high Vds: high={:.3e}, low={:.3e}",
            ids_high_vds, ids_low_vds);
    }
}
