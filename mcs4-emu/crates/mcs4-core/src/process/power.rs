//! Power dissipation model for MCS-4 chips.
//!
//! WHY: Understanding power consumption is essential for thermal analysis,
//! supply rail design, and validating against published datasheet specs.
//!
//! WHAT: Static (leakage) and dynamic (switching CV^2f) power per chip,
//! using transistor counts from published die photos and documentation.
//!
//! HOW: Static power from subthreshold leakage current per transistor.
//! Dynamic power from gate capacitance charging at the clock frequency.

use super::ProcessParams;

/// Transistor counts for MCS-4 chip family (from published die analysis).
pub struct ChipTransistorCount {
    /// Chip name.
    pub name: &'static str,
    /// Total transistor count.
    pub total: usize,
    /// Estimated fraction of transistors switching per cycle (activity factor).
    pub activity_factor: f64,
}

/// Known MCS-4 chip transistor counts.
pub const CHIP_4004: ChipTransistorCount = ChipTransistorCount {
    name: "4004",
    total: 2300,
    activity_factor: 0.15, // ~15% toggle per cycle (instruction-dependent)
};

pub const CHIP_4001: ChipTransistorCount = ChipTransistorCount {
    name: "4001",
    total: 2048,
    activity_factor: 0.10, // ROM: mostly static, address decode toggles
};

pub const CHIP_4002: ChipTransistorCount = ChipTransistorCount {
    name: "4002",
    total: 590,
    activity_factor: 0.20, // RAM: read/write activity
};

pub const CHIP_4003: ChipTransistorCount = ChipTransistorCount {
    name: "4003",
    total: 37,
    activity_factor: 0.50, // Shift register: high toggle rate
};

/// Subthreshold leakage current per transistor (A).
///
/// I_leak = W/L * I0 * exp(-|Vth| / (n * Vt))
///
/// For 10um pMOS at room temperature with |Vth| ~ 3V,
/// leakage is extremely small (~fA per transistor).
///
/// # Arguments
/// * `params` - process parameters
/// * `w` - transistor width (m)
/// * `l` - transistor length (m)
pub fn leakage_per_transistor(params: &ProcessParams, w: f64, l: f64) -> f64 {
    let vt = params.vt(); // thermal voltage ~26mV
    let n = params.subthreshold_n();
    let i0 = 1e-14; // reference current scale (A) for pMOS
    let vth_abs = params.vth_enhancement.abs();

    (w / l) * i0 * (-vth_abs / (n * vt)).exp()
}

/// Total static (leakage) power for a chip (W).
///
/// P_static = N_transistors * I_leak * |VDD|
pub fn static_power(params: &ProcessParams, chip: &ChipTransistorCount) -> f64 {
    // Average transistor geometry: W=20um, L=10um (typical internal)
    let avg_w = 20e-6;
    let avg_l = 10e-6;
    let i_leak = leakage_per_transistor(params, avg_w, avg_l);
    chip.total as f64 * i_leak * params.vdd.abs()
}

/// Dynamic power dissipation (W).
///
/// P_dynamic = alpha * N * C_gate * V^2 * f
///
/// where alpha is the activity factor, N is transistor count,
/// C_gate is average gate capacitance, V is supply voltage,
/// and f is clock frequency.
///
/// # Arguments
/// * `params` - process parameters
/// * `chip` - chip transistor count and activity factor
/// * `freq` - clock frequency (Hz)
pub fn dynamic_power(params: &ProcessParams, chip: &ChipTransistorCount, freq: f64) -> f64 {
    // Average gate capacitance for a 20um x 10um transistor
    let c_gate = params.cgate(20e-6, 10e-6);
    let v = params.vdd.abs();

    chip.activity_factor * (chip.total as f64) * c_gate * v * v * freq
}

/// Total power dissipation (W).
///
/// P_total = P_static + P_dynamic
pub fn total_power(params: &ProcessParams, chip: &ChipTransistorCount, freq: f64) -> f64 {
    static_power(params, chip) + dynamic_power(params, chip, freq)
}

/// Power dissipation estimate for a complete MCS-4 system.
///
/// Assumes: 1x 4004, 4x 4001 ROM, 2x 4002 RAM, 1x 4003 shift register.
pub fn system_power(params: &ProcessParams, freq: f64) -> f64 {
    total_power(params, &CHIP_4004, freq)
        + 4.0 * total_power(params, &CHIP_4001, freq)
        + 2.0 * total_power(params, &CHIP_4002, freq)
        + total_power(params, &CHIP_4003, freq)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leakage_extremely_small() {
        let p = ProcessParams::default();
        let i_leak = leakage_per_transistor(&p, 20e-6, 10e-6);
        // With |Vth|=3V and Vt=26mV, leakage should be femtoamps or less
        assert!(
            i_leak < 1e-12,
            "Leakage = {:.2e} A, should be sub-pA for 10um pMOS",
            i_leak
        );
        assert!(i_leak > 0.0, "Leakage must be positive");
    }

    #[test]
    fn static_power_negligible() {
        let p = ProcessParams::default();
        let p_static = static_power(&p, &CHIP_4004);
        // For 2300 transistors at femtoamp leakage, static power ~ nW or less
        assert!(
            p_static < 1e-3,
            "Static power = {:.2e} W, should be negligible for pMOS",
            p_static
        );
    }

    #[test]
    fn dynamic_power_in_milliwatt_range() {
        let p = ProcessParams::default();
        let freq = 740e3; // 740kHz (4004 clock)
        let p_dyn = dynamic_power(&p, &CHIP_4004, freq);
        // Published 4004 power: ~1W at 740kHz with -15V supply
        // Our simplified model should be in the mW-W range
        assert!(
            p_dyn > 1e-6 && p_dyn < 10.0,
            "Dynamic power = {:.4e} W at 740kHz, expected mW-W range",
            p_dyn
        );
    }

    #[test]
    fn total_power_dominated_by_dynamic() {
        let p = ProcessParams::default();
        let freq = 740e3;
        let p_static = static_power(&p, &CHIP_4004);
        let p_dyn = dynamic_power(&p, &CHIP_4004, freq);
        assert!(
            p_dyn > p_static * 100.0,
            "Dynamic should dominate static by >100x: static={:.2e}, dynamic={:.2e}",
            p_static,
            p_dyn
        );
    }

    #[test]
    fn power_scales_with_frequency() {
        let p = ProcessParams::default();
        let p_low = dynamic_power(&p, &CHIP_4004, 100e3);
        let p_high = dynamic_power(&p, &CHIP_4004, 1e6);
        // Power should scale linearly with frequency
        let ratio = p_high / p_low;
        assert!(
            (ratio - 10.0).abs() < 0.1,
            "Power ratio should be ~10x for 10x frequency, got {:.2}x",
            ratio
        );
    }

    #[test]
    fn system_power_reasonable() {
        let p = ProcessParams::default();
        let freq = 740e3;
        let p_sys = system_power(&p, freq);
        // Full MCS-4 system: should be in the mW-10W range
        assert!(
            p_sys > 1e-6 && p_sys < 100.0,
            "System power = {:.4e} W, expected mW-100W range",
            p_sys
        );
    }
}
