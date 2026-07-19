#!/usr/bin/env python3
"""Validate the tracked MOD 40 route ledger against its source registry."""

from __future__ import annotations

import argparse
import json
import tempfile
from pathlib import Path
from typing import cast

import yaml

VALID_EVIDENCE = frozenset({"direct", "partial"})
VALID_GATE_STATUS = frozenset({"blocked", "closed"})
VALID_CLOSURE_STATE = frozenset({"missing", "partial", "verified"})
REQUIRED_ROUTE_KEYS = frozenset({"id", "gate", "evidence", "source_ids", "locator", "from", "to", "assertions", "unresolved"})
REQUIRED_CLOSURE_KEYS = frozenset({"id", "gate", "state", "source_ids", "closure_artifact", "acceptance"})


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
    closure_requirements = ledger.get("closure_requirements")
    if not isinstance(gates, dict) or not isinstance(routes, list) or not isinstance(closure_requirements, list):
        raise EvidenceValidationError("route ledger gates, routes, or closure requirements are malformed")

    closure_by_id: dict[str, dict[str, object]] = {}
    closure_ids_by_gate: dict[str, set[str]] = {}
    for closure in closure_requirements:
        if not isinstance(closure, dict):
            raise EvidenceValidationError("closure requirement is not a mapping")
        missing_keys = REQUIRED_CLOSURE_KEYS.difference(closure)
        if missing_keys:
            raise EvidenceValidationError(f"closure requirement misses required fields: {sorted(missing_keys)}")
        closure_id = require_string(closure, "id", "closure requirement")
        if closure_id in closure_by_id:
            raise EvidenceValidationError(f"duplicate closure requirement ID: {closure_id}")
        gate = require_string(closure, "gate", closure_id)
        if gate not in gates:
            raise EvidenceValidationError(f"closure requirement {closure_id} references unknown gate {gate}")
        if closure.get("state") not in VALID_CLOSURE_STATE:
            raise EvidenceValidationError(f"closure requirement {closure_id} has invalid state")
        source_id_list = closure.get("source_ids")
        if not isinstance(source_id_list, list) or not source_id_list or not all(isinstance(value, str) for value in source_id_list):
            raise EvidenceValidationError(f"closure requirement {closure_id} has invalid source_ids")
        unknown_sources = set(source_id_list).difference(source_ids)
        if unknown_sources:
            raise EvidenceValidationError(
                f"closure requirement {closure_id} references unknown sources: {sorted(unknown_sources)}"
            )
        require_string(closure, "closure_artifact", closure_id)
        require_string(closure, "acceptance", closure_id)
        closure_by_id[closure_id] = closure
        closure_ids_by_gate.setdefault(gate, set()).add(closure_id)

    route_ids: set[str] = set()
    routes_by_id: dict[str, dict[str, object]] = {}
    referenced_closure_ids: set[str] = set()
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
        route_closure_ids = route.get("closure_requirement_ids", [])
        if not isinstance(route_closure_ids, list) or not all(isinstance(value, str) and value for value in route_closure_ids):
            raise EvidenceValidationError(f"route {route_id} has invalid closure_requirement_ids")
        if route["evidence"] == "partial" and not route_closure_ids:
            raise EvidenceValidationError(f"partial route {route_id} has no closure requirements")
        for closure_id in route_closure_ids:
            closure = closure_by_id.get(closure_id)
            if closure is None:
                raise EvidenceValidationError(f"route {route_id} references unknown closure requirement {closure_id}")
            if closure["gate"] != gate:
                raise EvidenceValidationError(f"route {route_id} crosses closure requirement gate {closure_id}")
            referenced_closure_ids.add(closure_id)

    unreferenced_closure_ids = set(closure_by_id).difference(referenced_closure_ids)
    if unreferenced_closure_ids:
        raise EvidenceValidationError(f"closure requirements lack route references: {sorted(unreferenced_closure_ids)}")

    for gate_id, gate in gates.items():
        if not isinstance(gate_id, str) or not isinstance(gate, dict):
            raise EvidenceValidationError("source gate entry is malformed")
        status = gate.get("status")
        blocking_routes = gate.get("blocking_routes")
        if status not in VALID_GATE_STATUS:
            raise EvidenceValidationError(f"gate {gate_id} has invalid status")
        gate_closure_ids = closure_ids_by_gate.get(gate_id, set())
        if not gate_closure_ids:
            raise EvidenceValidationError(f"gate {gate_id} has no closure requirements")
        if not isinstance(blocking_routes, list) or not blocking_routes or not all(isinstance(value, str) for value in blocking_routes):
            raise EvidenceValidationError(f"gate {gate_id} has invalid blocking routes")
        unknown_routes = set(blocking_routes).difference(route_ids)
        if unknown_routes:
            raise EvidenceValidationError(f"gate {gate_id} references unknown routes: {sorted(unknown_routes)}")
        if status == "blocked" and not any(routes_by_id[route_id]["evidence"] == "partial" for route_id in blocking_routes):
            raise EvidenceValidationError(f"blocked gate {gate_id} has no partial blocking route")
        if status == "closed" and any(routes_by_id[route_id]["evidence"] != "direct" for route_id in blocking_routes):
            raise EvidenceValidationError(f"closed gate {gate_id} retains partial evidence")
        closure_states = {closure_by_id[closure_id]["state"] for closure_id in gate_closure_ids}
        if status == "blocked" and closure_states == {"verified"}:
            raise EvidenceValidationError(f"blocked gate {gate_id} retains no incomplete closure requirement")
        if status == "closed" and closure_states != {"verified"}:
            raise EvidenceValidationError(f"closed gate {gate_id} retains incomplete closure requirements")


