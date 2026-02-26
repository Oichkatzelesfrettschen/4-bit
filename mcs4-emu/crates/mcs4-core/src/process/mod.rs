//! Semiconductor process parameters for Intel's 10um pMOS silicon-gate technology.
//!
//! This module encapsulates the material properties and process parameters
//! of the fabrication process used for the MCS-4 chip family (4001, 4002,
//! 4003, 4004). All parameters are drawn from published literature and
//! datasheets; see `docs/evidence/process_parameters_v0.md` for sources.
//!
//! # Process Overview
//!
//! Intel's first silicon-gate MOS technology (1971):
//! - p-channel enhancement-mode transistors with depletion-mode loads
//! - Self-aligned polysilicon gates
//! - 10um minimum feature size
//! - ~80nm (800A) thermally grown SiO2 gate oxide
//! - Single-layer aluminum metallization
//! - N-type silicon substrate (~10 ohm-cm)
//! - Supply: VDD = -15V, VSS = 0V (later -10V variants)

pub mod esd;
pub mod interconnect;
pub mod io_driver;
pub mod junction;
pub mod mobility;
pub mod oxide;
pub mod power;
pub mod rom_cell;
pub mod silicon;
pub mod sram_cell;
pub mod thermal;

use oxide::cox;
use silicon::{EPSILON_OX, EPSILON_SI};

/// Complete process parameter set for Intel's 10um pMOS SGT process.
///
/// Stores all parameters needed to instantiate transistor models and
/// compute interconnect parasitics. The `Default` implementation
/// provides best-estimate values for the original 4004 process.
#[derive(Clone, Debug)]
pub struct ProcessParams {
    // -- Supply --
    /// Positive supply rail (V). Typically 0V for pMOS.
    pub vss: f64,
    /// Negative supply rail (V). Typically -15V for MCS-4.
    pub vdd: f64,

    // -- Gate oxide --
    /// Gate oxide thickness (m).
    pub t_ox: f64,

    // -- Threshold voltages --
    /// Enhancement-mode pMOS threshold voltage at 300K (V, negative).
    pub vth_enhancement: f64,
    /// Depletion-mode pMOS threshold voltage at 300K (V, positive).
    pub vth_depletion: f64,

    // -- Mobility --
    /// Low-field hole mobility at 300K (cm^2/V*s).
    pub mu_0: f64,
    /// Mobility degradation coefficient (theta) for vertical field.
    pub theta: f64,

    // -- Substrate --
    /// Substrate doping concentration (cm^-3), n-type for pMOS.
    pub n_sub: f64,
    /// Source/drain doping concentration (cm^-3), p+ for pMOS.
    pub n_sd: f64,

    // -- Geometry --
    /// Minimum feature size / gate length (m).
    pub l_min: f64,
    /// Junction depth (m).
    pub x_j: f64,

    // -- Channel-length modulation --
    /// Lambda parameter (1/V) for Shichman-Hodges model.
    pub lambda: f64,

    // -- Body effect --
    /// DIBL coefficient (V/V). Threshold voltage reduction per volt of Vds.
    /// Small for 10um but structurally correct for shorter-channel models.
    pub eta_dibl: f64,

    // -- Temperature --
    /// Nominal operating temperature (K).
    pub temp: f64,
}

impl Default for ProcessParams {
    /// Best-estimate parameters for the Intel 4004 process (1971).
    fn default() -> Self {
        Self {
            vss: 0.0,
            vdd: -15.0,
            t_ox: 80e-9,           // 80nm = 800 angstrom
            vth_enhancement: -3.0, // -2V to -4V range, center estimate
            vth_depletion: 1.0,    // positive Vth for depletion mode
            mu_0: 175.0,           // cm^2/V*s (surface holes)
            theta: 0.03,           // mobility degradation
            n_sub: 4.5e14,         // ~10 ohm-cm n-type
            n_sd: 1e19,            // p+ source/drain
            l_min: 10e-6,          // 10um
            x_j: 2e-6,             // 2um junction depth (estimated)
            lambda: 0.02,          // 1/V, moderate CLM for 10um
            eta_dibl: 0.005,       // V/V, negligible at 10um
            temp: 300.0,
        }
    }
}

impl ProcessParams {
    /// Gate oxide capacitance per unit area (F/m^2).
    pub fn cox(&self) -> f64 {
        cox(self.t_ox)
    }

    /// Gate capacitance for given transistor dimensions (F).
    pub fn cgate(&self, w: f64, l: f64) -> f64 {
        oxide::cgate(self.cox(), w, l)
    }

    /// Transconductance parameter beta = mu * Cox * W/L (A/V^2).
    pub fn beta(&self, w: f64, l: f64) -> f64 {
        mobility::beta(self.mu_0, self.cox(), w, l)
    }

    /// Effective mobility at current operating conditions (cm^2/V*s).
    pub fn mu_eff(&self, vgs: f64) -> f64 {
        thermal::mu_eff_at_temp(self.mu_0, vgs, self.vth_enhancement, self.t_ox, self.theta, self.temp)
    }

    /// Zero-bias junction capacitance per unit area (F/m^2).
    pub fn cj0(&self) -> f64 {
        junction::cj0(self.n_sd, self.n_sub, self.temp)
    }

