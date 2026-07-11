//! Tests backing the documented 4004 timing claims against the datasheet
//! constants in `mcs4_core::timing::clock_spec`.

use mcs4_core::timing::{clock_spec, MICROSECOND, NANOSECOND};

/// An MCS-4 instruction cycle spans 8 clock periods (A1, A2, A3, M1, M2,
/// X1, X2, X3). At the typical 1.35 us clock period this is the documented
/// 10.8 us instruction cycle (740 kHz clock, ~92.6 kIPS).
#[test]
fn instruction_cycle_is_10_8_us_at_typical_clock() {
    let instruction_cycle = 8 * clock_spec::TCY_TYP;
    assert_eq!(instruction_cycle, 10_800 * NANOSECOND);
    // Same quantity expressed in datasheet units: 10.8 us.
    assert_eq!(instruction_cycle * 10, 108 * MICROSECOND);
}

/// Clock-period bounds from the Intel 4004 datasheet: 1.35 us (740 kHz max
/// clock) to 2.0 us (500 kHz minimum clock), with the typical simulation
/// period inside the window.
#[test]
fn clock_period_bounds_match_datasheet() {
    assert_eq!(clock_spec::TCY_MIN, 1_350 * NANOSECOND);
    assert_eq!(clock_spec::TCY_MAX, 2_000 * NANOSECOND);
    const { assert!(clock_spec::TCY_MIN <= clock_spec::TCY_TYP) };
    const { assert!(clock_spec::TCY_TYP <= clock_spec::TCY_MAX) };
}
