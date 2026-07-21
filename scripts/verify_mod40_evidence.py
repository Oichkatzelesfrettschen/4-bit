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
VALID_PIN_CONNECTIVITY = frozenset({"direct", "partial"})
VALID_PIN_BEHAVIOR = frozenset({"partial", "verified"})
VALID_PIN_KINDS = frozenset({"connector_to_connector", "net_to_pin", "observation"})
VALID_ENDPOINT_KINDS = frozenset(
    {"component_pin", "component_signal", "connector_contact", "connector_range", "named_net", "terminal_contact"}
)
REQUIRED_ROUTE_KEYS = frozenset({"id", "gate", "evidence", "source_ids", "locator", "from", "to", "assertions", "unresolved"})
REQUIRED_CLOSURE_KEYS = frozenset({"id", "gate", "state", "depends_on", "source_ids", "closure_artifact", "acceptance"})


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


def topological_requirement_ids(closure_by_id: dict[str, dict[str, object]]) -> list[str]:
    """Return a deterministic prerequisite-first closure-requirement order."""

    unresolved_dependencies = {
        closure_id: set(cast(list[str], closure["depends_on"]))
        for closure_id, closure in closure_by_id.items()
    }
    ready_ids = sorted(closure_id for closure_id, dependencies in unresolved_dependencies.items() if not dependencies)
    ordered_ids: list[str] = []
    while ready_ids:
        closure_id = ready_ids.pop(0)
        ordered_ids.append(closure_id)
        for dependent_id in sorted(unresolved_dependencies):
            dependencies = unresolved_dependencies[dependent_id]
            if closure_id not in dependencies:
                continue
            dependencies.remove(closure_id)
            if not dependencies and dependent_id not in ordered_ids and dependent_id not in ready_ids:
                ready_ids.append(dependent_id)
        ready_ids.sort()
    if len(ordered_ids) != len(closure_by_id):
        cyclic_ids = sorted(set(closure_by_id).difference(ordered_ids))
        raise EvidenceValidationError(f"closure requirement dependencies contain a cycle: {cyclic_ids}")
    return ordered_ids


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
        dependency_ids = closure.get("depends_on")
        if not isinstance(dependency_ids, list) or not all(isinstance(value, str) and value for value in dependency_ids):
            raise EvidenceValidationError(f"closure requirement {closure_id} has invalid depends_on")
        if len(set(dependency_ids)) != len(dependency_ids):
            raise EvidenceValidationError(f"closure requirement {closure_id} has duplicate dependencies")
        if closure_id in dependency_ids:
            raise EvidenceValidationError(f"closure requirement {closure_id} depends on itself")
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

    for closure_id, closure in closure_by_id.items():
        dependency_ids = cast(list[str], closure["depends_on"])
        unknown_dependency_ids = set(dependency_ids).difference(closure_by_id)
        if unknown_dependency_ids:
            raise EvidenceValidationError(
                f"closure requirement {closure_id} references unknown dependencies: {sorted(unknown_dependency_ids)}"
            )
        closure_state = cast(str, closure["state"])
        for dependency_id in dependency_ids:
            dependency_state = cast(str, closure_by_id[dependency_id]["state"])
            if closure_state == "verified" and dependency_state != "verified":
                raise EvidenceValidationError(
                    f"verified closure requirement {closure_id} depends on incomplete requirement {dependency_id}"
                )
    topological_requirement_ids(closure_by_id)

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


