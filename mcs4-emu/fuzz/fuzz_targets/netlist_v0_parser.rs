//! Fuzz target: NetlistV0 JSON deserialization.
//!
//! Invariant: arbitrary byte sequences fed as UTF-8 JSON must never cause a
//! panic. Parse failures are an acceptable outcome; the v0 schema is the
//! committed evidence-loading path and must reject malformed input cleanly.

#![no_main]

use libfuzzer_sys::fuzz_target;
use mcs4_core::netlist_v0::NetlistV0;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else { return };
    let _ = serde_json::from_str::<NetlistV0>(s);
});
