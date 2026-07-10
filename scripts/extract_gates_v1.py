#!/usr/bin/env python3
"""Ratioed-logic gate extraction for single-transistor-type pMOS netlists.

The Intel 10um pMOS process used by the MCS-4 family builds every device from
one transistor polarity. Logic is RATIOED, not complementary: a driver network
of enhancement devices pulls the output toward one rail, and a single load
device (a saturated or depletion transistor tied to the opposite rail) holds
the output at the other rail. A complementary-CMOS extractor (see
extract_gates_v0.py) never finds an inverter here because there is no p/n pair
to match; that is why gates_v0 reports zero inverters on all four chips.

Recognition model (negative-logic rails: VDD near -15 V, VSS at 0 V):

  Load device
    Two physical forms occur in the extracted netlist_v1 data.
      Depletion saturated load: gate tied to one of its own channel terminals
        (gate_node == a_node or gate_node == b_node); the other terminal sits
        on a rail. The shared node is the gate OUTPUT Y.
      Enhancement saturated load: gate tied to a rail and one channel terminal
        tied to that same rail; the other channel terminal is the OUTPUT Y.
        This form dominates a single-transistor-type process (evidence: 4004
        transistors with gate_node == a_node == VCC and b_node == Y).

  INV   load on Y + exactly one enhancement driver, channel Y-to-rail, gate = A.
  NAND  load on Y + a SERIES chain of n drivers from Y to a rail; interior chain
        nodes touch exactly the two adjacent chain devices.
  NOR   load on Y + n PARALLEL drivers, each channel Y-to-rail, distinct gates.
  AOI   load on Y + a driver network that is series-parallel but neither a pure
        chain nor a pure parallel bundle; the driver-network structure is
        recorded.
  pass  enhancement device whose channel joins two non-rail nodes and whose gate
        is a non-rail node, not part of any recognized driver network, and whose
        channel actually connects to another device (not a dangling stub).

Rail identification: the netlist_v1 `signals` list carries VDD/VSS/VCC-style
names with a `layout_node`. Those layout nodes are trusted only when they carry
real channel incidence. When signals do not supply a usable rail pair, the
highest-channel-incidence node(s) stand in as rails (a rail metal line touches
far more transistor channels than any signal net). The method used is recorded
per chip in the manifest.

Known schema gap: the netlist_v1 transistors are all `kind == "pmos_candidate"`
(no Enhancement/Depletion label), and for three of four chips the power rails
are fragmented across many un-merged layout nodes rather than consolidated into
one net. Gate recognition can only fire where a rail is resolved in the channel
network; unresolved transistors are reported honestly as unclassified with the
reason recorded, never paired into spurious gates.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]

# A node counts as a rail by incidence only when it is a strong outlier: at
# least this many channel contacts AND a large fraction of the busiest node.
RAIL_INCIDENCE_FLOOR = 8
RAIL_INCIDENCE_FRACTION = 0.4

RAIL_NAME_TOKENS = ("VDD", "VSS", "VGG", "VCC", "GND")

# Signal-anchor names that must never be promoted to rails by incidence:
# clock spines carry high channel incidence but are not supply rails.
NON_RAIL_ANCHOR_TOKENS = ("CLK", "CLOCK", "PHI", "SYNC", "RESET", "TEST", "POC")


@dataclass
class Transistor:
    """One pMOS device from a netlist_v1 transistor list."""

    id: int
    gate: int
    a: int
    b: int

    def channel_nodes(self) -> tuple[int, int]:
        return (self.a, self.b)

    def other_channel(self, node: int) -> int:
        """Return the channel terminal that is not `node`."""
        return self.b if node == self.a else self.a


@dataclass
class Gate:
    """A recognized ratioed-logic gate."""

    gate_type: str
    output: int
    inputs: list[int]
    rail: int
    load_transistors: list[int]
    driver_transistors: list[int]
    structure: str
    confidence: float

    def all_transistors(self) -> list[int]:
        return sorted(set(self.load_transistors) | set(self.driver_transistors))


@dataclass
class ChipResult:
    chip: str
    total_transistors: int
    rails: list[int]
    rail_method: str
    rail_detail: dict[str, Any]
    rails_resolved: bool = False
    gates: list[Gate] = field(default_factory=list)
    load_devices: list[int] = field(default_factory=list)
    pass_transistors: list[int] = field(default_factory=list)
    unclassified: dict[str, list[int]] = field(default_factory=dict)


def load_transistors(
    netlist_path: Path,
) -> tuple[list[Transistor], list[dict[str, Any]], dict[int, int]]:
    with open(netlist_path, encoding="utf-8") as handle:
        data = json.load(handle)
    raw = data.get("devices", {}).get("transistors", [])
    transistors: list[Transistor] = []
    for index, entry in enumerate(raw):
        transistors.append(
            Transistor(
                id=entry.get("id", index),
                gate=entry["gate_node"],
                a=entry["a_node"],
                b=entry["b_node"],
            )
        )
    node_metal_area = {
        node["node"]: node.get("metal_area") or 0
        for node in data.get("nodes", [])
    }
    return transistors, data.get("signals", []), node_metal_area


def channel_incidence(transistors: list[Transistor]) -> Counter[int]:
    incidence: Counter[int] = Counter()
    for transistor in transistors:
        incidence[transistor.a] += 1
        incidence[transistor.b] += 1
    return incidence


def identify_rails(
    transistors: list[Transistor],
    signals: list[dict[str, Any]],
    node_metal_area: dict[int, int],
) -> tuple[set[int], str, dict[str, Any]]:
    """Return (rail nodes, method label, evidence detail).

    Rails come from netlist_v1 signal anchors named VDD/VSS/VCC/GND FIRST; the
    anchor's layout_node is trusted when it touches the transistor channel
    network. Incidence ranking is a fallback that augments an incomplete rail
    set, and it excludes any node carrying a named non-rail anchor (CLK*,
    CLOCK, PHI*, SYNC, RESET, TEST, POC): clock spines carry rail-scale channel
    incidence but are not supply nets. A node holding BOTH a rail anchor and a
    non-rail anchor (4004 node 415 carries CLK1 and VCC) is kept as a rail --
    the rail anchor plus outlier metal_area decide -- and the conflict is
    recorded in the detail block for downstream review.
    """
    incidence = channel_incidence(transistors)

    anchor_names: dict[int, list[str]] = {}
    for signal in signals:
        node = signal.get("layout_node")
        if node is None:
            continue
        anchor_names.setdefault(node, []).append(signal.get("name") or "")

    signal_rails: dict[str, tuple[int, int]] = {}
    non_rail_anchor_nodes: set[int] = set()
    anchor_conflicts: list[dict[str, Any]] = []
    for node, names in anchor_names.items():
        upper = [name.upper() for name in names]
        is_rail = any(
            any(token in name for token in RAIL_NAME_TOKENS) for name in upper
        )
        is_non_rail = any(
            any(token in name for token in NON_RAIL_ANCHOR_TOKENS)
            for name in upper
        )
        if is_rail:
            for name in upper:
                if any(token in name for token in RAIL_NAME_TOKENS):
                    signal_rails[name] = (node, incidence.get(node, 0))
            if is_non_rail:
                anchor_conflicts.append(
                    {
                        "node": node,
                        "names": names,
                        "resolution": "kept as rail (rail anchor wins)",
                        "metal_area": node_metal_area.get(node, 0),
                        "channel_incidence": incidence.get(node, 0),
                    }
                )
        elif is_non_rail:
            non_rail_anchor_nodes.add(node)

    usable_signal = {
        node for _, (node, inc) in signal_rails.items() if inc >= 1
    }

    max_incidence = max(incidence.values(), default=0)
    incidence_threshold = max(
        RAIL_INCIDENCE_FLOOR, int(RAIL_INCIDENCE_FRACTION * max_incidence)
    )
    incidence_rails = {
        node
        for node, inc in incidence.items()
        if inc >= incidence_threshold and node not in non_rail_anchor_nodes
    }

    detail: dict[str, Any] = {
        "signal_rails": {
            name: {
                "node": node,
                "channel_incidence": inc,
                "metal_area": node_metal_area.get(node, 0),
            }
            for name, (node, inc) in sorted(signal_rails.items())
        },
        "anchor_conflicts": anchor_conflicts,
        "excluded_non_rail_anchor_nodes": sorted(non_rail_anchor_nodes),
        "incidence_threshold": incidence_threshold,
        "incidence_rail_candidates": sorted(incidence_rails),
        "busiest_channel_nodes": [
            {
                "node": node,
                "channel_incidence": inc,
                "metal_area": node_metal_area.get(node, 0),
                "named_anchor": anchor_names.get(node),
            }
            for node, inc in incidence.most_common(5)
        ],
    }

    extra_incidence = incidence_rails - usable_signal
    if len(usable_signal) >= 2 and not extra_incidence:
        return usable_signal, "signals", detail
    if usable_signal and extra_incidence:
        return usable_signal | extra_incidence, "signals+incidence", detail
    if usable_signal:
        return usable_signal, "signals", detail
    if incidence_rails:
        return incidence_rails, "incidence", detail
    return set(), "none", detail


def find_loads(
    transistors: list[Transistor],
    rails: set[int],
) -> dict[int, list[Transistor]]:
    """Map each output node Y to the load device(s) that hold it to a rail.

    Depletion saturated load: gate == one channel terminal, the other terminal
    on a rail; Y is the shared (gate) node.
    Enhancement saturated load: gate on a rail with one channel terminal on that
    same rail; Y is the non-rail channel terminal.
    """
    loads_by_output: dict[int, list[Transistor]] = {}
    for transistor in transistors:
        gate, node_a, node_b = transistor.gate, transistor.a, transistor.b
        a_rail = node_a in rails
        b_rail = node_b in rails

        # Depletion saturated load: gate tied to its own channel terminal, the
        # other terminal on a rail; the shared node is the output.
        if gate == node_a and b_rail and not a_rail:
            loads_by_output.setdefault(node_a, []).append(transistor)
            continue
        if gate == node_b and a_rail and not b_rail:
            loads_by_output.setdefault(node_b, []).append(transistor)
            continue

        # Rail-biased load: gate on a rail, exactly one channel terminal on a
        # rail (the source rail, same or opposite), the other terminal is the
        # output Y. Covers the enhancement saturated load (gate on the source
        # rail) and the bias load whose gate sits on the opposite rail.
        if gate in rails and (a_rail != b_rail):
            output = node_b if a_rail else node_a
            loads_by_output.setdefault(output, []).append(transistor)

    return loads_by_output


def classify_driver_network(
    output: int,
    drivers: list[Transistor],
    rails: set[int],
    incidence: Counter[int],
) -> tuple[str, str, list[int]]:
    """Return (gate_type, structure, ordered driver ids) for the pull network.

    A recognized gate requires the pull network to TERMINATE at a rail: a single
    device output-to-rail is an inverter; several devices each output-to-rail
    form a NOR; a single series path output->...->rail is a NAND; any other
    connected shape that still reaches a rail is an AOI/OAI composite. A pull
    network that never reaches a rail is not a completed gate (the driven logic
    is unresolved in the netlist); the caller keeps the load as a load device.
    """
    if not drivers:
        return "OPEN", "no-driver", []

    direct_to_rail = [
        d for d in drivers if output in d.channel_nodes() and (
            d.other_channel(output) in rails
        )
    ]

    # Every driver ties the output straight to a rail: inverter or NOR.
    if direct_to_rail and len(direct_to_rail) == len(drivers):
        if len(drivers) == 1:
            return "INV", "single-driver", [drivers[0].id]
        return "NOR", f"parallel-{len(drivers)}", [d.id for d in drivers]

    # Attempt a single series chain: output -> n1 -> ... -> rail, every interior
    # node touched by exactly two chain devices (channel incidence 2).
    chain = _trace_series_chain(output, drivers, rails, incidence)
    if chain is not None:
        return "NAND", f"series-{len(chain)}", [d.id for d in chain]

    # Connected series-parallel shape recognized only when it reaches a rail.
    if _network_reaches_rail(drivers, rails):
        return "AOI", f"series-parallel-{len(drivers)}", [d.id for d in drivers]

    return "OPEN", "driver-network-no-rail", []


def _network_reaches_rail(drivers: list[Transistor], rails: set[int]) -> bool:
    """True when any driver in the network has a channel terminal on a rail."""
    for driver in drivers:
        if driver.a in rails or driver.b in rails:
            return True
    return False


def _trace_series_chain(
    output: int,
    drivers: list[Transistor],
    rails: set[int],
    incidence: Counter[int],
) -> list[Transistor] | None:
    """Return an ordered series chain from output to a rail, or None."""
    by_node: dict[int, list[Transistor]] = {}
    for driver in drivers:
        by_node.setdefault(driver.a, []).append(driver)
        by_node.setdefault(driver.b, []).append(driver)

    start = [d for d in drivers if output in d.channel_nodes()]
    if len(start) != 1:
        return None

    chain: list[Transistor] = []
    visited: set[int] = set()
    current_node = output
    current = start[0]
    while True:
        chain.append(current)
        visited.add(current.id)
        next_node = current.other_channel(current_node)
        if next_node in rails:
            return chain
        # Interior node must be touched by exactly two chain devices.
        if incidence.get(next_node, 0) != 2:
            return None
        candidates = [d for d in by_node.get(next_node, []) if d.id not in visited]
        if len(candidates) != 1:
            return None
        current_node = next_node
        current = candidates[0]
        if len(chain) > len(drivers):
            return None


def extract_gates(
    transistors: list[Transistor],
    rails: set[int],
) -> tuple[list[Gate], list[int], set[int]]:
    """Recognize ratioed gates.

    Return (gates, load_only device ids, claimed transistor ids). A load_only
    device is a saturated load whose output has no driver network that resolves
    to a rail: the load is a genuine structural signature (gates_v0 finds none),
    but the driven logic is not resolvable in netlist_v1.
    """
    incidence = channel_incidence(transistors)
    loads_by_output = find_loads(transistors, rails)

    load_ids: set[int] = {
        load.id for loads in loads_by_output.values() for load in loads
    }
    claimed: set[int] = set()
    gates: list[Gate] = []
    load_only: list[int] = []

    for output, loads in loads_by_output.items():
        # Drivers on this output: enhancement devices touching Y that are not
        # loads and whose gate is an input (not a rail bias, not the output).
        drivers = [
            t
            for t in transistors
            if t.id not in load_ids
            and t.id not in claimed
            and output in t.channel_nodes()
            and t.gate not in rails
            and t.gate != output
        ]
        # Extend the pull network across interior chain nodes so a NAND series
        # tail is available to the classifier.
        drivers = _grow_pull_network(
            output, drivers, transistors, rails, load_ids, claimed
        )

        gate_type, structure, driver_ids = classify_driver_network(
            output, drivers, rails, incidence
        )
        available_loads = [load for load in loads if load.id not in claimed]
        used_drivers = [d for d in drivers if d.id in set(driver_ids)]

        if gate_type == "OPEN" or not used_drivers:
            # No rail-terminated driver network: record the load device itself.
            for load in available_loads:
                load_only.append(load.id)
                claimed.add(load.id)
            continue

        if not available_loads:
            continue

        inputs = sorted({d.gate for d in used_drivers})
        gate = Gate(
            gate_type=gate_type,
            output=output,
            inputs=inputs,
            rail=next(iter(sorted(rails))),
            load_transistors=[load.id for load in available_loads],
            driver_transistors=[d.id for d in used_drivers],
            structure=structure,
            confidence=0.9 if gate_type in ("INV", "NOR", "NAND") else 0.6,
        )
        for tid in gate.all_transistors():
            claimed.add(tid)
        gates.append(gate)

    return gates, sorted(set(load_only)), claimed


def _grow_pull_network(
    output: int,
    seed_drivers: list[Transistor],
    transistors: list[Transistor],
    rails: set[int],
    load_ids: set[int],
    claimed: set[int],
) -> list[Transistor]:
    """Follow interior nodes to gather a full series/parallel pull network."""
    network: dict[int, Transistor] = {d.id: d for d in seed_drivers}
    frontier = list(seed_drivers)
    while frontier:
        current = frontier.pop()
        for node in current.channel_nodes():
            if node in rails or node == output:
                continue
            for candidate in transistors:
                if candidate.id in network or candidate.id in load_ids:
                    continue
                if candidate.id in claimed:
                    continue
                if candidate.gate in rails:
                    continue
                if node in candidate.channel_nodes():
                    network[candidate.id] = candidate
                    frontier.append(candidate)
    return list(network.values())


def find_pass_transistors(
    transistors: list[Transistor],
    rails: set[int],
    claimed: set[int],
    incidence: Counter[int],
    rails_resolved: bool,
) -> tuple[list[int], dict[str, list[int]]]:
    """Classify remaining devices as pass transistors or unclassified.

    A transmission device joins two non-rail nodes under a non-rail-signal gate,
    with BOTH channel terminals genuinely connected (channel incidence >= 2 on
    each end, not a dangling stub). This tight rule fires only when the rail net
    is resolved (a rail node carries strong channel incidence); on a fragmented
    netlist "non-rail" is not meaningful, so no pass device is claimed and the
    remaining transistors are reported unclassified with their reason. This
    avoids the spurious over-labeling that inflated the gates_v0 counts.
    """
    passes: list[int] = []
    isolated: list[int] = []
    gate_on_rail: list[int] = []
    rail_touching: list[int] = []
    fragmented: list[int] = []

    for transistor in transistors:
        if transistor.id in claimed:
            continue
        gate, node_a, node_b = transistor.gate, transistor.a, transistor.b

        if gate in rails:
            gate_on_rail.append(transistor.id)
            continue
        if node_a in rails or node_b in rails:
            rail_touching.append(transistor.id)
            continue

        both_connected = (
            incidence.get(node_a, 0) >= 2 and incidence.get(node_b, 0) >= 2
        )
        if not both_connected:
            isolated.append(transistor.id)
            continue

        if rails_resolved:
            passes.append(transistor.id)
        else:
            fragmented.append(transistor.id)

    unclassified = {
        "isolated_dangling": sorted(isolated),
        "gate_on_rail_bias_no_load": sorted(gate_on_rail),
        "rail_touching_role_unresolved": sorted(rail_touching),
        "fragmented_no_resolved_rail": sorted(fragmented),
    }
    return sorted(passes), unclassified


def analyze_chip(chip: str, netlist_path: Path) -> ChipResult:
    transistors, signals, node_metal_area = load_transistors(netlist_path)
    incidence = channel_incidence(transistors)
    rails, method, detail = identify_rails(transistors, signals, node_metal_area)

    rails_resolved = any(
        incidence.get(rail, 0) >= RAIL_INCIDENCE_FLOOR for rail in rails
    )

    gates, load_devices, claimed = extract_gates(transistors, rails)
    passes, unclassified = find_pass_transistors(
        transistors, rails, claimed, incidence, rails_resolved
    )

    return ChipResult(
        chip=chip,
        total_transistors=len(transistors),
        rails=sorted(rails),
        rail_method=method,
        rail_detail=detail,
        rails_resolved=rails_resolved,
        gates=gates,
        load_devices=load_devices,
        pass_transistors=passes,
        unclassified=unclassified,
    )


def gate_type_counts(result: ChipResult) -> dict[str, int]:
    counts = {
        "INV": 0,
        "NAND2": 0,
        "NAND3": 0,
        "NAND_other": 0,
        "NOR2": 0,
        "NOR3": 0,
        "NOR_other": 0,
        "AOI": 0,
    }
    for gate in result.gates:
        if gate.gate_type == "INV":
            counts["INV"] += 1
        elif gate.gate_type == "NAND":
            width = len(gate.driver_transistors)
            key = {2: "NAND2", 3: "NAND3"}.get(width, "NAND_other")
            counts[key] += 1
        elif gate.gate_type == "NOR":
            width = len(gate.driver_transistors)
            key = {2: "NOR2", 3: "NOR3"}.get(width, "NOR_other")
            counts[key] += 1
        elif gate.gate_type == "AOI":
            counts["AOI"] += 1
    return counts


def coverage_counts(result: ChipResult) -> dict[str, Any]:
    """Coverage split into confirmed structure vs role-unconfirmed devices.

    Confirmed structure = recognized gates + identified load devices; each is a
    hand-checkable ratioed-logic signature. The pass bucket is role-unconfirmed:
    on this fragmented netlist a device between two non-rail nodes could be a
    transmission gate OR an unresolved driver, and the two are locally
    indistinguishable. Reporting them separately keeps the pass column (required
    by the recognition model) from masquerading as recognized logic.
    """
    confirmed: set[int] = set()
    double = 0
    for gate in result.gates:
        for tid in gate.all_transistors():
            if tid in confirmed:
                double += 1
            confirmed.add(tid)
    for tid in result.load_devices:
        if tid in confirmed:
            double += 1
        confirmed.add(tid)

    all_claimed = set(confirmed)
    for tid in result.pass_transistors:
        if tid in all_claimed:
            double += 1
        all_claimed.add(tid)

    total = result.total_transistors
    return {
        "confirmed_structure_transistors": len(confirmed),
        "confirmed_coverage_pct": (
            round(100.0 * len(confirmed) / total, 1) if total else 0.0
        ),
        "role_unconfirmed_pass_transistors": len(result.pass_transistors),
        "classified_transistors": len(all_claimed),
        "coverage_pct": round(100.0 * len(all_claimed) / total, 1) if total else 0.0,
        "double_claims": double,
    }


def v0_coverage(chip: str, gates_v0_dir: Path, total: int) -> dict[str, Any]:
    path = gates_v0_dir / chip / f"{chip}_gates_v0.json"
    if not path.exists():
        return {"available": False}
    data = json.loads(path.read_text(encoding="utf-8"))
    claimed: set[int] = set()
    double = 0
    for gate in data.get("gates", []):
        for tid in gate.get("transistors", []):
            if tid in claimed:
                double += 1
            claimed.add(tid)
    return {
        "available": True,
        "total_gates": len(data.get("gates", [])),
        "distinct_transistors_claimed": len(claimed),
        "double_claims": double,
        "coverage_pct": round(100.0 * len(claimed) / total, 1) if total else 0.0,
    }


def result_to_json(result: ChipResult) -> dict[str, Any]:
    gates_json = [
        {
            "gate_type": gate.gate_type,
            "output": gate.output,
            "inputs": gate.inputs,
            "rail": gate.rail,
            "load_transistors": gate.load_transistors,
            "driver_transistors": gate.driver_transistors,
            "structure": gate.structure,
            "confidence": gate.confidence,
        }
        for gate in result.gates
    ]
    counts = gate_type_counts(result)
    coverage = coverage_counts(result)
    return {
        "schema_version": 1,
        "chip": result.chip,
        "description": "Ratioed-logic gate netlist (pMOS enhancement driver + saturated load)",
        "rails": result.rails,
        "rail_identification_method": result.rail_method,
        "rails_resolved": result.rails_resolved,
        "rail_detail": result.rail_detail,
        "gates": gates_json,
        "load_devices": result.load_devices,
        "pass_transistors": result.pass_transistors,
        "unclassified": result.unclassified,
        "statistics": {
            "total_transistors": result.total_transistors,
            "total_gates": len(result.gates),
            "gate_type_counts": counts,
            "load_devices": len(result.load_devices),
            "pass_transistors": len(result.pass_transistors),
            "unclassified_transistors": sum(
                len(v) for v in result.unclassified.values()
            ),
            "coverage": coverage,
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Extract ratioed-logic gates from pMOS transistor netlists"
    )
    parser.add_argument(
        "--chips",
        nargs="+",
        default=["4001", "4002", "4003", "4004"],
        help="Chips to process",
    )
    parser.add_argument(
        "--input-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "netlists_v1",
        help="Input netlist_v1 directory",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "gates_v1",
        help="Output directory for gate-level netlists",
    )
    parser.add_argument(
        "--gates-v0-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "gates_v0",
        help="gates_v0 directory for coverage comparison",
    )
    args = parser.parse_args()

    args.output_dir.mkdir(parents=True, exist_ok=True)

    print("=== Ratioed-Logic Gate Extraction (v1) ===")
    print("")

    manifest_chips: list[dict[str, Any]] = []

    for chip in args.chips:
        netlist_path = args.input_dir / f"{chip}_netlist_v1.json"
        if not netlist_path.exists():
            print(f"  netlist not found: {netlist_path}")
            continue

        result = analyze_chip(chip, netlist_path)
        chip_json = result_to_json(result)

        output_file = args.output_dir / chip / f"{chip}_gates_v1.json"
        output_file.parent.mkdir(parents=True, exist_ok=True)
        output_file.write_text(
            json.dumps(chip_json, indent=2) + "\n", encoding="utf-8"
        )

        counts = chip_json["statistics"]["gate_type_counts"]
        coverage = chip_json["statistics"]["coverage"]
        v0 = v0_coverage(chip, args.gates_v0_dir, result.total_transistors)

        manifest_chips.append(
            {
                "chip": chip,
                "total_transistors": result.total_transistors,
                "rails": result.rails,
                "rail_identification_method": result.rail_method,
                "rails_resolved": result.rails_resolved,
                "gate_type_counts": counts,
                "load_devices": len(result.load_devices),
                "pass_transistors": len(result.pass_transistors),
                "unclassified_transistors": chip_json["statistics"][
                    "unclassified_transistors"
                ],
                "unclassified_breakdown": {
                    reason: len(ids)
                    for reason, ids in result.unclassified.items()
                },
                "coverage_v1": coverage,
                "coverage_v0": v0,
            }
        )

        print(f"{chip}: {result.total_transistors} transistors")
        print(
            f"  rails={result.rails} method={result.rail_method} "
            f"resolved={result.rails_resolved}"
        )
        print(
            f"  INV={counts['INV']} "
            f"NAND2={counts['NAND2']} NAND3={counts['NAND3']} "
            f"NOR2={counts['NOR2']} NOR3={counts['NOR3']} AOI={counts['AOI']} "
            f"load_devices={len(result.load_devices)}"
        )
        print(
            f"  pass={len(result.pass_transistors)} "
            f"unclassified={chip_json['statistics']['unclassified_transistors']} "
            f"coverage_v1={coverage['coverage_pct']}% "
            f"double_claims={coverage['double_claims']}"
        )
        print("")

    manifest = {
        "schema_version": 1,
        "tool": "scripts/extract_gates_v1.py",
        "description": (
            "Ratioed-logic gate extraction manifest. Recognizes pMOS "
            "enhancement-driver + saturated-load gates. Coverage is reported "
            "against gates_v0 for comparison; gates_v0 coverage is inflated by "
            "double-claimed transistors (spurious pairwise NAND matches)."
        ),
        "chips": manifest_chips,
    }
    manifest_path = args.output_dir / "manifest.json"
    manifest_path.write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )
    print(f"Wrote manifest: {manifest_path}")


if __name__ == "__main__":
    main()