def validate_pin_net_ledger(pin_ledger: object, route_ledger: dict[str, object], source_ids: set[str]) -> None:
    """Validate source-located component, pin, connector, and net records."""

    if not isinstance(pin_ledger, dict):
        raise EvidenceValidationError("component-pin ledger root is not a mapping")
    if pin_ledger.get("schema") != "mcs4.mod40.component-pin-net.v1":
        raise EvidenceValidationError("component-pin ledger schema is unsupported")
    if pin_ledger.get("source_registry") != route_ledger.get("source_registry"):
        raise EvidenceValidationError("component-pin and route ledgers use different source registries")
    source_pdf_sha256 = pin_ledger.get("source_pdf_sha256")
    if not isinstance(source_pdf_sha256, str) or len(source_pdf_sha256) != 64:
        raise EvidenceValidationError("component-pin ledger has no valid source PDF SHA-256")

    route_values = route_ledger.get("routes")
    if not isinstance(route_values, list):
        raise EvidenceValidationError("route ledger has no routes for component-pin validation")
    routes = {
        route_id: route
        for route in route_values
        if isinstance(route, dict)
        if isinstance((route_id := route.get("id")), str)
    }

    review_regions = pin_ledger.get("review_regions")
    records = pin_ledger.get("records")
    if not isinstance(review_regions, list) or not isinstance(records, list):
        raise EvidenceValidationError("component-pin ledger regions or records are malformed")

    region_ids: set[str] = set()
    for region in review_regions:
        if not isinstance(region, dict):
            raise EvidenceValidationError("component-pin review region is not a mapping")
        region_id = require_string(region, "id", "component-pin review region")
        if region_id in region_ids:
            raise EvidenceValidationError(f"duplicate component-pin review region ID: {region_id}")
        region_ids.add(region_id)
        source_id = require_string(region, "source_id", region_id)
        if source_id not in source_ids:
            raise EvidenceValidationError(f"review region {region_id} references unknown source {source_id}")
        require_string(region, "locator", region_id)
        if not isinstance(region.get("page"), int) or cast(int, region["page"]) < 1:
            raise EvidenceValidationError(f"review region {region_id} has invalid page")
        if not isinstance(region.get("dpi"), int) or cast(int, region["dpi"]) < 300:
            raise EvidenceValidationError(f"review region {region_id} has insufficient DPI")
        bbox = region.get("bbox")
        if not isinstance(bbox, list) or len(bbox) != 4 or not all(isinstance(value, int) and value >= 0 for value in bbox):
            raise EvidenceValidationError(f"review region {region_id} has invalid bbox")
        if bbox[2] == 0 or bbox[3] == 0:
            raise EvidenceValidationError(f"review region {region_id} has empty bbox")
        score = region.get("registration_score")
        if not isinstance(score, (int, float)) or not 0.9 <= score <= 1.0:
            raise EvidenceValidationError(f"review region {region_id} has unacceptable registration score")

    record_ids: set[str] = set()
    for record in records:
        if not isinstance(record, dict):
            raise EvidenceValidationError("component-pin record is not a mapping")
        record_id = require_string(record, "id", "component-pin record")
        if record_id in record_ids:
            raise EvidenceValidationError(f"duplicate component-pin record ID: {record_id}")
        record_ids.add(record_id)
        if record.get("kind") not in VALID_PIN_KINDS:
            raise EvidenceValidationError(f"component-pin record {record_id} has invalid kind")
        require_string(record, "scope", record_id)
        if record.get("connectivity_status") not in VALID_PIN_CONNECTIVITY:
            raise EvidenceValidationError(f"component-pin record {record_id} has invalid connectivity status")
        behavior_status = record.get("behavior_status")
        if behavior_status not in VALID_PIN_BEHAVIOR:
            raise EvidenceValidationError(f"component-pin record {record_id} has invalid behavior status")

        route_id = require_string(record, "derived_from_route_id", record_id)
        route = routes.get(route_id)
        if route is None:
            raise EvidenceValidationError(f"component-pin record {record_id} references unknown route {route_id}")

        net = record.get("net")
        if not isinstance(net, dict):
            raise EvidenceValidationError(f"component-pin record {record_id} has invalid net")
        require_string(net, "canonical", f"{record_id} net")
        aliases = net.get("aliases")
        if not isinstance(aliases, list) or not all(isinstance(alias, str) and alias for alias in aliases):
            raise EvidenceValidationError(f"component-pin record {record_id} has invalid aliases")

        endpoints = record.get("endpoints")
        if not isinstance(endpoints, list) or len(endpoints) < 2:
            raise EvidenceValidationError(f"component-pin record {record_id} has fewer than two endpoints")
        endpoints_by_id: dict[str, dict[str, object]] = {}
        for endpoint in endpoints:
            if not isinstance(endpoint, dict):
                raise EvidenceValidationError(f"component-pin record {record_id} has a malformed endpoint")
            endpoint_id = require_string(endpoint, "id", f"{record_id} endpoint")
            if endpoint_id in endpoints_by_id:
                raise EvidenceValidationError(f"component-pin record {record_id} has duplicate endpoint {endpoint_id}")
            endpoint_kind = endpoint.get("kind")
            if endpoint_kind not in VALID_ENDPOINT_KINDS:
                raise EvidenceValidationError(f"component-pin record {record_id} endpoint {endpoint_id} has invalid kind")
            require_string(endpoint, "board", f"{record_id} endpoint {endpoint_id}")
            require_string(endpoint, "signal", f"{record_id} endpoint {endpoint_id}")
            if endpoint_kind in {"component_pin", "connector_contact", "connector_range", "terminal_contact"}:
                require_string(endpoint, "refdes", f"{record_id} endpoint {endpoint_id}")
                require_string(endpoint, "pin", f"{record_id} endpoint {endpoint_id}")
            endpoints_by_id[endpoint_id] = endpoint

        segments = record.get("segments")
        if not isinstance(segments, list) or not segments:
            raise EvidenceValidationError(f"component-pin record {record_id} has no segments")
        for segment in segments:
            if not isinstance(segment, dict):
                raise EvidenceValidationError(f"component-pin record {record_id} has a malformed segment")
            source_endpoint = require_string(segment, "from", f"{record_id} segment")
            target_endpoint = require_string(segment, "to", f"{record_id} segment")
            if source_endpoint not in endpoints_by_id or target_endpoint not in endpoints_by_id:
                raise EvidenceValidationError(f"component-pin record {record_id} segment references unknown endpoint")
            require_string(segment, "polarity", f"{record_id} segment")

        source_refs = record.get("source_refs")
        if not isinstance(source_refs, list) or not source_refs:
            raise EvidenceValidationError(f"component-pin record {record_id} has no source references")
        record_source_ids: set[str] = set()
        for source_ref in source_refs:
            if not isinstance(source_ref, dict):
                raise EvidenceValidationError(f"component-pin record {record_id} has malformed source reference")
            source_id = require_string(source_ref, "source_id", f"{record_id} source reference")
            if source_id not in source_ids:
                raise EvidenceValidationError(f"component-pin record {record_id} references unknown source {source_id}")
            record_source_ids.add(source_id)
            require_string(source_ref, "locator", f"{record_id} source reference")
            if source_ref.get("primary_sheet_reviewed") is not True or source_ref.get("ocr_only") is not False:
                raise EvidenceValidationError(f"component-pin record {record_id} relies on unreviewed or OCR-only evidence")
        if not record_source_ids.intersection(set(cast(list[str], route["source_ids"]))):
            raise EvidenceValidationError(f"component-pin record {record_id} has no source shared with route {route_id}")

        review_region_ids = record.get("review_region_ids")
        if not isinstance(review_region_ids, list) or not all(isinstance(value, str) for value in review_region_ids):
            raise EvidenceValidationError(f"component-pin record {record_id} has invalid review regions")
        unknown_region_ids = set(review_region_ids).difference(region_ids)
        if unknown_region_ids:
            raise EvidenceValidationError(
                f"component-pin record {record_id} references unknown review regions: {sorted(unknown_region_ids)}"
            )

        unresolved = record.get("unresolved")
        if not isinstance(unresolved, list) or not all(isinstance(value, str) and value for value in unresolved):
            raise EvidenceValidationError(f"component-pin record {record_id} has invalid unresolved entries")
        if behavior_status == "partial" and not unresolved:
            raise EvidenceValidationError(f"partial component-pin behavior {record_id} names no unresolved fact")
        if behavior_status == "verified" and unresolved:
            raise EvidenceValidationError(f"verified component-pin behavior {record_id} retains unresolved facts")

        polarity = record.get("polarity")
        if not isinstance(polarity, dict) or polarity.get("state") not in {"explicit", "unknown", "conflict"}:
            raise EvidenceValidationError(f"component-pin record {record_id} has invalid polarity")
        if polarity["state"] == "explicit" and not isinstance(polarity.get("asserted_level"), str):
            raise EvidenceValidationError(f"component-pin record {record_id} has explicit polarity without asserted level")
        if polarity["state"] != "explicit" and polarity.get("asserted_level") is not None:
            raise EvidenceValidationError(f"component-pin record {record_id} asserts an unproved polarity")
        inverting_stages = polarity.get("inverting_stages")
        if not isinstance(inverting_stages, list):
            raise EvidenceValidationError(f"component-pin record {record_id} has invalid inversion stages")

        timing = record.get("timing")
        if not isinstance(timing, dict) or timing.get("state") not in {"explicit", "partial", "unknown"}:
            raise EvidenceValidationError(f"component-pin record {record_id} has invalid timing")
        constraints = timing.get("constraints")
        if not isinstance(constraints, list):
            raise EvidenceValidationError(f"component-pin record {record_id} has invalid timing constraints")
        for constraint in constraints:
            if not isinstance(constraint, dict):
                raise EvidenceValidationError(f"component-pin record {record_id} has malformed timing constraint")
            for key in ("name", "unit", "scope", "source_id", "locator"):
                require_string(constraint, key, f"{record_id} timing constraint")
            if constraint["source_id"] not in source_ids or not isinstance(constraint.get("value"), (int, float)):
                raise EvidenceValidationError(f"component-pin record {record_id} has unsupported timing evidence")


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
    closure_by_id = {
        closure_id: closure
        for closure in closure_requirements
        if isinstance(closure, dict)
        if isinstance((closure_id := closure.get("id")), str)
    }
    topological_ids = topological_requirement_ids(closure_by_id)

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
    ready_requirement_ids: list[str] = []
    blocked_requirement_ids: list[str] = []
    verified_requirement_ids: list[str] = []
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
            dependency_ids = closure.get("depends_on")
            source_id_list = closure.get("source_ids")
            if (
                not isinstance(closure_id, str)
                or not isinstance(state, str)
                or not isinstance(dependency_ids, list)
                or not isinstance(source_id_list, list)
            ):
                raise EvidenceValidationError("validated ledger has a malformed closure requirement")
            blocked_by_requirement_ids = sorted(
                dependency_id
                for dependency_id in dependency_ids
                if closure_by_id[dependency_id]["state"] != "verified"
            )
            if state == "verified":
                verified_requirement_ids.append(closure_id)
            elif blocked_by_requirement_ids:
                blocked_requirement_ids.append(closure_id)
            else:
                ready_requirement_ids.append(closure_id)
            gate_states[state] += 1
            total_states[state] += 1
            requirements.append(
                {
                    "id": closure_id,
                    "state": state,
                    "dependency_ids": sorted(dependency_ids),
                    "blocked_by_requirement_ids": blocked_by_requirement_ids,
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
        "work_queue": {
            "topological_requirement_ids": topological_ids,
            "ready_requirement_ids": sorted(ready_requirement_ids),
            "blocked_requirement_ids": sorted(blocked_requirement_ids),
            "verified_requirement_ids": sorted(verified_requirement_ids),
        },
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
    parser.add_argument(
        "--pin-nets",
        type=Path,
        default=Path("docs/evidence/intellec/mod40_component_pin_net_v1.json"),
    )
    parser.add_argument("--report", type=Path, help="Write a deterministic gate-status report after validation.")
    parsed = parser.parse_args(arguments)
    try:
        with parsed.ledger.open(encoding="ascii") as handle:
            ledger = json.load(handle)
        with parsed.pin_nets.open(encoding="ascii") as handle:
            pin_ledger = json.load(handle)
        source_ids = load_source_ids(parsed.sources)
        validate_ledger(ledger, source_ids)
        if not isinstance(ledger, dict):
            raise EvidenceValidationError("route ledger root is not a mapping")
        validate_pin_net_ledger(pin_ledger, ledger, source_ids)
        if parsed.report is not None:
            write_status_report(parsed.report, build_status_report(ledger))
    except (EvidenceValidationError, OSError, json.JSONDecodeError, yaml.YAMLError) as error:
        print(f"MOD 40 evidence validation failed: {error}")
        return 1
    print(f"PASS MOD 40 route ledger: {parsed.ledger}")
    print(f"PASS MOD 40 component-pin ledger: {parsed.pin_nets}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
