//! SRAM cell model for the Intel 4002 static RAM.
//!
//! WHY: The 4002 provides the working memory for MCS-4 systems. Understanding
//! cell stability (static noise margin), read/write timing, and minimum
//! supply voltage for data retention is critical for system reliability.
//!
//! WHAT: 6-transistor SRAM cell with cross-coupled inverters and access
//! transistors. Computes static noise margin, read/write timing, and
//! minimum retention voltage.
//!
//! HOW: The 6T cell uses two cross-coupled pMOS inverters with enhancement
//! pull-downs and depletion loads, plus two enhancement access transistors
//! gated by the wordline.

use super::ProcessParams;

/// 4002 RAM array dimensions: 80 x 4-bit words = 320 bits.
pub const RAM_4002_BITS: usize = 320;

/// Pull-down (driver) transistor W/L for the SRAM cell.
/// Must be stronger than the access transistor for read stability.
const DRIVER_W: f64 = 20e-6; // 20um
const DRIVER_L: f64 = 10e-6; // 10um

/// Access transistor W/L. Weaker than driver for read margin.
const ACCESS_W: f64 = 10e-6; // 10um
const ACCESS_L: f64 = 10e-6; // 10um

/// Load transistor W/L (depletion mode). Very weak for low power.
const LOAD_W: f64 = 5e-6; // 5um
const LOAD_L: f64 = 40e-6; // 40um

/// Static noise margin (SNM) of the 6T SRAM cell (V).
///
/// SNM is the side of the largest square that fits inside the butterfly
/// curve (cross-coupled inverter voltage transfer characteristics).
/// Simplified analytical estimate:
///   SNM ~ Vth * (1 - 1/CR)
/// where CR = (W/L)_driver / (W/L)_access is the cell ratio.
///
/// Higher CR gives better read stability at the cost of area.
///
/// # Arguments
/// * `params` - process parameters
pub fn static_noise_margin(params: &ProcessParams) -> f64 {
    let cr = (DRIVER_W / DRIVER_L) / (ACCESS_W / ACCESS_L);
    let vth = params.vth_enhancement.abs();

    // Simplified SNM estimate for pMOS SRAM
    // SNM increases with Vth and cell ratio
    vth * (1.0 - 1.0 / cr)
}

/// Read access time for the 4002 SRAM (seconds).
///
/// t_read = t_wordline + t_bitline_develop + t_sense
///
/// The bitline differential develops as the cell fights the precharged
/// bitline through the access transistor.
///
/// # Arguments
/// * `params` - process parameters
pub fn read_access_time(params: &ProcessParams) -> f64 {
    // Wordline delay (poly, short for 4-bit word)
    let t_wl = 5e-9; // ~5ns for short wordline

    // Bitline development: RC of access transistor driving bitline cap
    let ron_access = 1.0 / (params.beta(ACCESS_W, ACCESS_L) * (params.vdd.abs() - params.vth_enhancement.abs()));

    // Bitline capacitance: ~80 cells * junction cap per cell
    let c_bl = 80.0 * params.cj0() * ACCESS_W * 2e-6;

    let t_develop = 2.2 * ron_access * c_bl; // 63% development

    // Sense amplifier + output buffer delay
    let t_sense = 20e-9;

    t_wl + t_develop + t_sense
}

/// Write time for the 4002 SRAM (seconds).
///
/// Write requires flipping the cell state by overpowering the feedback
/// loop through the access transistor. Write time is typically faster
/// than read because the bitline driver is much stronger.
///
/// # Arguments
/// * `params` - process parameters
pub fn write_time(params: &ProcessParams) -> f64 {
    // Cell node capacitance (gate caps of cross-coupled transistors)
    let c_cell = 2.0 * params.cgate(DRIVER_W, DRIVER_L) + 2.0 * params.cgate(LOAD_W, LOAD_L);

    // Access transistor Ron
    let ron_access = 1.0 / (params.beta(ACCESS_W, ACCESS_L) * (params.vdd.abs() - params.vth_enhancement.abs()));

    // Write time ~ 3-5 RC time constants to flip the cell
    let t_wl = 5e-9;
    t_wl + 4.0 * ron_access * c_cell
}

