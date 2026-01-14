#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


def _load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _best_bbox(ns: dict[str, Any]) -> dict[str, int] | None:
    for k in ("metal_bbox", "poly_bbox", "diffusion_bbox"):
        b = ns.get(k)
        if isinstance(b, dict) and {"x0", "y0", "x1", "y1"} <= set(b.keys()):
            return {"x0": int(b["x0"]), "y0": int(b["y0"]), "x1": int(b["x1"]), "y1": int(b["y1"])}
    return None


def _bbox_area(bb: dict[str, int]) -> int:
    return max(0, int(bb["x1"]) - int(bb["x0"])) * max(0, int(bb["y1"]) - int(bb["y0"]))


def _edge_distance(bb: dict[str, int], *, w: int, h: int) -> int:
    return int(min(bb["x0"], bb["y0"], w - bb["x1"], h - bb["y1"]))


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Suggest candidate power-rail nodes (VSS/VDD/VCC) by edge proximity + terminal-heavy incidence (v0)."
    )
    ap.add_argument("--chip", required=True, choices=["4001", "4002", "4003", "4004"])
    ap.add_argument(
        "--netlist-v0",
        type=Path,
        default=None,
        help="netlist_v0 JSON (defaults under docs/evidence/netlists_v0/<chip>_netlist_v0.json).",
    )
    ap.add_argument("--top", type=int, default=30, help="How many candidates to emit.")
    ap.add_argument(
        "--out",
        type=Path,
        default=None,
        help="Write output JSON here (defaults under docs/evidence/power_rail_candidates_v0/).",
    )
    args = ap.parse_args()

    chip = str(args.chip)
    net_path = (
        (ROOT / args.netlist_v0).resolve()
        if args.netlist_v0 and not args.netlist_v0.is_absolute()
        else (args.netlist_v0 if args.netlist_v0 else ROOT / "docs" / "evidence" / "netlists_v0" / f"{chip}_netlist_v0.json")
    )
    net = _load(net_path)

    shape = net.get("inputs", {}).get("layout_shape", {})
    w, h = int(shape.get("w", 0) or 0), int(shape.get("h", 0) or 0)
    if w <= 0 or h <= 0:
        raise SystemExit("netlist missing inputs.layout_shape")

    # Incidence split: rails should be terminal-heavy and gate-light.
    inc: dict[int, dict[str, int]] = {}
    txs = net.get("devices", {}).get("transistors", []) if isinstance(net.get("devices"), dict) else []
    if not isinstance(txs, list):
        raise SystemExit("netlist missing devices.transistors[]")
    for t in txs:
        if not isinstance(t, dict):
            continue
        g = t.get("gate_node")
        a = t.get("a_node")
        b = t.get("b_node")
        if isinstance(g, int):
            d = inc.setdefault(int(g), {"gate": 0, "terminal": 0, "total": 0})
            d["gate"] += 1
            d["total"] += 1
        for n in (a, b):
            if isinstance(n, int):
                d = inc.setdefault(int(n), {"gate": 0, "terminal": 0, "total": 0})
                d["terminal"] += 1
                d["total"] += 1

    candidates: list[dict[str, Any]] = []
    for ns in net.get("node_stats", []) if isinstance(net.get("node_stats"), list) else []:
        if not isinstance(ns, dict) or not isinstance(ns.get("node"), int):
            continue
        node = int(ns["node"])
        bb = _best_bbox(ns)
        if not bb:
            continue
        area = _bbox_area(bb)
        if area <= 0:
            continue
        d = inc.get(node, {"gate": 0, "terminal": 0, "total": 0})
        edge = _edge_distance(bb, w=w, h=h)
        total = int(d.get("total", 0))
        gate = int(d.get("gate", 0))
        term = int(d.get("terminal", 0))
        term_ratio = (float(term) / float(total)) if total else 0.0
        candidates.append(
            {
                "node": node,
                "bbox": bb,
                "bbox_area": int(area),
                "edge_distance": int(edge),
                "incidence": {"total": total, "gate": gate, "terminal": term, "terminal_ratio": term_ratio},
            }
        )

    # Ranking heuristic: prioritize edge-adjacent, terminal-heavy, large area, low gate.
    candidates.sort(
        key=lambda r: (
            int(r["edge_distance"]),
            -int(r["incidence"]["terminal"]),
            -float(r["incidence"]["terminal_ratio"]),
            -int(r["bbox_area"]),
            int(r["incidence"]["gate"]),
            int(r["node"]),
        )
    )

    out_path = (
        (ROOT / args.out).resolve()
        if args.out and not args.out.is_absolute()
        else (
            args.out
            or (ROOT / "docs" / "evidence" / "power_rail_candidates_v0" / chip / f"{chip}_power_rail_candidates_v0.json")
        )
    )
    out_path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "chip": chip,
        "schema": {"version": 0, "description": "Heuristic-ranked candidate layout nodes for power rails (VSS/VDD/VCC)."},
        "tool": "scripts/suggest_power_rail_nodes_v0.py",
        "inputs": {"netlist_v0": str(net_path.relative_to(ROOT)) if net_path.is_relative_to(ROOT) else str(net_path)},
        "params": {"top": int(args.top)},
        "candidates": candidates[: int(args.top)],
    }
    out_path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(str(out_path.relative_to(ROOT)) if out_path.is_relative_to(ROOT) else str(out_path))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

