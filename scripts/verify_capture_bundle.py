#!/usr/bin/env python3
"""Write and verify a complete, checksummed call-graph capture manifest."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import tarfile
from collections.abc import Iterable
from pathlib import Path, PurePosixPath

SCHEMA_VERSION = 1
CAPTURE_PROFILE_VERSION = 2
MANIFEST_NAME = "capture_manifest.json"
REQUIRED_FILES = {
    "environment.txt",
    "source/inputs.files",
    "source/inputs.sha256",
    "source/inputs.tar",
    "source/tracked-working-tree.diff",
    "static/rust.files",
    "static/python.files",
    "static/verilog.files",
    "static/cpp.files",
}
REQUIRED_STATUS_SURFACES = {"cflow", "cscope", "mir", "python", "runtime"}
EXCLUDED_DIRECTORIES = {"cargo-target", "cpp-build", "__pycache__"}
REQUIRED_STATUS_PATHS = frozenset(
    {
        "cflow/fixture-runner.status",
        "cflow/gui-fixture.status",
        "cflow/solver-datasheet-timing.status",
        "cflow/fpga-export.status",
        "cflow/fpga-export-cli.status",
        "cflow/i4003-behavior.status",
        "cflow/i4003-export.status",
        "cflow/phase-trace.status",
        "cflow/trace-replay.status",
        "cflow/common-stimulus.status",
        "cflow/intellec-machine.status",
        "cflow/virtual-fpga-system.status",
        "cflow/gate-export-python.status",
        "cflow/netlist-publish-python.status",
        "cscope/build.status",
        "cscope/cpp.status",
        "cscope/gate-export-python.status",
        "mir/fixture-runner.status",
        "mir/fixture-runner-extract.status",
        "mir/phase-trace.status",
        "mir/phase-trace-extract.status",
        "mir/common-stimulus.status",
        "mir/common-stimulus-extract.status",
        "mir/gui-fixture.status",
        "mir/gui-fixture-extract.status",
        "mir/fpga-export-cli.status",
        "mir/fpga-export-cli-extract.status",
        "mir/system-library.status",
        "mir/system-library-extract.status",
        "mir/chips-library.status",
        "mir/chips-library-extract.status",
        "mir/core-library.status",
        "mir/core-library-extract.status",
        "mir/fpga-library.status",
        "mir/fpga-library-extract.status",
        "mir/intellec-library.status",
        "mir/intellec-library-extract.status",
        "mir/i4003-behavior-calls.status",
        "mir/i4003-export-calls.status",
        "mir/trace-replay-calls.status",
        "mir/intellec-machine-calls.status",
        "modules/cargo-modules-version.status",
        "python/gate_to_verilog_v0-callgraph.status",
        "python/build_netlist_v1_v0-callgraph.status",
        "python/common-stimulus-comparison-callgraph.status",
        "runtime/build.status",
        "runtime/virtual-fpga-build.status",
        "runtime/mcs4-fixture.status",
        "runtime/mcs40-fixture.status",
        "runtime/fixture-runner.status",
        "runtime/phase-trace.status",
        "runtime/trace-frame-capture.status",
        "runtime/common-stimulus.status",
        "runtime/fpga-export-cli.status",
        "runtime/netlist-v1-build.status",
        "runtime/solver-datasheet-test.status",
        "runtime/mcs40-integration-test.status",
        "runtime/fpga-export-test.status",
        "runtime/i4003-behavior-test.status",
        "runtime/i4003-system-wiring-test.status",
        "runtime/i4003-fpga-export-test.status",
        "runtime/trace-replay-cli-test.status",
        "runtime/trace-frame-comparison-test.status",
        "runtime/intellec-source-gate-test.status",
        "runtime/intellec-replay-test.status",
        "runtime/virtual-fpga-system.status",
        "runtime/virtual-fpga-common-stimulus.status",
        "runtime/mcs4-fixture.callgrind.status",
        "runtime/mcs4-fixture.callgrind-annotate.status",
        "runtime/mcs4-fixture.gprof2dot.status",
    }
)
REQUIRED_CARGO_MODULES_STATUS_PATHS = frozenset(
    {
        "modules/system-dependencies.status",
        "modules/core-dependencies.status",
        "modules/chips-structure.status",
    }
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def relative_path(capture_dir: Path, path: Path) -> str:
    try:
        return path.resolve().relative_to(capture_dir.resolve()).as_posix()
    except ValueError as error:
        raise ValueError(f"artifact escapes capture directory: {path}") from error


def capture_files(capture_dir: Path) -> list[Path]:
    files: list[Path] = []
    for path in capture_dir.rglob("*"):
        if not path.is_file():
            continue
        relative = path.relative_to(capture_dir)
        if relative.name == MANIFEST_NAME or any(
            part in EXCLUDED_DIRECTORIES for part in relative.parts
        ):
            continue
        files.append(path)
    return sorted(files, key=lambda path: relative_path(capture_dir, path))


def parse_environment(path: Path) -> dict[str, object]:
    values: dict[str, object] = {"dirty": []}
    in_status = False
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        if raw_line == "status:":
            in_status = True
            continue
        if in_status:
            if raw_line:
                dirty = values["dirty"]
                if not isinstance(dirty, list):
                    raise ValueError("capture environment dirty field is invalid")
                dirty.append(raw_line)
            continue
        if "=" in raw_line:
            key, value = raw_line.split("=", 1)
            values[key] = value
    for key in ("repository", "commit", "branch", "source_date_epoch"):
        if not isinstance(values.get(key), str) or not str(values[key]).strip():
            raise ValueError(f"capture environment lacks {key}")
    return values


def parse_status(path: Path) -> dict[str, object]:
    values: dict[str, object] = {}
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        if "=" not in raw_line:
            continue
        key, value = raw_line.split("=", 1)
        values[key] = value
    exit_value = values.get("exit")
    if not isinstance(exit_value, str) or not exit_value.isdigit():
        raise ValueError(f"status file lacks numeric exit code: {path}")
    return {"exit": int(exit_value), "metadata": values}


def validate_relative_source_path(value: str) -> str:
    candidate = PurePosixPath(value)
    if (
        not value
        or "\\" in value
        or "\x00" in value
        or candidate.is_absolute()
        or candidate.as_posix() != value
        or any(part in {"", ".", ".."} for part in candidate.parts)
    ):
        raise ValueError(f"source snapshot contains an unsafe path: {value!r}")
    return value


def parse_source_list(path: Path) -> list[str]:
    entries = [validate_relative_source_path(line) for line in path.read_text(encoding="utf-8").splitlines()]
    if not entries:
        raise ValueError("source snapshot input list is empty")
    if entries != sorted(entries):
        raise ValueError("source snapshot input list is not sorted")
    if len(entries) != len(set(entries)):
        raise ValueError("source snapshot input list contains duplicate paths")
    return entries


def parse_source_hashes(path: Path) -> dict[str, str]:
    hashes: dict[str, str] = {}
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        digest, separator, source_path = line.partition("\t")
        if not separator:
            raise ValueError(f"source snapshot hash line {line_number} lacks a tab separator")
        if len(digest) != 64 or not all(character in "0123456789abcdef" for character in digest.lower()):
            raise ValueError(f"source snapshot hash line {line_number} has an invalid SHA-256")
        source_path = validate_relative_source_path(source_path)
        if source_path in hashes:
            raise ValueError(f"source snapshot hash list repeats {source_path}")
        hashes[source_path] = digest.lower()
    return hashes


def validate_source_snapshot(capture_dir: Path) -> list[str]:
    errors: list[str] = []
    source_root = capture_dir / "source"
    try:
        source_paths = parse_source_list(source_root / "inputs.files")
        source_hashes = parse_source_hashes(source_root / "inputs.sha256")
        if set(source_paths) != set(source_hashes):
            errors.append("source snapshot input and hash manifests name different paths")
            return errors
        archive_paths: set[str] = set()
        with tarfile.open(source_root / "inputs.tar", mode="r:") as archive:
            for member in archive.getmembers():
                member_path = validate_relative_source_path(member.name)
                if not member.isfile():
                    errors.append(f"source snapshot archive member is not a regular file: {member_path}")
                    continue
                if member_path in archive_paths:
                    errors.append(f"source snapshot archive repeats {member_path}")
                    continue
                archive_paths.add(member_path)
                content = archive.extractfile(member)
                if content is None:
                    errors.append(f"source snapshot archive cannot read {member_path}")
                    continue
                digest = hashlib.sha256(content.read()).hexdigest()
                if digest != source_hashes.get(member_path):
                    errors.append(f"source snapshot archive hash mismatch: {member_path}")
        if archive_paths != set(source_paths):
            errors.append("source snapshot archive and input manifest name different paths")
    except (OSError, ValueError, tarfile.TarError, UnicodeDecodeError) as error:
        errors.append(f"source snapshot is invalid: {error}")
    return errors


def build_manifest(capture_dir: Path) -> dict[str, object]:
    environment_path = capture_dir / "environment.txt"
    if not environment_path.is_file():
        raise ValueError(f"capture lacks {environment_path}")

    artifacts = [
        {
            "path": relative_path(capture_dir, path),
            "sha256": sha256(path),
            "bytes": path.stat().st_size,
        }
        for path in capture_files(capture_dir)
    ]
    statuses = []
    for path in capture_dir.rglob("*.status"):
        if any(part in EXCLUDED_DIRECTORIES for part in path.relative_to(capture_dir).parts):
            continue
        statuses.append({"path": relative_path(capture_dir, path), **parse_status(path)})
    statuses.sort(key=lambda status: str(status["path"]))
    return {
        "schema_version": SCHEMA_VERSION,
        "capture_profile_version": CAPTURE_PROFILE_VERSION,
        "capture_root": capture_dir.name,
        "source": parse_environment(environment_path),
        "artifacts": artifacts,
        "statuses": statuses,
        "allowed_nonzero_statuses": [],
    }


def validate_manifest(
    capture_dir: Path, manifest: dict[str, object], require_success: bool
) -> list[str]:
    errors: list[str] = []
    if manifest.get("schema_version") != SCHEMA_VERSION:
        errors.append("capture manifest has an unsupported schema_version")
    if manifest.get("capture_profile_version") != CAPTURE_PROFILE_VERSION:
        errors.append("capture manifest has an unsupported capture_profile_version")
    if manifest.get("capture_root") != capture_dir.name:
        errors.append("capture manifest capture_root does not match the directory name")

    source = manifest.get("source")
    if not isinstance(source, dict):
        errors.append("capture manifest lacks source identity")
    else:
        for key in ("repository", "commit", "branch", "source_date_epoch", "dirty"):
            if key not in source:
                errors.append(f"capture manifest source lacks {key}")
        dirty_entries = source.get("dirty")
        if not isinstance(dirty_entries, list):
            errors.append("capture manifest source dirty field must be a list")
        elif any(
            not isinstance(entry, str) or len(entry) < 3 or entry[2] != " "
            for entry in dirty_entries
        ):
            errors.append("capture manifest source dirty field contains a non-git-status entry")

    artifacts = manifest.get("artifacts")
    if not isinstance(artifacts, list):
        return errors + ["capture manifest artifacts must be a list"]
    declared: dict[str, dict[str, object]] = {}
    for artifact in artifacts:
        if not isinstance(artifact, dict):
            errors.append("capture manifest contains a non-object artifact")
            continue
        path = artifact.get("path")
        digest = artifact.get("sha256")
        if not isinstance(path, str) or not isinstance(digest, str):
            errors.append("capture manifest artifact lacks path or sha256")
            continue
        if path in declared:
            errors.append(f"capture manifest repeats artifact {path}")
            continue
        declared[path] = artifact
        candidate = capture_dir / path
        try:
            relative_path(capture_dir, candidate)
        except ValueError as error:
            errors.append(str(error))
            continue
        if not candidate.is_file():
            errors.append(f"capture artifact is missing: {path}")
            continue
        if sha256(candidate) != digest:
            errors.append(f"capture artifact hash changed: {path}")
        if artifact.get("bytes") != candidate.stat().st_size:
            errors.append(f"capture artifact size changed: {path}")

    actual = {relative_path(capture_dir, path) for path in capture_files(capture_dir)}
    missing = sorted(actual - set(declared))
    stale = sorted(set(declared) - actual)
    if missing:
        errors.append(f"capture manifest omits artifacts: {', '.join(missing)}")
    if stale:
        errors.append(f"capture manifest lists absent artifacts: {', '.join(stale)}")
    for required in sorted(REQUIRED_FILES):
        if required not in declared:
            errors.append(f"capture manifest lacks required artifact: {required}")
    errors.extend(validate_source_snapshot(capture_dir))

    statuses = manifest.get("statuses")
    if not isinstance(statuses, list):
        return errors + ["capture manifest statuses must be a list"]
    allowed_nonzero = manifest.get("allowed_nonzero_statuses")
    if not isinstance(allowed_nonzero, list) or not all(
        isinstance(path, str) for path in allowed_nonzero
    ):
        errors.append("capture manifest allowed_nonzero_statuses must be a string list")
        allowed_nonzero = []
    status_paths: set[str] = set()
    surfaces: set[str] = set()
    for status in statuses:
        if not isinstance(status, dict):
            errors.append("capture manifest contains a non-object status")
            continue
        path = status.get("path")
        exit_code = status.get("exit")
        if not isinstance(path, str) or not isinstance(exit_code, int):
            errors.append("capture manifest status lacks path or exit")
            continue
        status_paths.add(path)
        surface = path.split("/", 1)[0]
        surfaces.add(surface)
        if path not in declared:
            errors.append(f"capture status is not an artifact: {path}")
        on_disk = capture_dir / path
        if on_disk.is_file():
            try:
                observed = parse_status(on_disk)
            except ValueError as error:
                errors.append(str(error))
                continue
            if observed["exit"] != exit_code:
                errors.append(f"capture status exit changed: {path}")
        else:
            errors.append(f"capture status is missing: {path}")
        if require_success and exit_code != 0 and path not in allowed_nonzero:
            errors.append(f"capture status failed: {path} exit={exit_code}")
    for surface in sorted(REQUIRED_STATUS_SURFACES - surfaces):
        errors.append(f"capture manifest lacks a status for surface: {surface}")
    for required_status in sorted(REQUIRED_STATUS_PATHS - status_paths):
        errors.append(f"capture manifest lacks required probe status: {required_status}")
    cargo_modules_status = next(
        (status for status in statuses if isinstance(status, dict) and status.get("path") == "modules/cargo-modules-version.status"),
        None,
    )
    if isinstance(cargo_modules_status, dict):
        metadata = cargo_modules_status.get("metadata")
        available = metadata.get("available") if isinstance(metadata, dict) else None
        if available not in {"0", "1"}:
            errors.append("cargo-modules status must declare available=0 or available=1")
        elif available == "1":
            for required_status in sorted(REQUIRED_CARGO_MODULES_STATUS_PATHS - status_paths):
                errors.append(f"capture manifest lacks required cargo-modules status: {required_status}")
    return errors


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture-dir", type=Path, required=True)
    parser.add_argument("--write", action="store_true", help="write capture_manifest.json")
    parser.add_argument("--require-success", action="store_true")
    arguments = parser.parse_args(argv)
    capture_dir = arguments.capture_dir.resolve()
    manifest_path = capture_dir / MANIFEST_NAME

    try:
        if arguments.write:
            capture_dir.mkdir(parents=True, exist_ok=True)
            manifest_path.write_text(
                json.dumps(build_manifest(capture_dir), indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        if not isinstance(manifest, dict):
            raise ValueError("capture manifest root must be an object")
        errors = validate_manifest(capture_dir, manifest, arguments.require_success)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"verify_capture_bundle: {error}", file=sys.stderr)
        return 1

    if errors:
        for error in errors:
            print(f"verify_capture_bundle: {error}", file=sys.stderr)
        return 1
    print(f"Validated capture manifest: {manifest_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
