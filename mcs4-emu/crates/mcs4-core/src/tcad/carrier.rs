//! Carrier statistics for semiconductor simulation.
//!
//! Provides electron and hole concentration calculations using
//! Boltzmann statistics, suitable for the moderate doping levels
//! of the 1970s Intel pMOS process.

use crate::process::silicon::{self, Q};

/// Carrier statistics calculator for a given doping profile.
#[derive(Clone, Debug)]
pub struct CarrierStats {
    /// Substrate doping concentration (cm^-3). N-type (donors) for pMOS.
    pub n_doping: f64,
    /// Temperature (K).
    pub temp: f64,
    /// Thermal voltage (V).
    vt: f64,
    /// Intrinsic concentration (cm^-3).
    ni: f64,
    /// Fermi potential (V).
    phi_f: f64,
}

impl CarrierStats {
    /// Create carrier statistics for given doping and temperature.
    pub fn new(n_doping: f64, temp: f64) -> Self {
        let vt = silicon::vt(temp);
        let ni = silicon::ni(temp);
        let phi_f = silicon::phi_f(n_doping, temp);
        Self {
            n_doping,
            temp,
            vt,
            ni,
            phi_f,
        }
    }

    /// Thermal voltage (V).
    pub fn vt(&self) -> f64 {
        self.vt
    }

    /// Intrinsic concentration (cm^-3).
    pub fn ni(&self) -> f64 {
        self.ni
    }

    /// Fermi potential (V).
    pub fn phi_f(&self) -> f64 {
        self.phi_f
    }

    /// Electron concentration at electrostatic potential psi (cm^-3).
    ///
    /// For n-type substrate (pMOS):
    ///   n(psi) = n_doping * exp(psi / V_T)
    ///
    /// psi is measured relative to the neutral bulk (psi_bulk = 0).
    /// Positive psi = accumulation (more electrons at surface).
    /// Negative psi = depletion/inversion (fewer electrons, more holes).
    pub fn electron_concentration(&self, psi: f64) -> f64 {
        // Clamp exponent to avoid overflow
        let arg = psi / self.vt;
        let arg_clamped = arg.clamp(-80.0, 80.0);
        self.n_doping * arg_clamped.exp()
    }

    /// Hole concentration at electrostatic potential psi (cm^-3).
    ///
    /// For n-type substrate:
    ///   p(psi) = (ni^2 / n_doping) * exp(-psi / V_T)
    pub fn hole_concentration(&self, psi: f64) -> f64 {
        let p0 = self.ni * self.ni / self.n_doping;
        let arg = -psi / self.vt;
        let arg_clamped = arg.clamp(-80.0, 80.0);
        p0 * arg_clamped.exp()
    }

    /// Net space charge density at potential psi (C/cm^3).
    ///
    /// rho = q * (N_d + p(psi) - n(psi))
    ///
    /// For n-type substrate: N_d = n_doping (ionized donors).
    /// In neutral bulk (psi=0): n ~ N_d, p ~ ni^2/N_d, rho ~ 0.
    /// In depletion (psi < 0): n drops exponentially, rho ~ q*N_d (positive).
    /// In strong inversion (psi << -2*phi_f): p >> N_d, rho goes negative.
    pub fn charge_density(&self, psi: f64) -> f64 {
        let n = self.electron_concentration(psi);
        let p = self.hole_concentration(psi);
        Q * (self.n_doping + p - n)
    }

    /// Charge density in SI units (C/m^3).
    pub fn charge_density_si(&self, psi: f64) -> f64 {
        // Convert from C/cm^3 to C/m^3 (multiply by 1e6)
        self.charge_density(psi) * 1e6
    }

