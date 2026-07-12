#!/usr/bin/env python3
"""Build a clean-tree developer proof bundle for the virtual i4003 board."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
import shlex
import shutil
import subprocess
import sys
from collections.abc import Iterable, Sequence
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BUNDLE_ROOT = ROOT / "target" / "developer-bundle"
REQUIRED_ARTIFACTS = (
    Path("source.tar.gz"),
    Path("virtual-fpga/generated/i4003_fpga.v"),
    Path("virtual-fpga/generated/i4003_fpga.v.manifest.json"),
    Path("virtual-fpga/test-artifacts/i4003-summary.json"),
    Path("virtual-fpga/test-artifacts/i4003-shift-gate.vcd"),
    Path("validation/just-verify.log"),
    Path("validation/cmake-configure.log"),
    Path("validation/cmake-build.log"),
    Path("validation/ctest.log"),
    Path("validation/export.log"),
)


class BundleError(RuntimeError):
    """A bundle precondition or evidence contract failed."""


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def command_output(command: Sequence[str], *, cwd: Path = ROOT) -> str:
    result = subprocess.run(  # noqa: S603 - fixed local developer commands
        list(command),
        check=True,
        capture_output=True,
        cwd=cwd,
        text=True,
    )
    return result.stdout.strip()


def required_executable(name: str) -> str:
    path = shutil.which(name)
    if path is None:
        raise BundleError(f"required executable is unavailable: {name}")
    return path


def require_clean_revision() -> tuple[str, str]:
    status = command_output(("git", "status", "--porcelain=v1"))
    if status:
        raise BundleError("developer bundle requires a clean working tree")
    revision = command_output(("git", "rev-parse", "HEAD"))
    if len(revision) != 40 or any(character not in "0123456789abcdef" for character in revision):
        raise BundleError("git did not return a full lowercase revision")
    source_date_epoch = command_output(("git", "show", "-s", "--format=%ct", revision))
    if not source_date_epoch.isdigit():
        raise BundleError("git did not return a numeric commit timestamp")
    return revision, source_date_epoch


def bundle_directory(value: Path | None, revision: str) -> Path:
    path = BUNDLE_ROOT / revision if value is None else value
    if not path.is_absolute():
        path = ROOT / path
    resolved = path.resolve()
    try:
        resolved.relative_to(BUNDLE_ROOT.resolve())
    except ValueError as error:
        raise BundleError(f"bundle output must remain under {BUNDLE_ROOT}: {path}") from error
    return resolved


def create_source_archive(revision: str, destination: Path) -> None:
    git = required_executable("git")
    result = subprocess.run(  # noqa: S603 - fixed local git invocation
        [git, "archive", "--format=tar", revision],
        check=True,
        capture_output=True,
        cwd=ROOT,
    )
    with destination.open("wb") as output:
        with gzip.GzipFile(filename="", mode="wb", fileobj=output, mtime=0) as compressed:
            compressed.write(result.stdout)


def run_logged(
    command: Sequence[str], *, cwd: Path, environment: dict[str, str], log: Path
) -> None:
    log.parent.mkdir(parents=True, exist_ok=True)
    with log.open("w", encoding="utf-8") as handle:
        handle.write(f"$ {shlex.join(command)}\n")
        handle.flush()
        result = subprocess.run(  # noqa: S603 - command is an internal fixed sequence
            list(command),
            check=False,
            cwd=cwd,
            env=environment,
            stdout=handle,
            stderr=subprocess.STDOUT,
            text=True,
        )
    if result.returncode != 0:
        raise BundleError(f"bundle command failed ({result.returncode}): {shlex.join(command)}")


def validate_export_manifest(manifest_path: Path, verilog_path: Path, revision: str) -> list[str]:
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read exporter manifest: {error}"]
    if not isinstance(manifest, dict):
        return ["exporter manifest root is not an object"]
    errors: list[str] = []
    if manifest.get("schema_version") != 1:
        errors.append("exporter manifest schema_version must be 1")
    if manifest.get("source_revision") != revision:
        errors.append("exporter manifest revision does not match bundle revision")
    if manifest.get("source_dirty") is not False:
        errors.append("exporter manifest must record source_dirty=false")
    request = manifest.get("request")
    if request != {"chip": "i4003", "flavor": "fpga"}:
        errors.append("exporter manifest must describe the i4003 FPGA request")
    if manifest.get("output_sha256") != sha256(verilog_path):
        errors.append("exporter manifest output SHA-256 does not match generated Verilog")
    return errors


def collect_artifacts(bundle: Path) -> list[dict[str, object]]:
    artifacts: list[dict[str, object]] = []
    for relative in REQUIRED_ARTIFACTS:
        path = bundle / relative
        if not path.is_file():
            raise BundleError(f"bundle artifact is missing: {relative}")
        if relative.name.endswith(".vcd") and path.stat().st_size < 128:
            raise BundleError(f"bundle VCD is unexpectedly small: {relative}")
        artifacts.append(
            {
                "path": relative.as_posix(),
                "sha256": sha256(path),
                "bytes": path.stat().st_size,
            }
        )
    return artifacts


def write_checksums(bundle: Path, artifacts: list[dict[str, object]]) -> Path:
    checksums = bundle / "checksums.sha256"
    rows = []
    for artifact in sorted(artifacts, key=lambda item: str(item["path"])):
        rows.append(f"{artifact['sha256']}  {artifact['path']}")
    checksums.write_text("\n".join(rows) + "\n", encoding="utf-8")
    return checksums


def tool_versions() -> dict[str, str]:
    commands = {
        "cargo": ("cargo", "--version"),
        "cmake": ("cmake", "--version"),
        "ctest": ("ctest", "--version"),
        "git": ("git", "--version"),
        "just": ("just", "--version"),
        "verilator": ("verilator", "--version"),
    }
    versions: dict[str, str] = {}
    for name, command in commands.items():
        output = command_output(command)
        versions[name] = output.splitlines()[0] if output else ""
        if not versions[name]:
            raise BundleError(f"{name} did not report a version")
    return versions


def build_bundle(bundle: Path, revision: str, source_date_epoch: str) -> None:
    if bundle.exists():
        raise BundleError(f"bundle destination already exists: {bundle}")
    bundle.mkdir(parents=True)
    (bundle / ".incomplete").write_text("bundle build is in progress\n", encoding="utf-8")

    environment = dict(os.environ)
    environment["SOURCE_DATE_EPOCH"] = source_date_epoch
    source_archive = bundle / "source.tar.gz"
    create_source_archive(revision, source_archive)

    validation = bundle / "validation"
    run_logged(
        ("just", "verify"), cwd=ROOT, environment=environment, log=validation / "just-verify.log"
    )

    work = bundle / ".work"
    run_logged(
        (
            "cmake",
            "-S",
            "tools/virtual-fpga",
            "-B",
            str(work),
            "-G",
            "Ninja",
            "-DCMAKE_BUILD_TYPE=Release",
        ),
        cwd=ROOT,
        environment=environment,
        log=validation / "cmake-configure.log",
    )
    run_logged(
        ("cmake", "--build", str(work)),
        cwd=ROOT,
        environment=environment,
        log=validation / "cmake-build.log",
    )
    run_logged(
        ("ctest", "--test-dir", str(work), "--output-on-failure"),
        cwd=ROOT,
        environment=environment,
        log=validation / "ctest.log",
    )

    generated = bundle / "virtual-fpga" / "generated"
    generated.mkdir(parents=True)
    final_verilog = generated / "i4003_fpga.v"
    final_manifest = generated / "i4003_fpga.v.manifest.json"
    run_logged(
        (
            "cargo",
            "run",
            "--locked",
            "-p",
            "mcs4-fpga",
            "--bin",
            "mcs4-fpga-export",
            "--",
            "--chip",
            "i4003",
            "--flavor",
            "fpga",
            "--output",
            str(final_verilog),
            "--manifest",
            str(final_manifest),
        ),
        cwd=ROOT,
        environment=environment,
        log=validation / "export.log",
    )
    work_verilog = work / "generated" / "i4003_fpga.v"
    if not work_verilog.is_file() or sha256(work_verilog) != sha256(final_verilog):
        raise BundleError(
            "virtual board did not test the same generated Verilog retained by the bundle"
        )
    export_errors = validate_export_manifest(final_manifest, final_verilog, revision)
    if export_errors:
        raise BundleError("; ".join(export_errors))

    test_artifacts = bundle / "virtual-fpga" / "test-artifacts"
    test_artifacts.mkdir(parents=True)
    for name in ("i4003-summary.json", "i4003-shift-gate.vcd"):
        source = work / "test-artifacts" / name
        if not source.is_file():
            raise BundleError(f"virtual board test artifact is missing: {source}")
        shutil.copy2(source, test_artifacts / name)
    shutil.rmtree(work)

    artifacts = collect_artifacts(bundle)
    checksums = write_checksums(bundle, artifacts)
    artifacts.append(
        {
            "path": checksums.relative_to(bundle).as_posix(),
            "sha256": sha256(checksums),
            "bytes": checksums.stat().st_size,
        }
    )
    manifest = {
        "schema_version": 1,
        "kind": "developer-proof-bundle",
        "source_revision": revision,
        "source_date_epoch": int(source_date_epoch),
        "source_dirty": False,
        "tool_versions": tool_versions(),
        "commands": [
            "just verify",
            "cmake -S tools/virtual-fpga -B <bundle>/.work -G Ninja -DCMAKE_BUILD_TYPE=Release",
            "cmake --build <bundle>/.work",
            "ctest --test-dir <bundle>/.work --output-on-failure",
            "cargo run --locked -p mcs4-fpga --bin mcs4-fpga-export -- --chip i4003 --flavor fpga",
        ],
        "artifacts": artifacts,
        "limitations": [
            "This bundle is developer evidence, not a release package or a target bitstream.",
            "It contains no board-programming artifact, host executable, credentials, or hardware conformance claim.",
        ],
    }
    (bundle / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (bundle / ".incomplete").unlink()


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output-dir",
        type=Path,
        help="destination below target/developer-bundle (defaults to the full source revision)",
    )
    arguments = parser.parse_args(argv)
    try:
        revision, source_date_epoch = require_clean_revision()
        build_bundle(bundle_directory(arguments.output_dir, revision), revision, source_date_epoch)
    except (BundleError, OSError, subprocess.CalledProcessError) as error:
        print(f"build_developer_bundle: {error}", file=sys.stderr)
        return 1
    print(f"Built developer proof bundle for {revision}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
