from __future__ import annotations

import copy
import json
from pathlib import Path

import pytest
from scripts.verify_mod40_evidence import (
    EvidenceValidationError,
    build_status_report,
    load_source_ids,
    validate_ledger,
    validate_pin_net_ledger,
    write_status_report,
)

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
LEDGER_PATH = REPOSITORY_ROOT / "docs/evidence/intellec/mod40_route_ledger_v1.json"
SOURCE_PATH = REPOSITORY_ROOT / "docs/evidence/intellec_sources.yaml"
PIN_NET_PATH = REPOSITORY_ROOT / "docs/evidence/intellec/mod40_component_pin_net_v1.json"


def canonical_ledger() -> dict[str, object]:
    return json.loads(LEDGER_PATH.read_text(encoding="ascii"))


def canonical_pin_net_ledger() -> dict[str, object]:
    return json.loads(PIN_NET_PATH.read_text(encoding="ascii"))


def test_canonical_mod40_route_ledger_validates() -> None:
    validate_ledger(canonical_ledger(), load_source_ids(SOURCE_PATH))


def test_canonical_mod40_component_pin_ledger_validates() -> None:
    validate_pin_net_ledger(canonical_pin_net_ledger(), canonical_ledger(), load_source_ids(SOURCE_PATH))


def test_component_pin_segment_rejects_unknown_endpoint() -> None:
    pin_ledger = copy.deepcopy(canonical_pin_net_ledger())
    pin_ledger["records"][0]["segments"][0]["to"] = "invented-pin"

    with pytest.raises(EvidenceValidationError, match="segment references unknown endpoint"):
        validate_pin_net_ledger(pin_ledger, canonical_ledger(), load_source_ids(SOURCE_PATH))


def test_component_pin_record_rejects_ocr_only_evidence() -> None:
    pin_ledger = copy.deepcopy(canonical_pin_net_ledger())
    pin_ledger["records"][0]["source_refs"][0]["ocr_only"] = True

    with pytest.raises(EvidenceValidationError, match="OCR-only evidence"):
        validate_pin_net_ledger(pin_ledger, canonical_ledger(), load_source_ids(SOURCE_PATH))


def test_component_pin_record_keeps_connectivity_and_behavior_separate() -> None:
    pin_ledger = canonical_pin_net_ledger()

    assert all(record["connectivity_status"] == "direct" for record in pin_ledger["records"])
    assert all(record["behavior_status"] == "partial" for record in pin_ledger["records"])


def test_status_report_preserves_every_gate_and_atomic_requirement(tmp_path: Path) -> None:
    ledger = canonical_ledger()
    validate_ledger(ledger, load_source_ids(SOURCE_PATH))

    report = build_status_report(ledger)
    report_path = tmp_path / "status.json"
    write_status_report(report_path, report)

    assert report["schema"] == "mcs4.mod40.evidence-status.v1"
    assert report["requirement_counts"] == {"missing": 29, "partial": 0, "total": 29, "verified": 0}
    assert [gate["id"] for gate in report["gates"]] == [
        "cpu-phase-reset",
        "in28-write-timing",
        "panel-arbitration",
        "terminal-electrical",
        "monitor-socket-transform",
        "monitor-raw-provenance",
    ]
    assert report["work_queue"]["ready_requirement_ids"] == [
        "cpu-divider-state-equation",
        "cpu-phase-output-polarity",
        "cpu-reset-inversion-route",
        "in28-local-3404-logic",
        "in28-one-shot-components",
        "monitor-a18-output-socket-map",
        "monitor-data-bit-routes",
        "monitor-first-raw-read-set",
        "monitor-independent-custody-set",
        "panel-stop-continuity",
        "terminal-q3-reader-driver",
        "terminal-q4-printer-driver",
        "terminal-q5-keyboard-receiver",
    ]
    assert len(report["work_queue"]["blocked_requirement_ids"]) == 16
    assert report["work_queue"]["verified_requirement_ids"] == []
    assert report["work_queue"]["topological_requirement_ids"].index("in28-local-3404-logic") < report[
        "work_queue"
    ]["topological_requirement_ids"].index("in28-setup-hold-budget")
    assert json.loads(report_path.read_text(encoding="ascii")) == report


