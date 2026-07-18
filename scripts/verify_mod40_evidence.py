#!/usr/bin/env python3
"""Validate the tracked MOD 40 route ledger against its source registry."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import yaml

VALID_EVIDENCE = frozenset({"direct", "partial"})
VALID_GATE_STATUS = frozenset({"blocked", "closed"})
REQUIRED_ROUTE_KEYS = frozenset({"id", "gate", "evidence", "source_ids", "locator", "from", "to", "assertions", "unresolved"})


class EvidenceValidationError(ValueError):
    """The route ledger violates its source-bound contract."""


def load_source_ids(path: Path) -> set[str]:
    """Return every declared Intellec source ID."""

    with path.open(encoding="utf-8") as handle:
        source_registry = yaml.safe_load(handle)
    if not isinstance(source_registry, dict):
        raise EvidenceValidationError("source registry root is not a mapping")
    source_ids = {
        source.get("id")
        for source in source_registry.get("sources", [])
        if isinstance(source, dict) and isinstance(source.get("id"), str)
    }
    if not source_ids:
        raise EvidenceValidationError("source registry contains no source IDs")
    return source_ids


def require_string(record: dict[str, object], key: str, context: str) -> str:
    """Return one nonempty string field or raise a precise validation error."""

    value = record.get(key)
    if not isinstance(value, str) or not value:
        raise EvidenceValidationError(f"{context} has no nonempty {key}")
    return value


def validate_ledger(ledger: object, source_ids: set[str]) -> None:
    """Require every gate and route to retain source-bound incompleteness."""

    if not isinstance(ledger, dict):
        raise EvidenceValidationError("route ledger root is not a mapping")
    if ledger.get("schema") != "mcs4.mod40.route-ledger.v1":
        raise EvidenceValidationError("route ledger schema is unsupported")
    gates = ledger.get("source_gates")
    routes = ledger.get("routes")
    if not isinstance(gates, dict) or not isinstance(routes, list):
        raise EvidenceValidationError("route ledger gates or routes are malformed")

    route_ids: set[str] = set()
    routes_by_id: dict[str, dict[str, object]] = {}
    for route in routes:
        if not isinstance(route, dict):
            raise EvidenceValidationError("route entry is not a mapping")
        missing_keys = REQUIRED_ROUTE_KEYS.difference(route)
        if missing_keys:
            raise EvidenceValidationError(f"route misses required fields: {sorted(missing_keys)}")
        route_id = require_string(route, "id", "route")
        if route_id in route_ids:
            raise EvidenceValidationError(f"duplicate route ID: {route_id}")
        route_ids.add(route_id)
        routes_by_id[route_id] = route
        gate = require_string(route, "gate", route_id)
        if gate not in gates:
            raise EvidenceValidationError(f"route {route_id} references unknown gate {gate}")
        if route.get("evidence") not in VALID_EVIDENCE:
            raise EvidenceValidationError(f"route {route_id} has invalid evidence state")
        source_id_list = route.get("source_ids")
        if not isinstance(source_id_list, list) or not source_id_list or not all(isinstance(value, str) for value in source_id_list):
            raise EvidenceValidationError(f"route {route_id} has invalid source_ids")
        unknown_sources = set(source_id_list).difference(source_ids)
        if unknown_sources:
            raise EvidenceValidationError(f"route {route_id} references unknown sources: {sorted(unknown_sources)}")
        for key in ("locator", "from", "to"):
            require_string(route, key, route_id)
        assertions = route.get("assertions")
        unresolved = route.get("unresolved")
        if not isinstance(assertions, list) or not assertions or not all(isinstance(value, str) and value for value in assertions):
            raise EvidenceValidationError(f"route {route_id} has invalid assertions")
        if not isinstance(unresolved, list) or not all(isinstance(value, str) and value for value in unresolved):
            raise EvidenceValidationError(f"route {route_id} has invalid unresolved entries")
        if route["evidence"] == "direct" and unresolved:
            raise EvidenceValidationError(f"direct route {route_id} retains unresolved claims")
        if route["evidence"] == "partial" and not unresolved:
            raise EvidenceValidationError(f"partial route {route_id} does not name its unresolved evidence")

    for gate_id, gate in gates.items():
        if not isinstance(gate_id, str) or not isinstance(gate, dict):
            raise EvidenceValidationError("source gate entry is malformed")
        status = gate.get("status")
        blocking_routes = gate.get("blocking_routes")
        if status not in VALID_GATE_STATUS:
            raise EvidenceValidationError(f"gate {gate_id} has invalid status")
        if not isinstance(blocking_routes, list) or not blocking_routes or not all(isinstance(value, str) for value in blocking_routes):
            raise EvidenceValidationError(f"gate {gate_id} has invalid blocking routes")
        unknown_routes = set(blocking_routes).difference(route_ids)
        if unknown_routes:
            raise EvidenceValidationError(f"gate {gate_id} references unknown routes: {sorted(unknown_routes)}")
        if status == "blocked" and not any(routes_by_id[route_id]["evidence"] == "partial" for route_id in blocking_routes):
            raise EvidenceValidationError(f"blocked gate {gate_id} has no partial blocking route")
        if status == "closed" and any(routes_by_id[route_id]["evidence"] != "direct" for route_id in blocking_routes):
            raise EvidenceValidationError(f"closed gate {gate_id} retains partial evidence")


def main(arguments: list[str] | None = None) -> int:
    """Validate the canonical tracked ledger and report one deterministic result."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ledger", type=Path, default=Path("docs/evidence/intellec/mod40_route_ledger_v1.json"))
    parser.add_argument("--sources", type=Path, default=Path("docs/evidence/intellec_sources.yaml"))
    parsed = parser.parse_args(arguments)
    try:
        with parsed.ledger.open(encoding="ascii") as handle:
            ledger = json.load(handle)
        validate_ledger(ledger, load_source_ids(parsed.sources))
    except (EvidenceValidationError, OSError, json.JSONDecodeError, yaml.YAMLError) as error:
        print(f"MOD 40 evidence validation failed: {error}")
        return 1
    print(f"PASS MOD 40 route ledger: {parsed.ledger}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