    /// Derivative of charge density with respect to psi (C/cm^3/V).
    ///
    /// d(rho)/d(psi) = q * (dp/dpsi - dn/dpsi)
    ///               = q * (-p/V_T - n/V_T) = -q*(n+p)/V_T
    ///
    /// Always negative (charge becomes more negative as psi increases).
    pub fn dcharge_dpsi(&self, psi: f64) -> f64 {
        let n = self.electron_concentration(psi);
        let p = self.hole_concentration(psi);
        -Q * (n + p) / self.vt
    }

    /// Derivative in SI units (C/m^3/V).
    pub fn dcharge_dpsi_si(&self, psi: f64) -> f64 {
        self.dcharge_dpsi(psi) * 1e6
    }

    /// Depletion width for given surface potential (m).
    ///
    /// W_dep = sqrt(2 * eps_si * |psi_s| / (q * N_d))
    ///
    /// Valid in the depletion approximation (no inversion charge).
    pub fn depletion_width(&self, psi_surface: f64) -> f64 {
        use crate::process::silicon::EPSILON_SI;
        let n_si = self.n_doping * 1e6; // cm^-3 to m^-3
        let psi_abs = psi_surface.abs();
        if psi_abs < 1e-15 {
            return 0.0;
        }
        (2.0 * EPSILON_SI * psi_abs / (Q * n_si)).sqrt()
    }

    /// Maximum depletion width at onset of strong inversion (m).
    ///
    /// W_max = sqrt(4 * eps_si * phi_f / (q * N_d))
    pub fn max_depletion_width(&self) -> f64 {
        use crate::process::silicon::EPSILON_SI;
        let n_si = self.n_doping * 1e6;
        (4.0 * EPSILON_SI * self.phi_f.abs() / (Q * n_si)).sqrt()
    }
}

/// Charge profile along a 1D mesh.
#[derive(Clone, Debug)]
pub struct ChargeProfile {
    /// Electrostatic potential at each mesh point (V).
    pub psi: Vec<f64>,
    /// Electron concentration at each point (cm^-3).
    pub n: Vec<f64>,
    /// Hole concentration at each point (cm^-3).
    pub p: Vec<f64>,
    /// Net charge density at each point (C/cm^3).
    pub rho: Vec<f64>,
}

