"""Focused tests for the netlist v1 provenance manifest verifier."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import verify_netlist_manifest as netlists


def _write_fixture(root: Path) -> dict[str, object]:
    source = root / "docs" / "evidence" / "inputs" / "source.json"
    output = root / "docs" / "evidence" / "netlists_v1" / "4001_netlist_v1.json"
    source.parent.mkdir(parents=True)
    output.parent.mkdir(parents=True)
    source.write_text('{"source": true}\n', encoding="utf-8")
    payload = {
        "chip": "4001",
        "schema": {"version": 1},
        "inputs": {
            "source": "docs/evidence/inputs/source.json",
            "sha256": {"source": hashlib.sha256(source.read_bytes()).hexdigest()},
        },
    }
    output.write_text(json.dumps(payload) + "\n", encoding="utf-8")
    return {
        "schema_version": 1,
        "tool": "scripts/build_netlist_v1_v0.py",
        "params": {"max_transistor_bbox_area": 1, "max_transistor_bbox_dim": 1},
        "outputs": [
            {
                "chip": "4001",
                "output": "docs/evidence/netlists_v1/4001_netlist_v1.json",
                "sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
            }
        ],
    }


def test_canonical_manifest_passes() -> None:
    document = json.loads(netlists.DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    assert netlists.verify_manifest(document) == []


def test_fixture_manifest_validates_output_and_inputs(tmp_path: Path) -> None:
    document = _write_fixture(tmp_path)
    assert (
        netlists.verify_manifest(document, root=tmp_path, expected_chips=frozenset({"4001"})) == []
    )


def test_manifest_rejects_stale_output_hash(tmp_path: Path) -> None:
    document = _write_fixture(tmp_path)
    output = document["outputs"][0]
    assert isinstance(output, dict)
    output["sha256"] = "0" * 64
    errors = netlists.verify_manifest(document, root=tmp_path, expected_chips=frozenset({"4001"}))
    assert any("output SHA-256 does not match" in error for error in errors)


def test_manifest_rejects_input_path_escape(tmp_path: Path) -> None:
    document = _write_fixture(tmp_path)
    output_path = tmp_path / "docs" / "evidence" / "netlists_v1" / "4001_netlist_v1.json"
    payload = json.loads(output_path.read_text(encoding="utf-8"))
    payload["inputs"]["source"] = "../../outside.json"
    output_path.write_text(json.dumps(payload) + "\n", encoding="utf-8")
    entry = document["outputs"][0]
    assert isinstance(entry, dict)
    entry["sha256"] = hashlib.sha256(output_path.read_bytes()).hexdigest()
    errors = netlists.verify_manifest(document, root=tmp_path, expected_chips=frozenset({"4001"}))
    assert any("path escapes repository" in error for error in errors)
