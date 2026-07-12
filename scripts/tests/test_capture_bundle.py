"""Tests for checksummed capture-bundle manifests."""

from __future__ import annotations

import hashlib
import json
import tarfile
from pathlib import Path

import verify_capture_bundle as capture


def _make_capture(root: Path) -> None:
    (root / "static").mkdir(parents=True)
    (root / "cflow").mkdir()
    (root / "cscope").mkdir()
    (root / "mir").mkdir()
    (root / "python").mkdir()
    (root / "runtime").mkdir()
    (root / "modules").mkdir()
    (root / "source").mkdir()
    (root / "environment.txt").write_text(
        "repository=/repo\ncommit=abc\nbranch=main\nsource_date_epoch=1\nstatus:\n",
        encoding="utf-8",
    )
    for name in ("rust.files", "python.files", "verilog.files", "cpp.files"):
        (root / "static" / name).write_text("example\n", encoding="utf-8")
    for status_path in capture.REQUIRED_STATUS_PATHS:
        status = root / status_path
        status.parent.mkdir(parents=True, exist_ok=True)
        metadata = "available=0\n" if status_path == "modules/cargo-modules-version.status" else ""
        status.write_text(f"exit=0\n{metadata}", encoding="utf-8")
    for surface in ("cflow", "cscope", "mir", "python", "runtime"):
        (root / surface / "focused.txt").write_text("evidence\n", encoding="utf-8")
    source_input = root / "source-input.rs"
    source_input.write_text("fn source_input() {}\n", encoding="utf-8")
    source_digest = hashlib.sha256(source_input.read_bytes()).hexdigest()
    (root / "source" / "inputs.files").write_text("source-input.rs\n", encoding="utf-8")
    (root / "source" / "inputs.sha256").write_text(
        f"{source_digest}\tsource-input.rs\n", encoding="utf-8"
    )
    with tarfile.open(root / "source" / "inputs.tar", mode="w") as archive:
        archive.add(source_input, arcname="source-input.rs")
    (root / "source" / "tracked-working-tree.diff").write_text("", encoding="utf-8")


def test_written_manifest_verifies_complete_capture(tmp_path: Path) -> None:
    _make_capture(tmp_path)
    manifest = capture.build_manifest(tmp_path)
    (tmp_path / capture.MANIFEST_NAME).write_text(json.dumps(manifest), encoding="utf-8")

    assert capture.validate_manifest(tmp_path, manifest, require_success=True) == []


def test_manifest_detects_post_capture_artifact_mutation(tmp_path: Path) -> None:
    _make_capture(tmp_path)
    manifest = capture.build_manifest(tmp_path)
    (tmp_path / "runtime" / "focused.txt").write_text("mutated\n", encoding="utf-8")

    errors = capture.validate_manifest(tmp_path, manifest, require_success=True)
    assert any("hash changed" in error for error in errors)


def test_manifest_requires_success_when_requested(tmp_path: Path) -> None:
    _make_capture(tmp_path)
    (tmp_path / "mir" / "focused.status").write_text("exit=1\n", encoding="utf-8")
    manifest = capture.build_manifest(tmp_path)

    assert capture.validate_manifest(tmp_path, manifest, require_success=False) == []
    errors = capture.validate_manifest(tmp_path, manifest, require_success=True)
    assert any("status failed" in error for error in errors)


def test_manifest_rejects_tool_output_misclassified_as_dirty_status(tmp_path: Path) -> None:
    _make_capture(tmp_path)
    manifest = capture.build_manifest(tmp_path)
    source = manifest["source"]
    assert isinstance(source, dict)
    source["dirty"] = ["tool_cflow_version=cflow 1.8"]

    errors = capture.validate_manifest(tmp_path, manifest, require_success=True)
    assert any("non-git-status" in error for error in errors)


def test_manifest_requires_each_versioned_probe_status(tmp_path: Path) -> None:
    _make_capture(tmp_path)
    (tmp_path / "runtime" / "virtual-fpga-system.status").unlink()
    manifest = capture.build_manifest(tmp_path)

    errors = capture.validate_manifest(tmp_path, manifest, require_success=True)
    assert any("runtime/virtual-fpga-system.status" in error for error in errors)


def test_manifest_requires_cargo_modules_probes_when_the_tool_is_available(tmp_path: Path) -> None:
    _make_capture(tmp_path)
    (tmp_path / "modules" / "cargo-modules-version.status").write_text(
        "exit=0\navailable=1\n", encoding="utf-8"
    )
    manifest = capture.build_manifest(tmp_path)

    errors = capture.validate_manifest(tmp_path, manifest, require_success=True)
    assert any("modules/system-dependencies.status" in error for error in errors)


def test_manifest_rejects_source_archive_with_content_different_from_the_hash_manifest(tmp_path: Path) -> None:
    _make_capture(tmp_path)
    replacement = tmp_path / "replacement.rs"
    replacement.write_text("fn replacement() {}\n", encoding="utf-8")
    with tarfile.open(tmp_path / "source" / "inputs.tar", mode="w") as archive:
        archive.add(replacement, arcname="source-input.rs")
    manifest = capture.build_manifest(tmp_path)

    errors = capture.validate_manifest(tmp_path, manifest, require_success=True)
    assert any("source snapshot archive hash mismatch" in error for error in errors)
