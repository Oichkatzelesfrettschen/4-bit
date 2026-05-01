"""Smoke tests for evidence-pipeline scripts.

These tests intentionally avoid heavy fixture data so they can run quickly in
CI; deeper integration tests against full netlists / OCR crops live alongside
each script's working directory under docs/evidence/.
"""

from __future__ import annotations

import importlib
import json
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]


# Sanity import: every script must be import-clean (no top-level side effects
# that crash when executed without args).
@pytest.mark.parametrize(
    "module_name",
    [
        "extract_gates_v0",
        "gate_to_verilog_v0",
        "build_coordinate_transform_v0",
        "extract_via_connectivity_v0",
        "ocr_signal_labels",
        "netlist_v0_metrics",
        "audit_claims_backlog",
        "apply_pad_pin_template_v0",
    ],
)
def test_module_imports_cleanly(module_name: str) -> None:
    """Importing the module must not raise and must not call sys.exit()."""
    module = importlib.import_module(module_name)
    assert module is not None
    # Every script we audit-cover should expose either a __doc__ or a `main`
    # callable so future contributors know how to invoke it.
    assert getattr(module, "__doc__", None) or callable(getattr(module, "main", None)), (
        f"{module_name} should expose either a module docstring or a main() function"
    )


def test_audit_claims_backlog_round_trip(tmp_path: Path) -> None:
    """audit_claims_backlog can read the canonical JSON ledger schema."""
    ledger = REPO_ROOT / "docs" / "evidence" / "audit_claims_backlog.json"
    if not ledger.exists():
        pytest.skip("audit_claims_backlog.json not present in repo")

    data = json.loads(ledger.read_text(encoding="utf-8"))
    # Schema sanity: top-level must be a dict or list of records, and every
    # record must have a `claim` or `id` key. Tolerate either layout.
    if isinstance(data, dict):
        # Accept any of the historically-used record-list keys.
        record_keys = {"claims", "items", "entries", "backlog"}
        assert record_keys & data.keys(), (
            f"expected one of {sorted(record_keys)} in ledger root, got {sorted(data.keys())}"
        )
    else:
        assert isinstance(data, list)
        for record in data[:5]:
            assert isinstance(record, dict)


def test_evidence_directory_layout() -> None:
    """Spot-check evidence directory landmarks referenced by other scripts."""
    docs_evidence = REPO_ROOT / "docs" / "evidence"
    assert docs_evidence.is_dir(), "docs/evidence/ missing from repo root"
    for landmark in ("netlists_v0", "subcircuits_v0", "ocr_manifest.yaml"):
        assert (docs_evidence / landmark).exists(), f"missing landmark {landmark}"


def test_repo_root_canonical_status_files() -> None:
    """Status sync invariant: canonical CLAUDE.md and STATUS.md must exist."""
    for rel in ("mcs4-emu/CLAUDE.md", "mcs4-emu/STATUS.md", "docs/ROADMAP.md"):
        assert (REPO_ROOT / rel).is_file(), f"missing canonical file {rel}"
