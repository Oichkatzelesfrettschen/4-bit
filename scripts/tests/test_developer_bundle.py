"""Focused contract tests for the clean-tree developer bundle builder."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

import build_developer_bundle as bundle


def _write_export_pair(tmp_path: Path, *, dirty: bool = False) -> tuple[Path, Path, str]:
    revision = "a" * 40
    verilog = tmp_path / "i4003_fpga.v"
    manifest = tmp_path / "i4003_fpga.v.manifest.json"
    verilog.write_text("module i4003_fpga; endmodule\n", encoding="utf-8")
    manifest.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "source_revision": revision,
                "source_dirty": dirty,
                "request": {"chip": "i4003", "flavor": "fpga"},
                "output_sha256": hashlib.sha256(verilog.read_bytes()).hexdigest(),
            }
        )
        + "\n",
        encoding="utf-8",
    )
    return manifest, verilog, revision


def test_export_manifest_accepts_clean_i4003_fpga_output(tmp_path: Path) -> None:
    manifest, verilog, revision = _write_export_pair(tmp_path)
    assert bundle.validate_export_manifest(manifest, verilog, revision) == []


def test_export_manifest_rejects_dirty_source(tmp_path: Path) -> None:
    manifest, verilog, revision = _write_export_pair(tmp_path, dirty=True)
    errors = bundle.validate_export_manifest(manifest, verilog, revision)
    assert "exporter manifest must record source_dirty=false" in errors


def test_bundle_directory_rejects_escape(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    bundle_root = tmp_path / "target" / "developer-bundle"
    monkeypatch.setattr(bundle, "BUNDLE_ROOT", bundle_root)
    with pytest.raises(bundle.BundleError, match="bundle output must remain"):
        bundle.bundle_directory(tmp_path / "outside", "a" * 40)
