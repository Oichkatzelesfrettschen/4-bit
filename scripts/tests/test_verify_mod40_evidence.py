from __future__ import annotations

import copy
import json
from pathlib import Path

import pytest
from scripts.verify_mod40_evidence import EvidenceValidationError, load_source_ids, validate_ledger

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
LEDGER_PATH = REPOSITORY_ROOT / "docs/evidence/intellec/mod40_route_ledger_v1.json"
SOURCE_PATH = REPOSITORY_ROOT / "docs/evidence/intellec_sources.yaml"


def canonical_ledger() -> dict[str, object]:
    return json.loads(LEDGER_PATH.read_text(encoding="ascii"))


def test_canonical_mod40_route_ledger_validates() -> None:
    validate_ledger(canonical_ledger(), load_source_ids(SOURCE_PATH))


def test_partial_route_requires_a_named_missing_fact() -> None:
    ledger = copy.deepcopy(canonical_ledger())
    route = next(record for record in ledger["routes"] if record["id"] == "monitor-data-transform")
    route["unresolved"] = []

    with pytest.raises(EvidenceValidationError, match="does not name its unresolved evidence"):
        validate_ledger(ledger, load_source_ids(SOURCE_PATH))


def test_blocked_gate_requires_a_partial_blocking_route() -> None:
    ledger = copy.deepcopy(canonical_ledger())
    ledger["source_gates"]["cpu-phase-reset"]["blocking_routes"] = ["cpu-clock-source"]

    with pytest.raises(EvidenceValidationError, match="has no partial blocking route"):
        validate_ledger(ledger, load_source_ids(SOURCE_PATH))
