//! I/O driver model for MCS-4 chip data bus pins.
//!
//! WHY: Accurate bus timing requires modeling the output driver's on-resistance,
//! slew rate, and output impedance. These determine how fast a chip can drive
//! the shared 4-bit data bus and whether signal integrity is maintained.
//!
//! WHAT: Ron, slew rate, and output impedance for pMOS open-drain and
//! push-pull drivers on the 4004 data bus.
//!
//! HOW: Derived from the enhancement/depletion transistor parameters in
//! ProcessParams with geometry scaled to typical I/O pad transistors.

use super::ProcessParams;

/// I/O pad transistor width (m). Larger than internal transistors for
/// driving external capacitive loads.
const IO_PAD_W: f64 = 100e-6; // 100um

/// I/O pad transistor length (m). Minimum feature size.
const IO_PAD_L: f64 = 10e-6; // 10um

/// Typical external load capacitance on the data bus (F).
/// Includes package pin, PCB trace, and fanout to other chips.
const BUS_LOAD_CAP: f64 = 30e-12; // 30pF

/// On-resistance of a pMOS output driver in the triode region (ohms).
///
/// Ron ~ 1 / (beta * (Vgs - Vth)) for a transistor in deep triode.
///
/// # Arguments
/// * `params` - process parameters
/// * `w` - transistor width (m)
/// * `l` - transistor length (m)
pub fn ron_pmos(params: &ProcessParams, w: f64, l: f64) -> f64 {
    let beta = params.beta(w, l);
    let vgs = params.vdd; // Gate driven to VDD for full ON
    let vth = params.vth_enhancement;
    let vov = vgs - vth; // Overdrive voltage (negative for pMOS)

    if vov.abs() < 1e-6 {
        return 1e9; // Effectively off
    }

    // Ron = 1 / (beta * |Vov|) for pMOS in triode
    1.0 / (beta * vov.abs())
}

/// On-resistance using default I/O pad geometry.
pub fn ron_io_pad(params: &ProcessParams) -> f64 {
    ron_pmos(params, IO_PAD_W, IO_PAD_L)
}

/// Output slew rate (V/s) for an open-drain driver.
///
/// Slew rate is determined by the driver current charging the load
/// capacitance: SR = I_drive / C_load.
///
/// # Arguments
/// * `params` - process parameters
/// * `c_load` - load capacitance (F)
pub fn slew_rate(params: &ProcessParams, c_load: f64) -> f64 {
    let ron = ron_io_pad(params);

    if c_load <= 0.0 || ron <= 0.0 {
        return 0.0;
    }

    // Drive current at midpoint: I ~ Vdd / (2 * Ron)
    let i_drive = params.vdd.abs() / (2.0 * ron);

    // SR = I / C
    i_drive / c_load
}

/// Rise/fall time (10%-90%) for the data bus (seconds).
///
/// t_rf ~ 2.2 * Ron * C_load (RC time constant approach)
pub fn rise_fall_time(params: &ProcessParams, c_load: f64) -> f64 {
    let ron = ron_io_pad(params);
    2.2 * ron * c_load
}

/// Default rise/fall time for the 4004 data bus.
pub fn bus_rise_fall_time(params: &ProcessParams) -> f64 {
    rise_fall_time(params, BUS_LOAD_CAP)
}

/// Output impedance at a given frequency (ohms).
///
/// Z_out = Ron || (1 / (2*pi*f*C_out))
///
/// At DC, Z_out = Ron. At high frequency, capacitive path dominates.
///
/// # Arguments
/// * `params` - process parameters
/// * `freq` - frequency (Hz)
/// * `c_out` - output capacitance (F)
pub fn output_impedance(params: &ProcessParams, freq: f64, c_out: f64) -> f64 {
    let ron = ron_io_pad(params);

    if freq <= 0.0 || c_out <= 0.0 {
        return ron;
    }

    let x_c = 1.0 / (2.0 * std::f64::consts::PI * freq * c_out);
    // Parallel combination: Ron || Xc
    (ron * x_c) / (ron.hypot(x_c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ron_reasonable_for_10um_pmos() {
        let p = ProcessParams::default();
        let ron = ron_io_pad(&p);
        // For 100um/10um pMOS with Vov~12V, Ron should be in 100-10000 ohm range
        assert!(
            ron > 10.0 && ron < 100_000.0,
            "Ron = {:.1} ohm, expected 10-100k range",
            ron
        );
    }

    #[test]
    fn slew_rate_positive() {
        let p = ProcessParams::default();
        let sr = slew_rate(&p, BUS_LOAD_CAP);
        assert!(sr > 0.0, "Slew rate should be positive");
        // Should be in V/us range (1e6 - 1e9 V/s)
        assert!(sr > 1e5 && sr < 1e12, "SR = {:.2e} V/s, expected 1e5-1e12 range", sr);
    }

    #[test]
    fn bus_rise_fall_in_ns_range() {
        let p = ProcessParams::default();
        let t_rf = bus_rise_fall_time(&p);
        // For 10um pMOS driving 30pF, expect ~10-500ns
        assert!(
            t_rf > 1e-9 && t_rf < 1e-6,
            "Rise/fall = {:.2e} s, expected 1-1000 ns range",
            t_rf
        );
    }

    #[test]
    fn output_impedance_decreases_with_frequency() {
        let p = ProcessParams::default();
        let c_out = 10e-12; // 10pF
        let z_dc = output_impedance(&p, 0.0, c_out);
        let z_1mhz = output_impedance(&p, 1e6, c_out);
        let z_100mhz = output_impedance(&p, 100e6, c_out);

        assert!(z_dc >= z_1mhz, "Z should decrease with frequency");
        assert!(z_1mhz >= z_100mhz, "Z should decrease with frequency");
    }
}
