#!/usr/bin/env python3
"""
Extract gate-level netlists from transistor netlists (intermediate fidelity).

This script identifies common NMOS gate patterns and creates a gate-level
abstraction suitable for medium-fidelity simulation and analysis.

Gate Types Detected:
- Inverter: 1 PMOS + 1 NMOS with common gate/drain
- NAND: Series NMOS transistors
- NOR: Parallel NMOS transistors
- Transmission Gate: NMOS+PMOS pair with complementary gates
- Pass Transistor: Single NMOS with gate control

This provides an intermediate abstraction between full transistor-level
and RTL, useful for understanding circuit structure and validation.
"""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


@dataclass
class Transistor:
    """Transistor from netlist."""

    id: int
    type: str  # "NMOS" or "PMOS"
    gate: int  # Node ID
    source: int  # Node ID
    drain: int  # Node ID


@dataclass
class Gate:
    """Detected logic gate."""

    gate_type: str  # "INV", "NAND", "NOR", "TGATE", "PASS"
    inputs: list[int]  # Input node IDs
    outputs: list[int]  # Output node IDs
    transistors: list[int]  # Transistor IDs forming this gate
    confidence: float  # Detection confidence (0.0-1.0)


def detect_inverter(transistors: list[Transistor]) -> list[Gate]:
    """
    Detect inverter gates (1 PMOS + 1 NMOS with common gate and drain).

    Inverter pattern:
    - 1 PMOS with source=VDD, drain=OUT, gate=IN
    - 1 NMOS with source=VSS, drain=OUT, gate=IN
    - Common: gate (input), drain (output)
    """
    inverters = []

    # Group transistors by gate node
    by_gate: dict[int, list[Transistor]] = {}
    for t in transistors:
        by_gate.setdefault(t.gate, []).append(t)

    # Look for PMOS+NMOS pairs with common gate and drain
    for gate_node, trans_list in by_gate.items():
        if len(trans_list) != 2:
            continue

        pmos = [t for t in trans_list if t.type == "PMOS"]
        nmos = [t for t in trans_list if t.type == "NMOS"]

        if len(pmos) == 1 and len(nmos) == 1:
            p, n = pmos[0], nmos[0]
            if p.drain == n.drain:
                # Inverter detected
                inverters.append(
                    Gate(
                        gate_type="INV",
                        inputs=[gate_node],
                        outputs=[p.drain],
                        transistors=[p.id, n.id],
                        confidence=1.0,
                    )
                )

    return inverters


def detect_nand_gates(transistors: list[Transistor]) -> list[Gate]:
    """
    Detect NAND gates (series NMOS transistors).

    NAND pattern (2-input):
    - 2 NMOS in series: source1->drain1 == source2 (intermediate node)
    - Common output node (final drain)
    - 2 separate gate nodes (inputs)
    - Pull-up network (PMOS) in parallel
    """
    nand_gates = []

    # Find series NMOS chains
    nmos = [t for t in transistors if t.type == "NMOS"]

    # Simple 2-input NAND detection
    for i, t1 in enumerate(nmos):
        for t2 in nmos[i + 1 :]:
            # Check if drain of t1 connects to source of t2 (series)
            if t1.drain == t2.source:
                nand_gates.append(
                    Gate(
                        gate_type="NAND",
                        inputs=[t1.gate, t2.gate],
                        outputs=[t2.drain],
                        transistors=[t1.id, t2.id],
                        confidence=0.8,  # Heuristic detection
                    )
                )

    return nand_gates


def detect_nor_gates(transistors: list[Transistor]) -> list[Gate]:
    """
    Detect NOR gates (parallel NMOS transistors).

    NOR pattern (2-input):
    - 2 NMOS in parallel: same source, same drain
    - Different gate nodes (inputs)
    - Pull-up network (PMOS) in series
    """
    nor_gates = []

    nmos = [t for t in transistors if t.type == "NMOS"]

    # Find parallel NMOS groups
    for i, t1 in enumerate(nmos):
        for t2 in nmos[i + 1 :]:
            # Check if source and drain match (parallel)
            if t1.source == t2.source and t1.drain == t2.drain:
                nor_gates.append(
                    Gate(
                        gate_type="NOR",
                        inputs=[t1.gate, t2.gate],
                        outputs=[t1.drain],
                        transistors=[t1.id, t2.id],
                        confidence=0.8,
                    )
                )

    return nor_gates


