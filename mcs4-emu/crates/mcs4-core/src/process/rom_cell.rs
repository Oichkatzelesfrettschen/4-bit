//! ROM cell model for the Intel 4001 mask-programmed ROM.
//!
//! WHY: The 4001 ROM is the primary instruction store for MCS-4 systems.
//! Modeling its access time requires understanding the single-transistor
//! cell, wordline RC delay, and bitline charge sharing.
//!
//! WHAT: Single-transistor ROM cell with wordline and bitline parasitics.
//! Predicts read access time based on process parameters and array geometry.
//!
//! HOW: Each ROM cell is a single enhancement pMOS transistor connected
//! between a bitline and ground. The gate is connected to a wordline.
//! A programmed '0' has no transistor (or transistor with high Vth);
//! a programmed '1' has an active transistor that discharges the bitline.

use super::ProcessParams;

/// ROM array dimensions for the 4001 (256 x 8 bits).
pub const ROM_4001_ROWS: usize = 256;
pub const ROM_4001_COLS: usize = 8;

/// ROM cell transistor width (m). Minimum geometry for density.
const CELL_W: f64 = 10e-6; // 10um (minimum feature)

/// ROM cell transistor length (m).
const CELL_L: f64 = 10e-6; // 10um

/// Wordline pitch (m). Cell-to-cell spacing along a row.
const WORDLINE_PITCH: f64 = 20e-6; // 20um

/// Bitline pitch (m). Cell-to-cell spacing along a column.
/// Used for bitline length calculations in future detailed models.
#[allow(dead_code)]
const BITLINE_PITCH: f64 = 15e-6; // 15um

/// Wordline width (m). Polysilicon.
const WORDLINE_WIDTH: f64 = 5e-6; // 5um

/// Bitline width (m). Diffusion or metal.
/// Used for bitline resistance calculations in future detailed models.
#[allow(dead_code)]
const BITLINE_WIDTH: f64 = 5e-6; // 5um

/// Wordline RC delay for the full array (seconds).
///
/// The wordline is polysilicon, so it has significant RC delay.
/// t_wl = R_total * C_total / 2 (Elmore approximation for distributed RC)
///
/// # Arguments
/// * `params` - process parameters
/// * `num_cols` - number of columns in the array
pub fn wordline_delay(params: &ProcessParams, num_cols: usize) -> f64 {
    use super::{
        interconnect::{parallel_plate_cap, R_SHEET_POLY},
        silicon::EPSILON_OX,
    };

    let wl_length = num_cols as f64 * WORDLINE_PITCH;

    // Wordline resistance: R = Rsh * L / W
    let r_wl = R_SHEET_POLY * wl_length / WORDLINE_WIDTH;

    // Wordline capacitance: gate cap of all cells + wire cap
    // Each cell contributes one gate capacitance
    let c_gate_per_cell = params.cgate(CELL_W, CELL_L);
    let c_gates = num_cols as f64 * c_gate_per_cell;

    // Wire capacitance (poly over field oxide, ~500nm)
    let field_oxide_thickness = 500e-9;
    let c_wire_per_area = parallel_plate_cap(EPSILON_OX, field_oxide_thickness);
    let c_wire = c_wire_per_area * WORDLINE_WIDTH * wl_length;

    let c_total = c_gates + c_wire;

    // Elmore delay for distributed RC line
    r_wl * c_total / 2.0
}

/// Bitline charge sharing ratio.
///
/// When a ROM cell transistor turns on, it shares charge between the
/// precharged bitline capacitance and the cell. The voltage swing is:
///   delta_V = V_precharge * C_cell / (C_bitline + C_cell)
///
/// # Arguments
/// * `params` - process parameters
/// * `num_rows` - number of rows (determines bitline length)
pub fn bitline_voltage_swing(params: &ProcessParams, num_rows: usize) -> f64 {
    // Bitline capacitance: diffusion junction caps of all cells
    let cj0 = params.cj0();
    // Each cell contributes source/drain junction cap
    let c_junction_per_cell = cj0 * CELL_W * 2e-6; // junction area ~ W * xj
    let c_bitline = num_rows as f64 * c_junction_per_cell;

    // Precharge voltage (full VDD swing)
    let v_precharge = params.vdd.abs();

    // Cell discharge capacitance (gate cap acts as coupling)
    let c_cell = params.cgate(CELL_W, CELL_L);

    // Voltage swing on bitline when cell discharges
    v_precharge * c_cell / (c_bitline + c_cell)
}

/// Total ROM read access time (seconds).
///
/// t_access = t_wordline_delay + t_cell_discharge + t_sense
///
/// # Arguments
/// * `params` - process parameters
pub fn read_access_time(params: &ProcessParams) -> f64 {
    let t_wl = wordline_delay(params, ROM_4001_COLS);

    // Cell discharge time: RC of cell transistor driving bitline
    let ron_cell = 1.0 / (params.beta(CELL_W, CELL_L) * (params.vdd.abs() - params.vth_enhancement.abs()));
    let c_bitline = ROM_4001_ROWS as f64 * params.cj0() * CELL_W * 2e-6;
    let t_discharge = ron_cell * c_bitline;

    // Sense amplifier delay (empirical, ~20-50ns for 1970s design)
    let t_sense = 30e-9;

    t_wl + t_discharge + t_sense
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wordline_delay_positive() {
        let p = ProcessParams::default();
        let t_wl = wordline_delay(&p, ROM_4001_COLS);
        assert!(t_wl > 0.0 && t_wl.is_finite());
        // Should be in ns range for 8 columns
        assert!(t_wl < 1e-6, "Wordline delay = {:.2e} s, expected sub-microsecond", t_wl);
    }

    #[test]
    fn bitline_swing_reasonable() {
        let p = ProcessParams::default();
        let dv = bitline_voltage_swing(&p, ROM_4001_ROWS);
        // Swing should be a fraction of VDD
        assert!(
            dv > 0.0 && dv < p.vdd.abs(),
            "Bitline swing = {:.2}V, should be 0 < dV < |VDD|",
            dv
        );
    }

    #[test]
    fn read_access_time_in_expected_range() {
        let p = ProcessParams::default();
        let t_access = read_access_time(&p);
        // Published 4001 access time: ~500ns-1us at 740kHz
        // Our model includes sense amp delay, so should be 50-1000ns
        assert!(
            t_access > 10e-9 && t_access < 5e-6,
            "Access time = {:.2e} s, expected 10ns-5us range",
            t_access
        );
    }

    #[test]
    fn more_columns_increases_wordline_delay() {
        let p = ProcessParams::default();
        let t8 = wordline_delay(&p, 8);
        let t16 = wordline_delay(&p, 16);
        // Delay scales quadratically with length (distributed RC)
        assert!(
            t16 > t8 * 2.0,
            "Double columns should more than double WL delay: 8col={:.2e}, 16col={:.2e}",
            t8,
            t16
        );
    }
}
