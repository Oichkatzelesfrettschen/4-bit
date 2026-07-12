#!/usr/bin/env python3
"""Verify the canonical netlist v1 artifact manifest and input provenance."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections.abc import Iterable
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MANIFEST = ROOT / "docs" / "evidence" / "netlists_v1" / "manifest.json"
DEFAULT_CHIPS = frozenset({"4001", "4002", "4003", "4004"})
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
CHIP = re.compile(r"[0-9]{4}\Z")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def resolve_repository_path(root: Path, value: str) -> Path:
    relative = Path(value)
    if relative.is_absolute():
        raise ValueError(f"path must be repository-relative: {value}")
    candidate = (root / relative).resolve()
    try:
        candidate.relative_to(root.resolve())
    except ValueError as error:
        raise ValueError(f"path escapes repository: {value}") from error
    return candidate


def verify_input_hashes(root: Path, chip: str, document: dict[str, object]) -> list[str]:
    errors: list[str] = []
    inputs = document.get("inputs")
    if not isinstance(inputs, dict):
        return [f"{chip}: output lacks inputs object"]
    hashes = inputs.get("sha256")
    if not isinstance(hashes, dict):
        return [f"{chip}: output lacks inputs.sha256 object"]
    for name, expected in sorted(hashes.items()):
        if not isinstance(name, str):
            errors.append(f"{chip}: input hash name is not a string")
            continue
        source = inputs.get(name)
        if source is None and expected is None:
            continue
        if not isinstance(source, str) or not isinstance(expected, str):
            errors.append(f"{chip}: input {name} must have paired path and SHA-256")
            continue
        if not SHA256.fullmatch(expected):
            errors.append(f"{chip}: input {name} SHA-256 is malformed")
            continue
        try:
            source_path = resolve_repository_path(root, source)
        except ValueError as error:
            errors.append(f"{chip}: {error}")
            continue
        if not source_path.is_file():
            errors.append(f"{chip}: input {name} is missing: {source}")
        elif sha256(source_path) != expected:
            errors.append(f"{chip}: input {name} SHA-256 does not match: {source}")
    return errors


def verify_manifest(
    document: dict[str, object],
    *,
    root: Path = ROOT,
    expected_chips: frozenset[str] = DEFAULT_CHIPS,
) -> list[str]:
    errors: list[str] = []
    if document.get("schema_version") != 1:
        return ["netlist manifest schema_version must be 1"]
    if document.get("tool") != "scripts/build_netlist_v1_v0.py":
        errors.append("netlist manifest tool must identify build_netlist_v1_v0.py")
    params = document.get("params")
    if not isinstance(params, dict) or not all(
        isinstance(params.get(name), int) and int(params[name]) > 0
        for name in ("max_transistor_bbox_area", "max_transistor_bbox_dim")
    ):
        errors.append("netlist manifest must define positive extraction bounds")
    outputs = document.get("outputs")
    if not isinstance(outputs, list) or not outputs:
        return errors + ["netlist manifest outputs must be a non-empty list"]

    seen_chips: set[str] = set()
    seen_outputs: set[str] = set()
    for entry in outputs:
        if not isinstance(entry, dict):
            errors.append("netlist manifest output entry is not an object")
            continue
        chip = entry.get("chip")
        output = entry.get("output")
        expected_hash = entry.get("sha256")
        if not isinstance(chip, str) or not CHIP.fullmatch(chip):
            errors.append("netlist manifest chip must be a four-digit string")
            continue
        if chip in seen_chips:
            errors.append(f"netlist manifest repeats chip {chip}")
        seen_chips.add(chip)
        if not isinstance(output, str):
            errors.append(f"{chip}: output must be a repository-relative path")
            continue
        expected_output = f"docs/evidence/netlists_v1/{chip}_netlist_v1.json"
        if output != expected_output:
            errors.append(f"{chip}: output must be {expected_output}")
        if output in seen_outputs:
            errors.append(f"netlist manifest repeats output {output}")
        seen_outputs.add(output)
        if not isinstance(expected_hash, str) or not SHA256.fullmatch(expected_hash):
            errors.append(f"{chip}: output SHA-256 is malformed")
            continue
        try:
            output_path = resolve_repository_path(root, output)
        except ValueError as error:
            errors.append(f"{chip}: {error}")
            continue
        if not output_path.is_file():
            errors.append(f"{chip}: output is missing: {output}")
            continue
        if sha256(output_path) != expected_hash:
            errors.append(f"{chip}: output SHA-256 does not match: {output}")
            continue
        try:
            payload = json.loads(output_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            errors.append(f"{chip}: cannot read generated output: {error}")
            continue
        if not isinstance(payload, dict):
            errors.append(f"{chip}: output root is not an object")
            continue
        if payload.get("chip") != chip:
            errors.append(f"{chip}: output chip does not match manifest")
        schema = payload.get("schema")
        if not isinstance(schema, dict) or schema.get("version") != 1:
            errors.append(f"{chip}: output schema.version must be 1")
        errors.extend(verify_input_hashes(root, chip, payload))

    if seen_chips != set(expected_chips):
        missing = sorted(expected_chips - seen_chips)
        unexpected = sorted(seen_chips - expected_chips)
        if missing:
            errors.append(f"netlist manifest omits chips: {', '.join(missing)}")
        if unexpected:
            errors.append(f"netlist manifest has unexpected chips: {', '.join(unexpected)}")
    return errors


def parse_expected_chips(value: str) -> frozenset[str]:
    chips = frozenset(item.strip() for item in value.split(",") if item.strip())
    if not chips or not all(CHIP.fullmatch(chip) for chip in chips):
        raise argparse.ArgumentTypeError(
            "expected chips must be comma-separated four-digit identifiers"
        )
    return chips


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument(
        "--expect-chips",
        type=parse_expected_chips,
        default=DEFAULT_CHIPS,
        help="comma-separated canonical chip identifiers",
    )
    arguments = parser.parse_args(argv)
    try:
        document = json.loads(arguments.manifest.read_text(encoding="utf-8"))
        if not isinstance(document, dict):
            raise ValueError("netlist manifest root must be an object")
        errors = verify_manifest(document, expected_chips=arguments.expect_chips)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"verify_netlist_manifest: {error}", file=sys.stderr)
        return 1
    if errors:
        for error in errors:
            print(f"verify_netlist_manifest: {error}", file=sys.stderr)
        return 1
    print(f"Validated netlist manifest: {arguments.manifest}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