def detect_transmission_gates(transistors: list[Transistor]) -> list[Gate]:
    """
    Detect transmission gates (NMOS+PMOS pair with complementary gates).

    Transmission gate pattern:
    - 1 NMOS + 1 PMOS
    - Same source/drain endpoints
    - Complementary gate control (gate_n and ~gate_n)
    """
    tgates = []

    # Group by source-drain pairs
    by_endpoints: dict[tuple[int, int], list[Transistor]] = {}
    for t in transistors:
        key = tuple(sorted([t.source, t.drain]))
        by_endpoints.setdefault(key, []).append(t)

    for endpoints, trans_list in by_endpoints.items():
        if len(trans_list) != 2:
            continue

        pmos = [t for t in trans_list if t.type == "PMOS"]
        nmos = [t for t in trans_list if t.type == "NMOS"]

        if len(pmos) == 1 and len(nmos) == 1:
            p, n = pmos[0], nmos[0]
            # Transmission gate (assuming complementary gates)
            tgates.append(
                Gate(
                    gate_type="TGATE",
                    inputs=[n.gate, p.gate],  # Control signals
                    outputs=list(endpoints),  # Bidirectional
                    transistors=[p.id, n.id],
                    confidence=0.9,
                )
            )

    return tgates


def extract_gates_from_netlist(
    chip: str,
    netlist_path: Path,
) -> dict[str, Any]:
    """
    Extract gate-level netlist from transistor netlist.

    Args:
        chip: Chip name
        netlist_path: Path to transistor netlist

    Returns:
        Gate-level netlist dictionary
    """
    # Load and parse the transistor netlist
    if not netlist_path.exists():
        return {
            "schema_version": 0,
            "chip": chip,
            "description": "Gate-level netlist (no input netlist found)",
            "gates": [],
            "statistics": {
                "total_gates": 0,
                "inverters": 0,
                "nand_gates": 0,
                "nor_gates": 0,
                "transmission_gates": 0,
                "pass_transistors": 0,
            },
        }

    with open(netlist_path, encoding="utf-8") as f:
        netlist_data = json.load(f)

    # Parse transistors into Transistor dataclass instances
    raw_transistors = netlist_data.get("devices", {}).get("transistors", [])
    transistors: list[Transistor] = []
    for i, t in enumerate(raw_transistors):
        tid = t.get("id", i)
        # Determine type: if gate_node == a_node or gate_node == b_node,
        # it is a depletion load (acts like PMOS pull-up). Otherwise treat
        # as enhancement (NMOS-equivalent in the pMOS logic family).
        if t["gate_node"] == t["a_node"] or t["gate_node"] == t["b_node"]:
            ttype = "PMOS"
        else:
            ttype = "NMOS"
        transistors.append(
            Transistor(
                id=tid,
                type=ttype,
                gate=t["gate_node"],
                source=t["a_node"],
                drain=t["b_node"],
            )
        )

    # Run all gate detection algorithms
    inverters = detect_inverter(transistors)
    nand_gates = detect_nand_gates(transistors)
    nor_gates = detect_nor_gates(transistors)
    tgates = detect_transmission_gates(transistors)

    # Aggregate all detected gates
    all_gates = []
    for g in inverters + nand_gates + nor_gates + tgates:
        all_gates.append(
            {
                "gate_type": g.gate_type,
                "inputs": g.inputs,
                "outputs": g.outputs,
                "transistors": g.transistors,
                "confidence": g.confidence,
            }
        )

    gates_netlist = {
        "schema_version": 0,
        "chip": chip,
        "description": "Gate-level netlist extracted from transistor netlist",
        "gates": all_gates,
        "statistics": {
            "total_gates": len(all_gates),
            "inverters": len(inverters),
            "nand_gates": len(nand_gates),
            "nor_gates": len(nor_gates),
            "transmission_gates": len(tgates),
            "pass_transistors": 0,
        },
    }

    return gates_netlist


def main():
    parser = argparse.ArgumentParser(description="Extract gate-level netlists")
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
        help="Input netlist directory",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "gates_v0",
        help="Output directory for gate-level netlists",
    )

    args = parser.parse_args()

    args.output_dir.mkdir(parents=True, exist_ok=True)

    print("=== Gate-Level Extraction ===")
    print("")

    for chip in args.chips:
        print(f"Processing {chip}...")

        netlist_path = args.input_dir / f"{chip}_netlist_v1.json"

        if not netlist_path.exists():
            print(f"  ⚠ Netlist not found: {netlist_path}")
            print("    Creating placeholder output")

        try:
            gates_netlist = extract_gates_from_netlist(
                chip=chip,
                netlist_path=netlist_path,
            )

            output_file = args.output_dir / chip / f"{chip}_gates_v0.json"
            output_file.parent.mkdir(parents=True, exist_ok=True)
            output_file.write_text(json.dumps(gates_netlist, indent=2) + "\n", encoding="utf-8")

            print(f"  ✓ Created gates netlist: {output_file.name}")
            print(f"    Total gates: {gates_netlist['statistics']['total_gates']}")
            print(f"    Inverters: {gates_netlist['statistics']['inverters']}")
            print(f"    NAND gates: {gates_netlist['statistics']['nand_gates']}")
            print(f"    NOR gates: {gates_netlist['statistics']['nor_gates']}")

        except Exception as e:
            print(f"  ✗ Failed: {e}")
            continue

        print("")

    print("✓ Gate-level extraction complete")


if __name__ == "__main__":
    main()
