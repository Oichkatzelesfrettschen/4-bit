"""Focused policy tests for RustSec advisory exception verification."""

from __future__ import annotations

import datetime as dt

import pytest

import verify_advisory_exceptions as advisory


def _exception(
    identifier: str, package: str, category: str, expiry: str = "2026-10-01"
) -> dict[str, object]:
    return {
        "id": identifier,
        "package": package,
        "category": category,
        "expires": expiry,
        "scope": "focused test scope",
        "risk": "focused test risk",
        "mitigation": "focused test mitigation",
        "review": "focused test review",
        "dependency_paths": ["root -> dependency"],
    }


def test_exception_entries_reject_expired_records() -> None:
    document = {
        "schema_version": 1,
        "exception": [_exception("RUSTSEC-2026-0001", "crate", "vulnerabilities", "2026-01-01")],
    }
    with pytest.raises(ValueError, match="expired"):
        advisory.exception_entries(document, dt.date(2026, 7, 11))


def test_verify_rejects_untracked_audit_advisory() -> None:
    exceptions = {
        "RUSTSEC-2026-0001": _exception("RUSTSEC-2026-0001", "crate", "vulnerabilities"),
    }
    errors = advisory.verify(
        exceptions,
        {"RUSTSEC-2026-0001"},
        {
            "RUSTSEC-2026-0001": {("vulnerabilities", "crate")},
            "RUSTSEC-2026-0002": {("unmaintained", "other")},
        },
    )
    assert any("untracked" in error for error in errors)


def test_verify_requires_deny_and_audit_sets_to_match_registry() -> None:
    exceptions = {
        "RUSTSEC-2026-0001": _exception("RUSTSEC-2026-0001", "crate", "vulnerabilities"),
    }
    assert (
        advisory.verify(
            exceptions,
            {"RUSTSEC-2026-0001"},
            {"RUSTSEC-2026-0001": {("vulnerabilities", "crate")}},
        )
        == []
    )
    errors = advisory.verify(
        exceptions, set(), {"RUSTSEC-2026-0001": {("vulnerabilities", "crate")}}
    )
    assert any("omits" in error for error in errors)
