#!/usr/bin/env python3
"""Verify that every RustSec exception stays explicit, current, and time-bounded."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import shutil
import subprocess
import sys
import tomllib
from collections.abc import Iterable
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REQUIRED_EXCEPTION_FIELDS = {
    "id",
    "package",
    "category",
    "expires",
    "scope",
    "risk",
    "mitigation",
    "review",
    "dependency_paths",
}


def load_toml(path: Path) -> dict[str, object]:
    with path.open("rb") as handle:
        document = tomllib.load(handle)
    if not isinstance(document, dict):
        raise ValueError(f"{path}: TOML root must be a table")
    return document


def load_audit_report(path: Path | None) -> dict[str, object]:
    if path is not None:
        document = json.loads(path.read_text(encoding="utf-8"))
    else:
        cargo = shutil.which("cargo")
        if cargo is None:
            raise ValueError("cargo is not available on PATH")
        result = subprocess.run(  # noqa: S603 - fixed repository-local audit command
            [cargo, "audit", "--json"],
            check=False,
            capture_output=True,
            cwd=ROOT,
            text=True,
        )
        if not result.stdout:
            raise ValueError(f"cargo audit produced no JSON output: {result.stderr.strip()}")
        document = json.loads(result.stdout)
    if not isinstance(document, dict):
        raise ValueError("cargo audit JSON root must be an object")
    return document


def audit_entries(report: dict[str, object]) -> dict[str, set[tuple[str, str]]]:
    entries: dict[str, set[tuple[str, str]]] = {}

    def collect(category: str, values: object) -> None:
        if not isinstance(values, list):
            raise ValueError(f"cargo audit {category} must be a list")
        for value in values:
            if not isinstance(value, dict):
                raise ValueError(f"cargo audit {category} contains a non-object entry")
            advisory = value.get("advisory")
            package = value.get("package")
            if not isinstance(advisory, dict) or not isinstance(package, dict):
                raise ValueError(f"cargo audit {category} entry lacks advisory or package")
            identifier = advisory.get("id")
            package_name = package.get("name")
            if not isinstance(identifier, str) or not isinstance(package_name, str):
                raise ValueError(f"cargo audit {category} entry lacks advisory id or package name")
            entries.setdefault(identifier, set()).add((category, package_name))

    vulnerabilities = report.get("vulnerabilities")
    if not isinstance(vulnerabilities, dict):
        raise ValueError("cargo audit report lacks vulnerabilities object")
    collect("vulnerabilities", vulnerabilities.get("list"))
    warnings = report.get("warnings")
    if not isinstance(warnings, dict):
        raise ValueError("cargo audit report lacks warnings object")
    for category, values in warnings.items():
        if not isinstance(category, str):
            raise ValueError("cargo audit warning category is not a string")
        collect(category, values)
    return entries


def exception_entries(document: dict[str, object], today: dt.date) -> dict[str, dict[str, object]]:
    if document.get("schema_version") != 1:
        raise ValueError("advisory exception schema_version must be 1")
    values = document.get("exception")
    if not isinstance(values, list):
        raise ValueError("advisory exceptions must contain an exception list")

    exceptions: dict[str, dict[str, object]] = {}
    for value in values:
        if not isinstance(value, dict):
            raise ValueError("advisory exception must be a table")
        missing = REQUIRED_EXCEPTION_FIELDS - value.keys()
        if missing:
            raise ValueError(f"advisory exception lacks fields: {', '.join(sorted(missing))}")
        identifier = value["id"]
        if not isinstance(identifier, str) or not identifier.startswith("RUSTSEC-"):
            raise ValueError("advisory exception id must be a RustSec identifier")
        if identifier in exceptions:
            raise ValueError(f"duplicate advisory exception: {identifier}")
        for field in ("package", "category", "scope", "risk", "mitigation", "review"):
            if not isinstance(value[field], str) or not str(value[field]).strip():
                raise ValueError(f"{identifier}: {field} must be a non-empty string")
        dependency_paths = value["dependency_paths"]
        if (
            not isinstance(dependency_paths, list)
            or not dependency_paths
            or not all(isinstance(path, str) and path.strip() for path in dependency_paths)
        ):
            raise ValueError(f"{identifier}: dependency_paths must be a non-empty string list")
        try:
            expiry = dt.date.fromisoformat(str(value["expires"]))
        except ValueError as error:
            raise ValueError(f"{identifier}: expires must use YYYY-MM-DD") from error
        if expiry <= today:
            raise ValueError(f"{identifier}: exception expired on {expiry.isoformat()}")
        exceptions[identifier] = value
    return exceptions


def deny_ignored_ids(document: dict[str, object]) -> set[str]:
    advisories = document.get("advisories")
    if not isinstance(advisories, dict):
        raise ValueError("deny.toml lacks advisories table")
    ignored = advisories.get("ignore")
    if not isinstance(ignored, list):
        raise ValueError("deny.toml advisories.ignore must be a list")
    identifiers: set[str] = set()
    for value in ignored:
        if not isinstance(value, dict) or not isinstance(value.get("id"), str):
            raise ValueError("deny.toml advisory ignore lacks an id")
        identifier = value["id"]
        if identifier in identifiers:
            raise ValueError(f"deny.toml repeats ignored advisory {identifier}")
        identifiers.add(identifier)
    return identifiers


def verify(
    exceptions: dict[str, dict[str, object]],
    ignored_ids: set[str],
    audited: dict[str, set[tuple[str, str]]],
) -> list[str]:
    errors: list[str] = []
    exception_ids = set(exceptions)
    if ignored_ids != exception_ids:
        missing = sorted(exception_ids - ignored_ids)
        stale = sorted(ignored_ids - exception_ids)
        if missing:
            errors.append(f"deny.toml omits registered exceptions: {', '.join(missing)}")
        if stale:
            errors.append(f"deny.toml has unregistered exceptions: {', '.join(stale)}")

    audited_ids = set(audited)
    untracked = sorted(audited_ids - exception_ids)
    stale = sorted(exception_ids - audited_ids)
    if untracked:
        errors.append(f"cargo audit has untracked advisories: {', '.join(untracked)}")
    if stale:
        errors.append(f"registered exceptions no longer appear in cargo audit: {', '.join(stale)}")

    for identifier in sorted(exception_ids & audited_ids):
        exception = exceptions[identifier]
        expected = (str(exception["category"]), str(exception["package"]))
        observed = audited[identifier]
        if expected not in observed:
            values = ", ".join(f"{category}:{package}" for category, package in sorted(observed))
            errors.append(f"{identifier}: expected {expected[0]}:{expected[1]}, observed {values}")
    return errors


def parse_today(value: str) -> dt.date:
    try:
        return dt.date.fromisoformat(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("today must use YYYY-MM-DD") from error


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--exceptions",
        type=Path,
        default=ROOT / "security" / "advisory-exceptions.toml",
    )
    parser.add_argument("--deny-config", type=Path, default=ROOT / "deny.toml")
    parser.add_argument("--audit-json", type=Path)
    parser.add_argument("--today", type=parse_today, default=dt.date.today())
    arguments = parser.parse_args(argv)

    try:
        exceptions = exception_entries(load_toml(arguments.exceptions), arguments.today)
        ignored_ids = deny_ignored_ids(load_toml(arguments.deny_config))
        audited = audit_entries(load_audit_report(arguments.audit_json))
        errors = verify(exceptions, ignored_ids, audited)
    except (OSError, ValueError, json.JSONDecodeError, tomllib.TOMLDecodeError) as error:
        print(f"verify_advisory_exceptions: {error}", file=sys.stderr)
        return 1

    if errors:
        for error in errors:
            print(f"verify_advisory_exceptions: {error}", file=sys.stderr)
        return 1
    print(f"Verified {len(exceptions)} advisory exceptions against cargo audit.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
