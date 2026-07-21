//! Generated from docs/evidence/intellec/mod40_route_ledger_v1.json.
//! Run `just mod40-evidence-generate` after changing the canonical ledger.

pub(crate) const MOD40_EVIDENCE_GATE_IDS: [&str; 6] = [
    "cpu-phase-reset",
    "in28-write-timing",
    "panel-arbitration",
    "terminal-electrical",
    "monitor-socket-transform",
    "monitor-raw-provenance",
];

pub(crate) const MOD40_EVIDENCE_GATE_CLOSED: [bool; 6] = [false, false, false, false, false, false];

pub(crate) const MONITOR_SOCKET_MAP_TRACED: bool = false;
pub(crate) const MONITOR_DATA_TRANSFORM_PRIMARY_BACKED: bool = false;
pub(crate) const ACCEPTED_MONITOR_READ_SET_COUNT: u8 = 0;