def build_status_report(ledger: dict[str, object]) -> dict[str, object]:
    """Build one deterministic consumer report from a validated route ledger."""

    gates_value = ledger["source_gates"]
    routes_value = ledger["routes"]
    closure_requirements_value = ledger["closure_requirements"]
    if not isinstance(gates_value, dict) or not isinstance(routes_value, list) or not isinstance(closure_requirements_value, list):
        raise EvidenceValidationError("validated ledger has malformed status-report inputs")
    gates = cast(dict[str, object], gates_value)
    routes = cast(list[object], routes_value)
    closure_requirements = cast(list[object], closure_requirements_value)

    closure_routes: dict[str, list[str]] = {}
    for route in routes:
        if not isinstance(route, dict):
            raise EvidenceValidationError("validated ledger has a non-mapping route")
        route_id = route.get("id")
        if not isinstance(route_id, str):
            raise EvidenceValidationError("validated ledger route has no string ID")
        for closure_id in route.get("closure_requirement_ids", []):
            if not isinstance(closure_id, str):
                raise EvidenceValidationError("validated ledger route has a non-string closure ID")
            closure_routes.setdefault(closure_id, []).append(route_id)

    report_gates: list[dict[str, object]] = []
    total_states = {state: 0 for state in sorted(VALID_CLOSURE_STATE)}
    for gate_id in gates:
        if not isinstance(gate_id, str):
            raise EvidenceValidationError("validated ledger has a non-string gate ID")
        gate = gates[gate_id]
        if not isinstance(gate, dict):
            raise EvidenceValidationError(f"validated ledger gate {gate_id} is not a mapping")
        requirements: list[dict[str, object]] = []
        gate_states = {state: 0 for state in sorted(VALID_CLOSURE_STATE)}
        for closure in sorted(
            (item for item in closure_requirements if isinstance(item, dict) and item.get("gate") == gate_id),
            key=lambda item: str(item["id"]),
        ):
            closure_id = closure.get("id")
            state = closure.get("state")
            source_id_list = closure.get("source_ids")
            if not isinstance(closure_id, str) or not isinstance(state, str) or not isinstance(source_id_list, list):
                raise EvidenceValidationError("validated ledger has a malformed closure requirement")
            gate_states[state] += 1
            total_states[state] += 1
            requirements.append(
                {
                    "id": closure_id,
                    "state": state,
                    "source_ids": sorted(source_id_list),
                    "closure_artifact": closure["closure_artifact"],
                    "acceptance": closure["acceptance"],
                    "route_ids": sorted(closure_routes[closure_id]),
                }
            )
        report_gates.append(
            {
                "id": gate_id,
                "status": gate["status"],
                "blocking_route_ids": sorted(gate["blocking_routes"]),
                "requirements": requirements,
                "requirement_counts": {**gate_states, "total": len(requirements)},
            }
        )

    return {
        "schema": "mcs4.mod40.evidence-status.v1",
        "source_registry": ledger["source_registry"],
        "gates": report_gates,
        "requirement_counts": {**total_states, "total": sum(total_states.values())},
    }


def write_status_report(path: Path, report: dict[str, object]) -> None:
    """Write a report atomically without modifying the source ledger."""

    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        mode="w",
        encoding="ascii",
        dir=path.parent,
        prefix=f".{path.name}.",
        suffix=".tmp",
        delete=False,
    ) as handle:
        temporary_path = Path(handle.name)
        json.dump(report, handle, indent=2, sort_keys=True)
        handle.write("\n")
    temporary_path.replace(path)


def main(arguments: list[str] | None = None) -> int:
    """Validate the canonical tracked ledger and report one deterministic result."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ledger", type=Path, default=Path("docs/evidence/intellec/mod40_route_ledger_v1.json"))
    parser.add_argument("--sources", type=Path, default=Path("docs/evidence/intellec_sources.yaml"))
    parser.add_argument("--report", type=Path, help="Write a deterministic gate-status report after validation.")
    parsed = parser.parse_args(arguments)
    try:
        with parsed.ledger.open(encoding="ascii") as handle:
            ledger = json.load(handle)
        validate_ledger(ledger, load_source_ids(parsed.sources))
        if parsed.report is not None:
            if not isinstance(ledger, dict):
                raise EvidenceValidationError("route ledger root is not a mapping")
            write_status_report(parsed.report, build_status_report(ledger))
    except (EvidenceValidationError, OSError, json.JSONDecodeError, yaml.YAMLError) as error:
        print(f"MOD 40 evidence validation failed: {error}")
        return 1
    print(f"PASS MOD 40 route ledger: {parsed.ledger}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
