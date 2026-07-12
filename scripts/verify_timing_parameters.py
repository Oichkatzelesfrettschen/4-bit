#!/usr/bin/env python3
"""Verify source, bounds, and use-site links for active timing parameters."""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections.abc import Iterable
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_LEDGER = ROOT / "docs" / "evidence" / "timing_parameters.json"
IDENTIFIER = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*\Z")


def resolve_path(root: Path, value: str) -> Path:
    relative = Path(value)
    if relative.is_absolute():
        raise ValueError(f"path must be repository-relative: {value}")
    path = (root / relative).resolve()
    try:
        path.relative_to(root.resolve())
    except ValueError as error:
        raise ValueError(f"path escapes repository: {value}") from error
    return path


def nonempty_string(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip())


def verify(document: dict[str, object], *, root: Path = ROOT) -> list[str]:
    errors: list[str] = []
    if document.get("schema_version") != 1:
        return ["timing parameter ledger schema_version must be 1"]
    parameters = document.get("parameters")
    if not isinstance(parameters, list) or not parameters:
        return ["timing parameter ledger must contain a non-empty parameters list"]

    identifiers: set[str] = set()
    for parameter in parameters:
        if not isinstance(parameter, dict):
            errors.append("timing parameter record is not an object")
            continue
        identifier = parameter.get("id")
        if not isinstance(identifier, str) or not IDENTIFIER.fullmatch(identifier):
            errors.append("timing parameter id must use lowercase mechanism words")
            continue
        if identifier in identifiers:
            errors.append(f"duplicate timing parameter id: {identifier}")
        identifiers.add(identifier)
        for field in ("quantity", "unit", "status", "limitation", "falsifier"):
            if not nonempty_string(parameter.get(field)):
                errors.append(f"{identifier}: {field} must be a non-empty string")

        bounds = parameter.get("bounds")
        selected = parameter.get("selected_value")
        if (
            not isinstance(bounds, dict)
            or not isinstance(bounds.get("minimum"), int)
            or not isinstance(bounds.get("maximum"), int)
            or not isinstance(selected, int)
        ):
            errors.append(f"{identifier}: bounds and selected_value must be integer values")
        elif bounds["minimum"] <= 0 or bounds["minimum"] > bounds["maximum"]:
            errors.append(f"{identifier}: bounds must be positive and ordered")
        elif not bounds["minimum"] <= selected <= bounds["maximum"]:
            errors.append(f"{identifier}: selected_value must stay inside bounds")

        source = parameter.get("source")
        if not isinstance(source, dict) or source.get("kind") != "primary-ocr":
            errors.append(f"{identifier}: source must be a primary-ocr object")
            continue
        source_path = source.get("path")
        start = source.get("line_start")
        end = source.get("line_end")
        tokens = source.get("required_tokens")
        if (
            not isinstance(source_path, str)
            or not isinstance(start, int)
            or not isinstance(end, int)
            or start <= 0
            or end < start
            or not isinstance(tokens, list)
            or not tokens
            or not all(nonempty_string(token) for token in tokens)
        ):
            errors.append(f"{identifier}: source locator is malformed")
        else:
            try:
                # OCR locators use `rg -n` line numbers. Split only on LF so
                # embedded form-feed page separators do not shift those locators.
                lines = resolve_path(root, source_path).read_text(encoding="utf-8").split("\n")
            except (OSError, ValueError) as error:
                errors.append(f"{identifier}: cannot read source locator: {error}")
            else:
                if end > len(lines):
                    errors.append(f"{identifier}: source locator exceeds file length")
                else:
                    excerpt = "\n".join(lines[start - 1 : end]).lower()
                    missing = [str(token) for token in tokens if str(token).lower() not in excerpt]
                    if missing:
                        errors.append(
                            f"{identifier}: source locator lacks required tokens: {', '.join(missing)}"
                        )

        use_sites = parameter.get("use_sites")
        if not isinstance(use_sites, list) or not use_sites:
            errors.append(f"{identifier}: use_sites must be a non-empty list")
            continue
        for use_site in use_sites:
            if not isinstance(use_site, dict):
                errors.append(f"{identifier}: use site is not an object")
                continue
            path = use_site.get("path")
            symbol = use_site.get("symbol")
            if not isinstance(path, str) or not nonempty_string(symbol):
                errors.append(f"{identifier}: use site requires path and symbol")
                continue
            try:
                content = resolve_path(root, path).read_text(encoding="utf-8")
            except (OSError, ValueError) as error:
                errors.append(f"{identifier}: cannot read use site: {error}")
                continue
            if str(symbol) not in content:
                errors.append(f"{identifier}: use-site symbol is absent: {path}: {symbol}")
    return errors


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ledger", type=Path, default=DEFAULT_LEDGER)
    arguments = parser.parse_args(argv)
    try:
        document = json.loads(arguments.ledger.read_text(encoding="utf-8"))
        if not isinstance(document, dict):
            raise ValueError("timing parameter ledger root must be an object")
        errors = verify(document)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"verify_timing_parameters: {error}", file=sys.stderr)
        return 1
    if errors:
        for error in errors:
            print(f"verify_timing_parameters: {error}", file=sys.stderr)
        return 1
    print(f"Validated {len(document['parameters'])} timing parameter records.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
