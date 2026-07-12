"""Focused schema tests for the capability registry verifier."""

from __future__ import annotations

import verify_capability_registry as capabilities


def _record(**overrides: object) -> dict[str, object]:
    record: dict[str, object] = {
        "id": "test-capability",
        "state": "tested",
        "owner": "emulator-maintainers",
        "scope": "focused test capability",
        "evidence": [{"kind": "source", "path": "Cargo.toml"}],
        "limitations": ["focused limitation"],
        "next_gate": "focused next gate",
    }
    record.update(overrides)
    return record


def test_valid_registry_passes() -> None:
    assert capabilities.verify({"schema_version": 1, "capabilities": [_record()]}) == []


def test_blocked_record_requires_blocker() -> None:
    errors = capabilities.verify({"schema_version": 1, "capabilities": [_record(state="blocked")]})
    assert any("requires a blocker" in error for error in errors)


def test_synthesized_record_requires_synthesis_evidence() -> None:
    errors = capabilities.verify(
        {"schema_version": 1, "capabilities": [_record(state="synthesized")]}
    )
    assert any("requires synthesis evidence" in error for error in errors)


def test_record_requires_declared_owner() -> None:
    errors = capabilities.verify(
        {"schema_version": 1, "capabilities": [_record(owner="unknown-maintainers")]}
    )
    assert any("owner must be a declared maintenance role" in error for error in errors)