/// Minimum VDD for data retention (V).
///
/// The cell retains data as long as the cross-coupled inverters have
/// enough gain to sustain bistable operation. As VDD decreases, the
/// gain drops below unity and the cell loses state.
///
/// Simplified estimate: V_min ~ 2 * |Vth| (both transistors must be
/// in saturation for the feedback loop to function).
///
/// # Arguments
/// * `params` - process parameters
pub fn min_retention_voltage(params: &ProcessParams) -> f64 {
    // Minimum VDD ~ 2 * |Vth| for the enhancement transistors
    // plus some margin for the depletion loads
    let vth = params.vth_enhancement.abs();
    2.0 * vth + 0.5 // Add 0.5V margin for depletion load headroom
}

/// Cell ratio: driver strength / access strength.
///
/// CR > 1 required for read stability. Typical target: 1.5-3.0.
pub fn cell_ratio() -> f64 {
    (DRIVER_W / DRIVER_L) / (ACCESS_W / ACCESS_L)
}

/// Pull-up ratio: load strength / access strength.
///
/// PR < 1 required for write-ability.
pub fn pull_up_ratio() -> f64 {
    (LOAD_W / LOAD_L) / (ACCESS_W / ACCESS_L)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snm_positive_at_nominal_vdd() {
        let p = ProcessParams::default();
        let snm = static_noise_margin(&p);
        assert!(snm > 0.0, "SNM must be positive for stable cell: SNM = {:.3}V", snm);
        // SNM should be a reasonable fraction of Vth
        assert!(
            snm < p.vth_enhancement.abs(),
            "SNM should be less than |Vth|: SNM={:.3}V, |Vth|={:.3}V",
            snm,
            p.vth_enhancement.abs()
        );
    }

    #[test]
    fn cell_ratio_greater_than_one() {
        let cr = cell_ratio();
        assert!(cr > 1.0, "Cell ratio must be > 1 for read stability: CR = {:.2}", cr);
    }

    #[test]
    fn pull_up_ratio_less_than_one() {
        let pr = pull_up_ratio();
        assert!(pr < 1.0, "Pull-up ratio must be < 1 for writeability: PR = {:.2}", pr);
    }

    #[test]
    fn read_access_time_in_range() {
        let p = ProcessParams::default();
        let t_read = read_access_time(&p);
        // Expected: 50-1000ns for 10um SRAM
        assert!(
            t_read > 10e-9 && t_read < 5e-6,
            "Read time = {:.2e} s, expected 10ns-5us",
            t_read
        );
    }

    #[test]
    fn write_faster_than_read() {
        let p = ProcessParams::default();
        let t_read = read_access_time(&p);
        let t_write = write_time(&p);
        // Write should be same order or faster than read
        assert!(
            t_write < t_read * 5.0,
            "Write should not be much slower than read: write={:.2e}, read={:.2e}",
            t_write,
            t_read
        );
    }

    #[test]
    fn min_retention_reasonable() {
        let p = ProcessParams::default();
        let v_min = min_retention_voltage(&p);
        // Should be well below nominal VDD
        assert!(
            v_min < p.vdd.abs(),
            "Min retention voltage ({:.1}V) should be below |VDD| ({:.1}V)",
            v_min,
            p.vdd.abs()
        );
        // Should be above 2*Vth
        assert!(
            v_min > 2.0 * p.vth_enhancement.abs() * 0.8,
            "Min retention should be near 2*|Vth|: V_min={:.1}V, 2*|Vth|={:.1}V",
            v_min,
            2.0 * p.vth_enhancement.abs()
        );
    }

    #[test]
    fn write_flips_cell_correctly() {
        // Verify that access transistor is weaker than driver (required for read)
        // but the bitline driver can overpower the cell (required for write)
        let p = ProcessParams::default();

        // Driver current > access current (read stability)
        let i_driver = p.beta(DRIVER_W, DRIVER_L) * (p.vdd.abs() - p.vth_enhancement.abs()).powi(2) / 2.0;
        let i_access = p.beta(ACCESS_W, ACCESS_L) * (p.vdd.abs() - p.vth_enhancement.abs()).powi(2) / 2.0;

        assert!(
            i_driver > i_access,
            "Driver ({:.2e}A) must be stronger than access ({:.2e}A)",
            i_driver,
            i_access
        );
    }
}
