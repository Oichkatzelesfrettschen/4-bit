"""Focused schema tests for the timing parameter provenance verifier."""

from __future__ import annotations

import copy
import json

import verify_timing_parameters as timing


def test_canonical_timing_ledger_passes() -> None:
    document = json.loads(timing.DEFAULT_LEDGER.read_text(encoding="utf-8"))
    assert timing.verify(document) == []


def test_ledger_rejects_out_of_range_selected_value() -> None:
    document = json.loads(timing.DEFAULT_LEDGER.read_text(encoding="utf-8"))
    mutated = copy.deepcopy(document)
    parameter = mutated["parameters"][0]
    assert isinstance(parameter, dict)
    parameter["selected_value"] = 1
    errors = timing.verify(mutated)
    assert any("selected_value must stay inside bounds" in error for error in errors)


def test_ledger_rejects_missing_source_token() -> None:
    document = json.loads(timing.DEFAULT_LEDGER.read_text(encoding="utf-8"))
    mutated = copy.deepcopy(document)
    parameter = mutated["parameters"][0]
    assert isinstance(parameter, dict)
    source = parameter["source"]
    assert isinstance(source, dict)
    source["required_tokens"] = ["not a retained source token"]
    errors = timing.verify(mutated)
    assert any("source locator lacks required tokens" in error for error in errors)
