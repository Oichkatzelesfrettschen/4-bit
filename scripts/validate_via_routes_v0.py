#!/usr/bin/env python3
"""
Validate heuristic via-routing outputs against anchor-mapped layout nodes.

This script cross-checks `netlists_v2` via-node routing edges against
`schematic_layout_anchors_v1.json` node mappings and reports per-chip
coverage statistics.
"""

from __future__ import annotations

import argparse
import json
import re
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
PRIORITY_SIGNAL_RE = re.compile(
    r"(PAD|CLK|SYNC|RESET|CMROM|CMRAM|CM|TEST|POC|VDD|VSS|VCC|^D[0-3]$|^IO[0-3]$)"
)


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def resolve_netlist_v2_path(base_dir: Path, chip: str) -> Path:
    nested = base_dir / chip / f"{chip}_netlist_v2.json"
    if nested.exists():
        return nested
    flat = base_dir / f"{chip}_netlist_v2.json"
    if flat.exists():
        return flat
    raise FileNotFoundError(f"netlist_v2 not found for chip {chip} under {base_dir}")


def anchor_trace_nodes(anchor: dict[str, Any]) -> list[int]:
    nodes: set[int] = set()
    layout_node = anchor.get("layout_node")
    if isinstance(layout_node, int):
        nodes.add(layout_node)

    remap = anchor.get("remap_v1")
    if isinstance(remap, dict):
        dst_node = remap.get("dst_node")
        if isinstance(dst_node, int):
            nodes.add(dst_node)

    return sorted(nodes)


def summarize_chip(chip: str, chip_anchors: dict[str, Any], netlist_v2: dict[str, Any]) -> dict[str, Any]:
    net_nodes = {
        node.get("node")
        for node in netlist_v2.get("nodes", [])
        if isinstance(node, dict) and isinstance(node.get("node"), int)
    }

    node_via_edges: dict[int, int] = {}
    for via in netlist_v2.get("vias", []):
        if not isinstance(via, dict):
            continue
        connects_to = via.get("connects_to", [])
        if not isinstance(connects_to, list):
            continue
        for node_id in connects_to:
            if isinstance(node_id, int):
                node_via_edges[node_id] = node_via_edges.get(node_id, 0) + 1

    anchors_total = 0
    anchors_with_trace_nodes = 0
    anchors_trace_nodes_in_netlist = 0
    anchors_with_via_evidence = 0
    missing_trace_nodes_signals: list[str] = []
    no_via_evidence_signals: list[str] = []

    for signal, anchor in chip_anchors.items():
        if not isinstance(anchor, dict):
            continue
        anchors_total += 1
        trace_nodes = anchor_trace_nodes(anchor)
        if not trace_nodes:
            missing_trace_nodes_signals.append(signal)
            continue

        anchors_with_trace_nodes += 1
        present_nodes = [node_id for node_id in trace_nodes if node_id in net_nodes]
        if not present_nodes:
            missing_trace_nodes_signals.append(signal)
            continue

        anchors_trace_nodes_in_netlist += 1
        if any(node_via_edges.get(node_id, 0) > 0 for node_id in present_nodes):
            anchors_with_via_evidence += 1
        else:
            no_via_evidence_signals.append(signal)

    priority_missing_trace_nodes = sorted(
        signal for signal in missing_trace_nodes_signals if PRIORITY_SIGNAL_RE.search(signal)
    )
    priority_no_via_evidence = sorted(
        signal for signal in no_via_evidence_signals if PRIORITY_SIGNAL_RE.search(signal)
    )

    def ratio(numerator: int, denominator: int) -> float:
        if denominator == 0:
            return 0.0
        return float(numerator) / float(denominator)

    return {
        "chip": chip,
        "anchors_total": anchors_total,
        "anchors_with_trace_nodes": anchors_with_trace_nodes,
        "anchors_trace_nodes_in_netlist": anchors_trace_nodes_in_netlist,
        "anchors_with_via_evidence": anchors_with_via_evidence,
        "coverage_ratio_trace_nodes": ratio(anchors_with_trace_nodes, anchors_total),
        "coverage_ratio_in_netlist": ratio(anchors_trace_nodes_in_netlist, anchors_total),
        "coverage_ratio_via_evidence": ratio(anchors_with_via_evidence, anchors_total),
        "coverage_ratio_via_given_in_netlist": ratio(
            anchors_with_via_evidence, anchors_trace_nodes_in_netlist
        ),
        "signals_missing_trace_nodes": sorted(missing_trace_nodes_signals),
        "signals_without_via_evidence": sorted(no_via_evidence_signals),
        "priority_signals_missing_trace_nodes": priority_missing_trace_nodes,
        "priority_signals_without_via_evidence": priority_no_via_evidence,
        "netlist_statistics": {
            "total_nodes": len(net_nodes),
            "nodes_with_via_edges": len(node_via_edges),
            "total_vias": netlist_v2.get("statistics", {}).get("total_vias", 0),
            "routing_edges": netlist_v2.get("statistics", {}).get("routing_edges", 0),
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Validate via-route evidence against anchor mappings")
    parser.add_argument(
        "--anchors",
        type=Path,
        default=ROOT / "docs" / "evidence" / "schematic_layout_anchors_v1.json",
        help="Path to schematic_layout_anchors_v1.json",
    )
    parser.add_argument(
        "--netlists-v2-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "netlists_v2",
        help="Directory containing per-chip netlist_v2 outputs",
    )
    parser.add_argument(
        "--chips",
        nargs="+",
        default=["4001", "4002", "4003", "4004"],
        help="Chip ids to validate",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / "docs" / "evidence" / "netlists_v2" / "via_route_validation_summary.json",
        help="Output JSON summary path",
    )
    args = parser.parse_args()

    anchors_payload = load_json(args.anchors)
    anchor_root = anchors_payload.get("anchors", {})
    if not isinstance(anchor_root, dict):
        raise SystemExit("anchors payload missing top-level 'anchors' object")

    results: list[dict[str, Any]] = []
    for chip in args.chips:
        chip_anchors = anchor_root.get(chip, {})
        if not isinstance(chip_anchors, dict):
            raise SystemExit(f"missing anchors for chip {chip}")
        netlist_path = resolve_netlist_v2_path(args.netlists_v2_dir, chip)
        netlist_v2 = load_json(netlist_path)
        results.append(summarize_chip(chip, chip_anchors, netlist_v2))

    summary = {
        "schema_version": 1,
        "generated_at_utc": datetime.now(UTC).isoformat(),
        "source": "scripts/validate_via_routes_v0.py",
        "chips": results,
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")

    print("Via-route validation summary:")
    for chip_result in results:
        chip = chip_result["chip"]
        anchors = chip_result["anchors_total"]
        with_nodes = chip_result["anchors_trace_nodes_in_netlist"]
        with_via = chip_result["anchors_with_via_evidence"]
        ratio = chip_result["coverage_ratio_via_given_in_netlist"] * 100.0
        print(
            f"  {chip}: anchors={anchors}, trace_nodes_in_netlist={with_nodes}, "
            f"via_evidence={with_via}, via_given_in_netlist={ratio:.1f}%"
        )
    print(f"Wrote {args.output}")


if __name__ == "__main__":
    main()
