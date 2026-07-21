#!/usr/bin/env python3
"""Generate the Rust MOD 40 evidence contract from the canonical ledger."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import cast

try:
    from scripts.verify_mod40_evidence import load_source_ids, validate_ledger
except ModuleNotFoundError:
    from verify_mod40_evidence import load_source_ids, validate_ledger

DEFAULT_LEDGER = Path("docs/evidence/intellec/mod40_route_ledger_v1.json")
DEFAULT_SOURCES = Path("docs/evidence/intellec_sources.yaml")
DEFAULT_OUTPUT = Path("mcs4-emu/crates/mcs4-intellec/src/mod40_evidence_generated.rs")


def rust_bool(value: bool) -> str:
    """Return one Rust Boolean literal."""

    return "true" if value else "false"


def generate_contract(ledger: dict[str, object]) -> str:
    """Render deterministic Rust constants from a validated evidence ledger."""

    gates = cast(dict[str, dict[str, object]], ledger["source_gates"])
    requirements = cast(list[dict[str, object]], ledger["closure_requirements"])
    requirement_verified = {str(item["id"]): item["state"] == "verified" for item in requirements}
    gate_items = list(gates.items())

    def all_verified(*requirement_ids: str) -> bool:
        return all(requirement_verified[requirement_id] for requirement_id in requirement_ids)

    monitor_socket_map_traced = all_verified(
        "monitor-a18-output-socket-map",
        "monitor-chip-select-polarity",
        "monitor-address-block-socket-order",
    )
    monitor_data_transform_primary_backed = all_verified(
        "monitor-data-bit-routes",
        "monitor-data-inversion-vector",
    )
    accepted_monitor_read_set_count = sum(
        requirement_verified[requirement_id]
        for requirement_id in ("monitor-first-raw-read-set", "monitor-independent-custody-set")
    )

    lines = [
        "//! Generated from docs/evidence/intellec/mod40_route_ledger_v1.json.",
        "//! Run `just mod40-evidence-generate` after changing the canonical ledger.",
        "",
        f"pub(crate) const MOD40_EVIDENCE_GATE_IDS: [&str; {len(gate_items)}] = [",
    ]
    lines.extend(f'    "{gate_id}",' for gate_id, _gate in gate_items)
    gate_closed_values = ", ".join(rust_bool(gate["status"] == "closed") for _gate_id, gate in gate_items)
    lines.extend(
        [
            "];",
            "",
            f"pub(crate) const MOD40_EVIDENCE_GATE_CLOSED: [bool; {len(gate_items)}] = [{gate_closed_values}];",
            "",
            "pub(crate) const MONITOR_SOCKET_MAP_TRACED: bool = " + rust_bool(monitor_socket_map_traced) + ";",
            "pub(crate) const MONITOR_DATA_TRANSFORM_PRIMARY_BACKED: bool = "
            + rust_bool(monitor_data_transform_primary_backed)
            + ";",
            f"pub(crate) const ACCEPTED_MONITOR_READ_SET_COUNT: u8 = {accepted_monitor_read_set_count};",
            "",
        ]
    )
    return "\n".join(lines)


def main(arguments: list[str] | None = None) -> int:
    """Validate inputs and write or check the generated Rust contract."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ledger", type=Path, default=DEFAULT_LEDGER)
    parser.add_argument("--sources", type=Path, default=DEFAULT_SOURCES)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    parsed = parser.parse_args(arguments)

    with parsed.ledger.open(encoding="ascii") as handle:
        ledger = json.load(handle)
    validate_ledger(ledger, load_source_ids(parsed.sources))
    if not isinstance(ledger, dict):
        raise TypeError("validated ledger root is not a mapping")
    generated = generate_contract(ledger)

    if parsed.check:
        try:
            existing = parsed.output.read_text(encoding="ascii")
        except FileNotFoundError:
            print(f"generated MOD 40 evidence contract is missing: {parsed.output}")
            return 1
        if existing != generated:
            print(f"generated MOD 40 evidence contract is stale: {parsed.output}")
            return 1
        print(f"PASS generated MOD 40 evidence contract: {parsed.output}")
        return 0

    parsed.output.parent.mkdir(parents=True, exist_ok=True)
    parsed.output.write_text(generated, encoding="ascii")
    print(f"WROTE generated MOD 40 evidence contract: {parsed.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
