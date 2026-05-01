//! Fuzz target: NetlistV1 JSON deserialization + circuit bridge.
//!
//! Invariant: arbitrary byte sequences fed as UTF-8 JSON must never cause a
//! panic.  Parse failures and empty graphs are both acceptable outcomes.

#![no_main]

use libfuzzer_sys::fuzz_target;
use mcs4_core::{
    circuit::netlist_bridge::{self, BridgeConfig},
    layout_netlist::NetlistV1,
};

fuzz_target!(|data: &[u8]| {
    // Accept only valid UTF-8; non-UTF-8 bytes are uninteresting for a JSON parser.
    let Ok(s) = std::str::from_utf8(data) else { return };

    let Ok(netlist) = serde_json::from_str::<NetlistV1>(s) else { return };

    // If parsing succeeded, building the circuit graph must also not panic.
    let config = BridgeConfig::default();
    let _graph = netlist_bridge::netlist_v1_to_circuit(&netlist, &config);
});
