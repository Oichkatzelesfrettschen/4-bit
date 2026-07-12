#!/usr/bin/env python3
"""Verify the evidence-backed capability registry."""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections.abc import Iterable
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VALID_STATES = {
    "implemented",
    "tested",
    "reproduced",
    "synthesized",
    "hardware-probed",
    "blocked",
}
VALID_EVIDENCE_KINDS = {
    "corpus",
    "design",
    "hardware",
    "manifest",
    "runtime",
    "source",
    "synthesis",
    "test",
}
VALID_OWNERS = {
    "emulator-maintainers",
    "evidence-maintainers",
    "hardware-maintainers",
}
CAPABILITY_ID = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*\Z")


def resolve_path(value: str) -> Path:
    path = (ROOT / value).resolve()
    try:
        path.relative_to(ROOT)
    except ValueError as error:
        raise ValueError(f"evidence path escapes repository: {value}") from error
    return path


def verify(document: dict[str, object]) -> list[str]:
    errors: list[str] = []
    if document.get("schema_version") != 1:
        return ["capability registry schema_version must be 1"]
    values = document.get("capabilities")
    if not isinstance(values, list) or not values:
        return ["capability registry must contain a non-empty capabilities list"]

    identifiers: set[str] = set()
    for capability in values:
        if not isinstance(capability, dict):
            errors.append("capability record is not an object")
            continue
        identifier = capability.get("id")
        if not isinstance(identifier, str) or not CAPABILITY_ID.fullmatch(identifier):
            errors.append("capability id must use lowercase mechanism words")
            continue
        if identifier in identifiers:
            errors.append(f"duplicate capability id: {identifier}")
        identifiers.add(identifier)
        state = capability.get("state")
        if state not in VALID_STATES:
            errors.append(f"{identifier}: unsupported state {state!r}")
        for field in ("scope", "next_gate"):
            if not isinstance(capability.get(field), str) or not str(capability[field]).strip():
                errors.append(f"{identifier}: {field} must be a non-empty string")
        owner = capability.get("owner")
        if owner not in VALID_OWNERS:
            errors.append(f"{identifier}: owner must be a declared maintenance role")
        limitations = capability.get("limitations")
        if (
            not isinstance(limitations, list)
            or not limitations
            or not all(isinstance(item, str) and item.strip() for item in limitations)
        ):
            errors.append(f"{identifier}: limitations must be a non-empty string list")
        if state == "blocked" and (
            not isinstance(capability.get("blocker"), str) or not str(capability["blocker"]).strip()
        ):
            errors.append(f"{identifier}: blocked capability requires a blocker")

        evidence = capability.get("evidence")
        if not isinstance(evidence, list) or not evidence:
            errors.append(f"{identifier}: evidence must be a non-empty list")
            continue
        kinds: set[str] = set()
        for item in evidence:
            if not isinstance(item, dict):
                errors.append(f"{identifier}: evidence entry is not an object")
                continue
            kind = item.get("kind")
            path = item.get("path")
            if kind not in VALID_EVIDENCE_KINDS:
                errors.append(f"{identifier}: unsupported evidence kind {kind!r}")
            else:
                kinds.add(str(kind))
            if not isinstance(path, str) or not path:
                errors.append(f"{identifier}: evidence path must be a string")
            else:
                try:
                    if not resolve_path(path).is_file():
                        errors.append(f"{identifier}: evidence path is missing: {path}")
                except ValueError as error:
                    errors.append(f"{identifier}: {error}")
            command = item.get("command")
            if command is not None and (not isinstance(command, str) or not command.strip()):
                errors.append(
                    f"{identifier}: evidence command must be a non-empty string when present"
                )
        if state == "synthesized" and "synthesis" not in kinds:
            errors.append(f"{identifier}: synthesized state requires synthesis evidence")
        if state == "hardware-probed" and "hardware" not in kinds:
            errors.append(f"{identifier}: hardware-probed state requires hardware evidence")
        if state == "reproduced" and not ({"runtime", "manifest", "test"} & kinds):
            errors.append(
                f"{identifier}: reproduced state requires runtime, manifest, or test evidence"
            )
    return errors


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--registry", type=Path, default=ROOT / "docs" / "meta" / "capabilities.json"
    )
    arguments = parser.parse_args(argv)
    try:
        document = json.loads(arguments.registry.read_text(encoding="utf-8"))
        if not isinstance(document, dict):
            raise ValueError("capability registry root must be an object")
        errors = verify(document)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"verify_capability_registry: {error}", file=sys.stderr)
        return 1
    if errors:
        for error in errors:
            print(f"verify_capability_registry: {error}", file=sys.stderr)
        return 1
    print(f"Validated {len(document['capabilities'])} capability records.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