    /// Thermal voltage at operating temperature (V).
    pub fn vt(&self) -> f64 {
        silicon::vt(self.temp)
    }

    /// Fermi potential of substrate (V).
    pub fn phi_f(&self) -> f64 {
        silicon::phi_f(self.n_sub, self.temp)
    }

    /// Silicon permittivity (F/m). Convenience accessor.
    pub fn epsilon_si(&self) -> f64 {
        EPSILON_SI
    }

    /// Oxide permittivity (F/m). Convenience accessor.
    pub fn epsilon_ox(&self) -> f64 {
        EPSILON_OX
    }

    /// Body effect coefficient gamma (V^0.5).
    ///
    /// gamma = sqrt(2 * q * eps_si * N_sub) / Cox
    ///
    /// For the 10um process with N_sub=4.5e14 and tox=80nm,
    /// gamma is approximately 0.37 V^0.5.
    pub fn gamma(&self) -> f64 {
        let cox = self.cox();
        // n_sub in cm^-3 -> convert to m^-3 for SI
        let n_sub_si = self.n_sub * 1e6;
        (2.0 * silicon::Q * EPSILON_SI * n_sub_si).sqrt() / cox
    }

    /// Bulk Fermi potential phi_b = 2 * phi_f (V).
    ///
    /// Used in body effect and threshold voltage calculations.
    /// For n-type substrate, phi_f > 0, so phi_b > 0.
    pub fn phi_b(&self) -> f64 {
        2.0 * self.phi_f()
    }

    /// Subthreshold slope ideality factor n.
    ///
    /// n = 1 + gamma / (2 * sqrt(phi_b))
    ///
    /// Typically 1.2-1.5 for bulk MOSFET processes.
    pub fn subthreshold_n(&self) -> f64 {
        let phi_b = self.phi_b();
        if phi_b > 0.0 {
            1.0 + self.gamma() / (2.0 * phi_b.sqrt())
        } else {
            1.5 // fallback
        }
    }

    /// Create process params for a -10V supply variant.
    pub fn mcs4_10v() -> Self {
        Self {
            vdd: -10.0,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_params_internally_consistent() {
        let p = ProcessParams::default();

        // Supply: VDD < 0, VSS = 0
        assert!(p.vdd < 0.0);
        assert!((p.vss - 0.0).abs() < 1e-10);

        // Oxide thickness positive
        assert!(p.t_ox > 0.0);

        // Enhancement Vth negative, depletion Vth positive
        assert!(p.vth_enhancement < 0.0);
        assert!(p.vth_depletion > 0.0);

        // Mobility positive
        assert!(p.mu_0 > 0.0);

        // Doping concentrations positive and substrate < source/drain
        assert!(p.n_sub > 0.0);
        assert!(p.n_sd > p.n_sub);
    }

    #[test]
    fn cox_within_expected_range() {
        let p = ProcessParams::default();
        let c = p.cox();
        // For 80nm SiO2: ~4.3e-4 F/m^2
        assert!(c > 1e-4 && c < 1e-3, "C_ox = {:.3e} F/m^2", c);
    }

    #[test]
    fn beta_reasonable_for_10um_transistor() {
        let p = ProcessParams::default();
        let b = p.beta(10e-6, 10e-6);
        // Should be in micro-amps/V^2 range
        assert!(b > 1e-7 && b < 1e-4, "beta = {:.3e} A/V^2", b);
    }

    #[test]
    fn cj0_positive() {
        let p = ProcessParams::default();
        let c = p.cj0();
        assert!(c > 0.0 && c.is_finite());
    }

    #[test]
    fn mcs4_10v_variant() {
        let p = ProcessParams::mcs4_10v();
        assert!((p.vdd - (-10.0)).abs() < 1e-10);
        // All other params should be same as default
        let d = ProcessParams::default();
        assert!((p.t_ox - d.t_ox).abs() < 1e-15);
        assert!((p.vth_enhancement - d.vth_enhancement).abs() < 1e-15);
    }

    #[test]
    fn gamma_within_expected_range() {
        let p = ProcessParams::default();
        let g = p.gamma();
        // For N_sub=4.5e14, tox=80nm: gamma ~ 0.3-0.5 V^0.5
        assert!(g > 0.1 && g < 1.0, "gamma = {:.3} V^0.5, expected 0.1-1.0", g);
    }

    #[test]
    fn phi_b_positive_for_n_type_substrate() {
        let p = ProcessParams::default();
        let pb = p.phi_b();
        // phi_b = 2 * phi_f ~ 2 * 0.27 = 0.54 V
        assert!(pb > 0.3 && pb < 1.0, "phi_b = {:.3} V, expected 0.3-1.0", pb);
    }

    #[test]
    fn subthreshold_n_reasonable() {
        let p = ProcessParams::default();
        let n = p.subthreshold_n();
        // Typical n ~ 1.2-1.5
        assert!(n > 1.0 && n < 2.0, "subthreshold n = {:.3}, expected 1.0-2.0", n);
    }

    #[test]
    fn eta_dibl_non_negative() {
        let p = ProcessParams::default();
        assert!(p.eta_dibl >= 0.0);
        assert!(p.eta_dibl < 0.1, "eta_dibl should be small for 10um");
    }
}