def test_partial_route_requires_a_named_missing_fact() -> None:
    ledger = copy.deepcopy(canonical_ledger())
    route = next(record for record in ledger["routes"] if record["id"] == "monitor-data-transform")
    route["unresolved"] = []

    with pytest.raises(EvidenceValidationError, match="does not name its unresolved evidence"):
        validate_ledger(ledger, load_source_ids(SOURCE_PATH))


def test_partial_route_requires_atomic_closure_requirements() -> None:
    ledger = copy.deepcopy(canonical_ledger())
    route = next(record for record in ledger["routes"] if record["id"] == "monitor-data-transform")
    route["closure_requirement_ids"] = []

    with pytest.raises(EvidenceValidationError, match="has no closure requirements"):
        validate_ledger(ledger, load_source_ids(SOURCE_PATH))


def test_closure_requirement_must_belong_to_its_route_gate() -> None:
    ledger = copy.deepcopy(canonical_ledger())
    route = next(record for record in ledger["routes"] if record["id"] == "monitor-data-transform")
    route["closure_requirement_ids"] = ["cpu-divider-state-equation"]

    with pytest.raises(EvidenceValidationError, match="crosses closure requirement gate"):
        validate_ledger(ledger, load_source_ids(SOURCE_PATH))


def test_blocked_gate_requires_a_partial_blocking_route() -> None:
    ledger = copy.deepcopy(canonical_ledger())
    ledger["source_gates"]["cpu-phase-reset"]["blocking_routes"] = ["cpu-clock-source"]

    with pytest.raises(EvidenceValidationError, match="has no partial blocking route"):
        validate_ledger(ledger, load_source_ids(SOURCE_PATH))


def test_closed_gate_requires_every_atomic_requirement_to_be_verified() -> None:
    ledger = copy.deepcopy(canonical_ledger())
    ledger["source_gates"]["cpu-phase-reset"]["status"] = "closed"
    for route in ledger["routes"]:
        if route["gate"] == "cpu-phase-reset":
            route["evidence"] = "direct"
            route["unresolved"] = []

    with pytest.raises(EvidenceValidationError, match="retains incomplete closure requirements"):
        validate_ledger(ledger, load_source_ids(SOURCE_PATH))


def test_closure_requirement_dependencies_must_exist_and_remain_acyclic() -> None:
    ledger = copy.deepcopy(canonical_ledger())
    requirement = next(record for record in ledger["closure_requirements"] if record["id"] == "cpu-divider-state-equation")
    requirement["depends_on"] = ["not-a-requirement"]

    with pytest.raises(EvidenceValidationError, match="references unknown dependencies"):
        validate_ledger(ledger, load_source_ids(SOURCE_PATH))

    ledger = copy.deepcopy(canonical_ledger())
    divider = next(record for record in ledger["closure_requirements"] if record["id"] == "cpu-divider-state-equation")
    phase = next(record for record in ledger["closure_requirements"] if record["id"] == "cpu-phase-output-polarity")
    divider["depends_on"] = ["cpu-phase-output-polarity"]
    phase["depends_on"] = ["cpu-divider-state-equation"]

    with pytest.raises(EvidenceValidationError, match="dependencies contain a cycle"):
        validate_ledger(ledger, load_source_ids(SOURCE_PATH))


def test_verified_requirement_cannot_bypass_an_incomplete_dependency() -> None:
    ledger = copy.deepcopy(canonical_ledger())
    requirement = next(record for record in ledger["closure_requirements"] if record["id"] == "cpu-4040-clock-edge-timing")
    requirement["state"] = "verified"

    with pytest.raises(EvidenceValidationError, match="depends on incomplete requirement"):
        validate_ledger(ledger, load_source_ids(SOURCE_PATH))
