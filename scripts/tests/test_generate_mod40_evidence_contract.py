from __future__ import annotations

import copy
import json
from pathlib import Path

from scripts.generate_mod40_evidence_contract import generate_contract
from scripts.verify_mod40_evidence import load_source_ids, validate_ledger

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
LEDGER_PATH = REPOSITORY_ROOT / "docs/evidence/intellec/mod40_route_ledger_v1.json"
SOURCE_PATH = REPOSITORY_ROOT / "docs/evidence/intellec_sources.yaml"
GENERATED_PATH = REPOSITORY_ROOT / "mcs4-emu/crates/mcs4-intellec/src/mod40_evidence_generated.rs"


def canonical_ledger() -> dict[str, object]:
    return json.loads(LEDGER_PATH.read_text(encoding="ascii"))


def test_generated_contract_matches_canonical_ledger() -> None:
    ledger = canonical_ledger()
    validate_ledger(ledger, load_source_ids(SOURCE_PATH))

    assert GENERATED_PATH.read_text(encoding="ascii") == generate_contract(ledger)


def test_generated_gate_order_follows_canonical_ledger_order() -> None:
    contract = generate_contract(canonical_ledger())

    positions = [contract.index(f'    "{gate_id}",') for gate_id in canonical_ledger()["source_gates"]]
    assert positions == sorted(positions)


def test_generated_monitor_projection_requires_atomic_evidence() -> None:
    ledger = copy.deepcopy(canonical_ledger())
    requirement = next(
        item for item in ledger["closure_requirements"] if item["id"] == "monitor-a18-output-socket-map"
    )
    requirement["state"] = "verified"

    contract = generate_contract(ledger)

    assert "pub(crate) const MONITOR_SOCKET_MAP_TRACED: bool = false;" in contract


def test_generated_read_count_uses_distinct_custody_requirements() -> None:
    ledger = copy.deepcopy(canonical_ledger())
    for requirement in ledger["closure_requirements"]:
        if requirement["id"] in {"monitor-first-raw-read-set", "monitor-independent-custody-set"}:
            requirement["state"] = "verified"

    contract = generate_contract(ledger)

    assert "pub(crate) const ACCEPTED_MONITOR_READ_SET_COUNT: u8 = 2;" in contract
    assert "pub(crate) const MOD40_EVIDENCE_GATE_CLOSED: [bool; 6] = [false, false, false, false, false, false];" in contract
