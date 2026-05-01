//! Fuzz target: ROM hex fixture loader.
//!
//! Invariant: any byte sequence interpreted as a text fixture file must not
//! cause a panic.  Parse errors and TooLarge errors are both acceptable
//! outcomes; panics are not.

#![no_main]

use libfuzzer_sys::fuzz_target;
use mcs4_system::fixture::{parse_hex_bytes, parse_hex_bytes_bounded};

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else { return };

    // Unbounded parse: must not panic regardless of content.
    let _ = parse_hex_bytes(s);

    // Bounded parse: enforce 4001 single-bank limit (256 bytes).
    let _ = parse_hex_bytes_bounded(s, 256);
});