impl ChargeProfile {
    /// Compute charge profile from potential distribution.
    pub fn from_potential(psi: &[f64], stats: &CarrierStats) -> Self {
        let n: Vec<f64> = psi.iter().map(|&v| stats.electron_concentration(v)).collect();
        let p: Vec<f64> = psi.iter().map(|&v| stats.hole_concentration(v)).collect();
        let rho: Vec<f64> = psi.iter().map(|&v| stats.charge_density(v)).collect();
        Self {
            psi: psi.to_vec(),
            n,
            p,
            rho,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_stats() -> CarrierStats {
        // N-type substrate, 4.5e14 cm^-3
        CarrierStats::new(4.5e14, 300.0)
    }

    #[test]
    fn neutral_bulk_charge_near_zero() {
        let stats = default_stats();
        let rho = stats.charge_density(0.0);
        // At psi=0 (neutral bulk): n = N_d, p = ni^2/N_d
        // rho = q * (N_d - N_d + ni^2/N_d) ~ q * ni^2/N_d
        // This is very small compared to q*N_d
        let rho_max = Q * stats.n_doping;
        assert!(
            rho.abs() / rho_max < 1e-4,
            "rho at psi=0: {:.3e}, max {:.3e}",
            rho,
            rho_max
        );
    }

    #[test]
    fn depletion_charge_positive() {
        let stats = default_stats();
        // Negative psi = depletion in n-type substrate (electrons repelled)
        let rho = stats.charge_density(-0.5);
        // Positive charge (ionized donors exposed)
        assert!(rho > 0.0, "depletion charge should be positive, got {:.3e}", rho);
    }

    #[test]
    fn inversion_has_large_hole_concentration() {
        let stats = default_stats();
        // Strong inversion: psi_s ~ -2*phi_f ~ -0.55V
        // Holes accumulate at the surface in n-type substrate
        let phi_f = stats.phi_f();
        let psi_inv = -2.5 * phi_f; // well into inversion
        let p = stats.hole_concentration(psi_inv);
        // In strong inversion, p >> n_doping at the surface
        assert!(
            p > stats.n_doping,
            "hole concentration at psi={:.3}V should exceed N_d: p={:.3e}, N_d={:.3e}",
            psi_inv,
            p,
            stats.n_doping
        );
    }

    #[test]
    fn electron_concentration_at_equilibrium() {
        let stats = default_stats();
        let n0 = stats.electron_concentration(0.0);
        // At equilibrium, n = N_d
        assert!(
            (n0 - stats.n_doping).abs() / stats.n_doping < 1e-6,
            "n(0) = {:.3e}, expected {:.3e}",
            n0,
            stats.n_doping
        );
    }

    #[test]
    fn hole_concentration_at_equilibrium() {
        let stats = default_stats();
        let p0 = stats.hole_concentration(0.0);
        let expected = stats.ni * stats.ni / stats.n_doping;
        assert!(
            (p0 - expected).abs() / expected < 1e-6,
            "p(0) = {:.3e}, expected {:.3e}",
            p0,
            expected
        );
    }

    #[test]
    fn np_product_equals_ni_squared() {
        let stats = default_stats();
        // At any potential, n*p should approximately equal ni^2
        // (exact in Boltzmann statistics at thermal equilibrium)
        for psi in [-0.3, 0.0, 0.2] {
            let n = stats.electron_concentration(psi);
            let p = stats.hole_concentration(psi);
            let product = n * p;
            let ni_sq = stats.ni * stats.ni;
            assert!(
                (product - ni_sq).abs() / ni_sq < 1e-4,
                "n*p = {:.3e}, ni^2 = {:.3e} at psi={:.3}V",
                product,
                ni_sq,
                psi
            );
        }
    }

    #[test]
    fn dcharge_dpsi_always_negative() {
        let stats = default_stats();
        for psi in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            let drho = stats.dcharge_dpsi(psi);
            assert!(
                drho < 0.0,
                "drho/dpsi should be < 0 at psi={:.1}V, got {:.3e}",
                psi,
                drho
            );
        }
    }

    #[test]
    fn depletion_width_increases_with_bias() {
        let stats = default_stats();
        let w1 = stats.depletion_width(-0.5);
        let w2 = stats.depletion_width(-1.0);
        let w3 = stats.depletion_width(-2.0);
        assert!(w2 > w1, "depletion width should grow with |psi|");
        assert!(w3 > w2);
    }

    #[test]
    fn max_depletion_width_order_of_magnitude() {
        let stats = default_stats();
        let w_max = stats.max_depletion_width();
        // For N_d=4.5e14, W_max ~ 1-3 um
        let w_um = w_max * 1e6;
        assert!(w_um > 0.5 && w_um < 10.0, "W_max = {:.2} um, expected 0.5-10 um", w_um);
    }

    #[test]
    fn charge_profile_consistent() {
        let stats = default_stats();
        let psi_vals = vec![0.0, -0.2, -0.5, -0.8];
        let profile = ChargeProfile::from_potential(&psi_vals, &stats);
        assert_eq!(profile.psi.len(), 4);
        assert_eq!(profile.n.len(), 4);
        assert_eq!(profile.p.len(), 4);
        assert_eq!(profile.rho.len(), 4);
    }

    #[test]
    fn charge_density_si_conversion() {
        let stats = default_stats();
        let rho_cgs = stats.charge_density(-0.5);
        let rho_si = stats.charge_density_si(-0.5);
        assert!(
            (rho_si - rho_cgs * 1e6).abs() / rho_si.abs() < 1e-10,
            "SI conversion: {:.3e} vs {:.3e} * 1e6",
            rho_si,
            rho_cgs
        );
    }
}
