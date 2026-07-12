#!/usr/bin/env python3
"""Validate the reviewed Gowin board-clock contract before FPGA programming."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from decimal import Decimal, InvalidOperation, getcontext
from pathlib import Path
from typing import Any

SCHEMA_VERSION = 1
SHA256_PATTERN = re.compile(r"^[0-9a-f]{64}$")
IDENTIFIER_PATTERN = re.compile(r"^[A-Za-z0-9_]+$")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def require_mapping(value: Any, name: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{name} must be a JSON object")
    return value


def require_string(mapping: dict[str, Any], name: str) -> str:
    value = mapping.get(name)
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{name} must be a non-empty string")
    return value


def require_sha256(mapping: dict[str, Any], name: str) -> str:
    value = require_string(mapping, name).lower()
    if not SHA256_PATTERN.fullmatch(value):
        raise ValueError(f"{name} must be a lowercase SHA-256")
    return value


def require_positive_integer(mapping: dict[str, Any], name: str) -> int:
    value = mapping.get(name)
    if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
        raise ValueError(f"{name} must be a positive integer")
    return value


def require_percentage(mapping: dict[str, Any], name: str) -> Decimal:
    value = mapping.get(name)
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"{name} must be numeric")
    try:
        decimal_value = Decimal(str(value))
    except InvalidOperation as error:
        raise ValueError(f"{name} must be numeric") from error
    if decimal_value <= 0 or decimal_value >= 100:
        raise ValueError(f"{name} must be between zero and one hundred")
    return decimal_value


def read_json(path: Path) -> tuple[dict[str, Any], bytes]:
    try:
        raw = path.read_bytes()
        value = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read clock evidence {path}: {error}") from error
    return require_mapping(value, "clock evidence"), raw


def normalize_source_path(path: str) -> str:
    candidate = Path(path)
    if (
        not path
        or "\x00" in path
        or candidate.is_absolute()
        or any(part in {"", ".", ".."} for part in candidate.parts)
    ):
        raise ValueError(f"source path is unsafe: {path!r}")
    return path


def validate_evidence(
    evidence: dict[str, Any], expected_device: str, expected_top_module: str
) -> dict[str, Any]:
    if evidence.get("schema_version") != SCHEMA_VERSION:
        raise ValueError("clock evidence has an unsupported schema_version")
    target = require_mapping(evidence.get("target"), "target")
    if require_string(target, "device") != expected_device:
        raise ValueError("clock evidence target device does not match the Makefile target")
    if require_string(target, "top_module") != expected_top_module:
        raise ValueError("clock evidence top_module does not match the Makefile target")
    require_string(target, "board")
    require_string(target, "revision")

    review = require_mapping(evidence.get("review"), "review")
    require_string(review, "record_id")
    require_string(review, "reviewer")
    reviewed_at = require_string(review, "reviewed_at")
    if not re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}", reviewed_at):
        raise ValueError("reviewed_at must use YYYY-MM-DD")

    clock = require_mapping(evidence.get("clock"), "clock")
    if require_string(clock, "port") != "sys_clk_in":
        raise ValueError("clock evidence port must be sys_clk_in")
    for name in ("io_loc", "io_type"):
        if not IDENTIFIER_PATTERN.fullmatch(require_string(clock, name)):
            raise ValueError(f"clock {name} contains unsupported characters")
    frequency_hz = require_positive_integer(clock, "frequency_hz")
    measured_frequency_hz = require_positive_integer(clock, "measured_frequency_hz")
    timing_frequency_hz = require_positive_integer(clock, "timing_frequency_hz")
    if timing_frequency_hz < frequency_hz or timing_frequency_hz < measured_frequency_hz:
        raise ValueError("timing_frequency_hz must be at least nominal and measured frequency")
    require_percentage(clock, "duty_cycle_percent")
    require_string(clock, "measurement_method")

    artifacts = require_mapping(evidence.get("artifacts"), "artifacts")
    require_sha256(artifacts, "constraints_sha256")

    sources = evidence.get("source_files")
    if not isinstance(sources, list) or not sources:
        raise ValueError("source_files must be a non-empty list")
    source_hashes: dict[str, str] = {}
    for entry in sources:
        source = require_mapping(entry, "source_files entry")
        source_path = normalize_source_path(require_string(source, "path"))
        if source_path in source_hashes:
            raise ValueError(f"source_files repeats {source_path}")
        source_hashes[source_path] = require_sha256(source, "sha256")
    return {
        "target": target,
        "clock": clock,
        "artifacts": artifacts,
        "source_hashes": source_hashes,
    }


def parse_constraints(path: Path, clock: dict[str, Any]) -> None:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as error:
        raise ValueError(f"cannot read constraints {path}: {error}") from error
    text = re.sub(r"//.*$|#.*$", "", text, flags=re.MULTILINE)
    locations = re.findall(r'IO_LOC\s+"sys_clk_in"\s+([^;\s]+)\s*;', text)
    ports = re.findall(r'IO_PORT\s+"sys_clk_in"\s+([^;]+);', text)
    if not locations:
        raise ValueError("constraints do not assign sys_clk_in")
    if len(locations) != 1:
        raise ValueError("constraints assign sys_clk_in more than once")
    if locations[0] != clock["io_loc"]:
        raise ValueError("constraints sys_clk_in IO_LOC does not match reviewed evidence")
    if not ports:
        raise ValueError("constraints do not declare sys_clk_in electrical standard")
    if len(ports) != 1:
        raise ValueError("constraints declare sys_clk_in electrical standard more than once")
    io_type = re.search(r"\bIO_TYPE=([A-Za-z0-9_]+)", ports[0])
    if io_type is None:
        raise ValueError("constraints sys_clk_in declaration lacks IO_TYPE")
    if io_type.group(1) != clock["io_type"]:
        raise ValueError("constraints sys_clk_in IO_TYPE does not match reviewed evidence")


def validate_sources(
    source_arguments: list[str], source_hashes: dict[str, str], require_complete: bool
) -> list[dict[str, str]]:
    if not source_arguments:
        raise ValueError("at least one --source path is required")
    observed: dict[str, str] = {}
    for source_argument in source_arguments:
        source_path = normalize_source_path(source_argument)
        candidate = Path(source_path)
        if not candidate.is_file():
            raise ValueError(f"source file does not exist: {source_path}")
        if source_path in observed:
            raise ValueError(f"--source repeats {source_path}")
        observed[source_path] = sha256_file(candidate)
        if source_hashes.get(source_path) != observed[source_path]:
            raise ValueError(f"source hash does not match reviewed evidence: {source_path}")
    if require_complete and set(observed) != set(source_hashes):
        raise ValueError("reviewed source list does not exactly match the deployment source list")
    return [
        {"path": source_path, "sha256": observed[source_path]}
        for source_path in sorted(observed)
    ]


def decimal_text(value: Decimal) -> str:
    return format(value.quantize(Decimal("0.000000001")), "f").rstrip("0").rstrip(".")


def render_timing_constraints(clock: dict[str, Any]) -> bytes:
    getcontext().prec = 40
    period_ns = Decimal(1_000_000_000) / Decimal(clock["timing_frequency_hz"])
    high_ns = period_ns * require_percentage(clock, "duty_cycle_percent") / Decimal(100)
    return (
        "# Generated from the reviewed Gowin board-clock contract.\n"
        f"create_clock -name sys_clk_in -period {decimal_text(period_ns)} "
        f"-waveform {{0 {decimal_text(high_ns)}}} [get_ports {{sys_clk_in}}]\n"
    ).encode("ascii")


def write_atomic(path: Path, value: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.partial")
    if temporary.exists():
        raise ValueError(f"temporary output already exists: {temporary}")
    try:
        with temporary.open("xb") as handle:
            handle.write(value)
            handle.flush()
        temporary.replace(path)
    except OSError as error:
        temporary.unlink(missing_ok=True)
        raise ValueError(f"cannot write {path}: {error}") from error


def validate_timing_constraints(path: Path, clock: dict[str, Any]) -> str:
    expected = render_timing_constraints(clock)
    try:
        actual = path.read_bytes()
    except OSError as error:
        raise ValueError(f"cannot read generated timing constraints {path}: {error}") from error
    if actual != expected:
        raise ValueError("generated timing constraints do not exactly match reviewed clock evidence")
    return sha256_bytes(actual)


def contract_payload(
    evidence_bytes: bytes,
    constraints_path: Path,
    timing_constraints_path: Path,
    source_records: list[dict[str, str]],
    clock: dict[str, Any],
) -> dict[str, Any]:
    return {
        "schema_version": SCHEMA_VERSION,
        "evidence_sha256": sha256_bytes(evidence_bytes),
        "constraints_sha256": sha256_file(constraints_path),
        "timing_constraints_sha256": sha256_file(timing_constraints_path),
        "clock": {
            "port": clock["port"],
            "io_loc": clock["io_loc"],
            "io_type": clock["io_type"],
            "timing_frequency_hz": clock["timing_frequency_hz"],
        },
        "source_files": source_records,
    }


def read_contract(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read clock contract {path}: {error}") from error
    return require_mapping(value, "clock contract")


def validate_contract(path: Path, expected: dict[str, Any]) -> None:
    actual = read_contract(path)
    if actual != expected:
        raise ValueError("clock contract does not match the reviewed source and clock inputs")


def validate_timing_report(
    report_path: Path,
    timing_paths_path: Path,
    timing_constraints_path: Path,
    clock: dict[str, Any],
) -> None:
    try:
        report = report_path.read_text(encoding="utf-8", errors="strict")
        timing_paths = timing_paths_path.read_text(encoding="utf-8", errors="strict")
    except OSError as error:
        raise ValueError(f"cannot read synthesis timing output: {error}") from error
    timing_file = re.search(r"<Timing Constraints File>:\s*(.+?)\s*$", report, flags=re.MULTILINE)
    if timing_file is None or timing_file.group(1).strip() in {"", "---"}:
        raise ValueError("synthesis report does not identify active timing constraints")
    if Path(timing_file.group(1).strip()).name != timing_constraints_path.name:
        raise ValueError("synthesis report used a different timing-constraints file")
    port_row = None
    for line in report.splitlines():
        columns = [column.strip() for column in line.split("|")]
        if columns and columns[0] == "sys_clk_in":
            port_row = columns
            break
    if port_row is None or len(port_row) < 8:
        raise ValueError("synthesis report does not contain a sys_clk_in pinout row")
    if port_row[2].split("/", maxsplit=1)[0] != clock["io_loc"]:
        raise ValueError("synthesis report sys_clk_in location does not match reviewed evidence")
    if port_row[7] != clock["io_type"]:
        raise ValueError("synthesis report sys_clk_in IO_TYPE does not match reviewed evidence")
    if not re.search(r"^=====\s*$\nSETUP\s*$", timing_paths, flags=re.MULTILINE):
        raise ValueError("timing-path output does not contain a setup analysis section")
    if re.search(r"\b(timing\s+(violation|fail)|failed\s+timing)\b", report, flags=re.IGNORECASE):
        raise ValueError("synthesis report records a timing failure")


def build_evidence_payload(
    contract_path: Path,
    report_path: Path,
    timing_paths_path: Path,
    bitstream_path: Path,
) -> dict[str, Any]:
    return {
        "schema_version": SCHEMA_VERSION,
        "clock_contract_sha256": sha256_file(contract_path),
        "synthesis_report_sha256": sha256_file(report_path),
        "timing_paths_sha256": sha256_file(timing_paths_path),
        "bitstream_sha256": sha256_file(bitstream_path),
    }


def validate_build_evidence(path: Path, expected: dict[str, Any]) -> None:
    try:
        actual = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read build evidence {path}: {error}") from error
    if actual != expected:
        raise ValueError("build evidence does not match the programmed artifacts")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stage", choices=("preflight", "source", "programming"), required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--constraints", type=Path, required=True)
    parser.add_argument("--expected-device", required=True)
    parser.add_argument("--expected-top-module", required=True)
    parser.add_argument("--source", action="append", default=[])
    parser.add_argument("--timing-constraints", type=Path, required=True)
    parser.add_argument("--contract", type=Path)
    parser.add_argument("--synthesis-report", type=Path)
    parser.add_argument("--timing-paths", type=Path)
    parser.add_argument("--bitstream", type=Path)
    parser.add_argument("--build-evidence", type=Path)
    arguments = parser.parse_args()

    try:
        evidence, evidence_bytes = read_json(arguments.evidence)
        validated = validate_evidence(
            evidence, arguments.expected_device, arguments.expected_top_module
        )
        clock = validated["clock"]
        constraints_digest = sha256_file(arguments.constraints)
        if constraints_digest != validated["artifacts"]["constraints_sha256"]:
            raise ValueError("constraints SHA-256 does not match reviewed evidence")
        parse_constraints(arguments.constraints, clock)

        if arguments.stage == "preflight":
            validate_sources(arguments.source, validated["source_hashes"], False)
            write_atomic(arguments.timing_constraints, render_timing_constraints(clock))
        elif arguments.stage == "source":
            if arguments.contract is None:
                raise ValueError("--contract is required for the source stage")
            source_records = validate_sources(arguments.source, validated["source_hashes"], True)
            timing_digest = validate_timing_constraints(arguments.timing_constraints, clock)
            expected_contract = contract_payload(
                evidence_bytes,
                arguments.constraints,
                arguments.timing_constraints,
                source_records,
                clock,
            )
            if expected_contract["timing_constraints_sha256"] != timing_digest:
                raise ValueError("generated timing constraints digest is inconsistent")
            write_atomic(
                arguments.contract,
                (json.dumps(expected_contract, indent=2, sort_keys=True) + "\n").encode("utf-8"),
            )
        else:
            required_paths = {
                "--contract": arguments.contract,
                "--synthesis-report": arguments.synthesis_report,
                "--timing-paths": arguments.timing_paths,
                "--bitstream": arguments.bitstream,
                "--build-evidence": arguments.build_evidence,
            }
            missing = [name for name, path in required_paths.items() if path is None]
            if missing:
                raise ValueError(f"programming stage requires {', '.join(missing)}")
            source_records = validate_sources(arguments.source, validated["source_hashes"], True)
            validate_timing_constraints(arguments.timing_constraints, clock)
            expected_contract = contract_payload(
                evidence_bytes,
                arguments.constraints,
                arguments.timing_constraints,
                source_records,
                clock,
            )
            validate_contract(arguments.contract, expected_contract)
            if not arguments.bitstream.is_file() or arguments.bitstream.stat().st_size == 0:
                raise ValueError("bitstream does not exist or is empty")
            validate_timing_report(
                arguments.synthesis_report,
                arguments.timing_paths,
                arguments.timing_constraints,
                clock,
            )
            generated_build_evidence = build_evidence_payload(
                arguments.contract,
                arguments.synthesis_report,
                arguments.timing_paths,
                arguments.bitstream,
            )
            write_atomic(
                arguments.build_evidence,
                (json.dumps(generated_build_evidence, indent=2, sort_keys=True) + "\n").encode("utf-8"),
            )
            validate_build_evidence(arguments.build_evidence, generated_build_evidence)
        print(f"Validated Gowin clock contract stage: {arguments.stage}")
        return 0
    except (OSError, ValueError) as error:
        print(f"verify_gowin_clock_contract: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
