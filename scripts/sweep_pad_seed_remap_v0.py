#!/usr/bin/env python3
"""
Sweep pad-derived seed nodes through the remap_anchors_to_incident_nodes_v1
selection logic.

Goal: help disambiguate which physical pad (layout_pads_v0) best seeds each
signal anchor when the metal mask lacks per-pin labels (e.g. 4001/4002/4003).

This script is intentionally deterministic and warning-clean (run with -W error).
"""

from __future__ import annotations

import argparse
import json
import re

# Reuse the remap algorithm (bbox scoring + incidence) rather than re-implementing.
import sys
from pathlib import Path
from typing import Any

# Allow importing sibling script modules when run from repo root.
SCRIPTS_DIR = Path(__file__).resolve().parent
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from remap_anchors_to_incident_nodes_v1 import (  # type: ignore  # noqa: E402
    _best_bbox,
    _build_incidence,
    _index_nodes,
    _pick_candidate,
)

ROOT = Path(__file__).resolve().parent.parent


def _load(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as f:
        data = json.load(f)
    if not isinstance(data, dict):
        raise SystemExit(f"expected object JSON: {path}")
    return data


def _pad_seed_nodes(layout_pads: dict[str, Any], *, k: int) -> list[dict[str, Any]]:
    pads = layout_pads.get("pads")
    if not isinstance(pads, list):
        raise SystemExit("layout_pads_v0: missing pads[]")
    out: list[dict[str, Any]] = []
    for p in pads:
        if not isinstance(p, dict):
            continue
        idx = p.get("idx_perimeter_ccw")
        edge = p.get("nearest_edge")
        bbox = p.get("bbox")
        suggested = p.get("suggested_nodes")
        if not isinstance(idx, int) or not isinstance(edge, str) or not isinstance(bbox, dict) or not isinstance(suggested, list):
            continue
        if not (0 <= idx <= 15):
            continue
        if not (0 <= int(k) < len(suggested)):
            continue
        s = suggested[int(k)]
        if not isinstance(s, dict) or not isinstance(s.get("node"), int):
            continue
        out.append(
            {
                "pad_idx_perimeter_ccw": int(idx),
                "edge": str(edge),
                "pad_bbox": {
                    "x0": int(bbox["x0"]),
                    "y0": int(bbox["y0"]),
                    "x1": int(bbox["x1"]),
                    "y1": int(bbox["y1"]),
                },
                "seed_node": int(s["node"]),
                "seed_node_uid": str(s.get("node_uid", "")),
                "seed_score": float(s.get("score", 0.0)),
                "seed_rect_distance": float(s.get("rect_distance", 0.0)),
            }
        )
    out.sort(key=lambda r: int(r["pad_idx_perimeter_ccw"]))
    return out


def main() -> None:
    ap = argparse.ArgumentParser(description="Sweep pad seeds through remap logic (v0).")
    ap.add_argument("--chip", required=True, choices=["4001", "4002", "4003", "4004"])
    ap.add_argument("--anchors", required=True, help="Anchors JSON (schematic_layout_anchors_v0.json or v1.json)")
    ap.add_argument("--netlist-v0", required=True, help="netlist_v0 JSON for the chip")
    ap.add_argument("--layout-pads-v0", required=True, help="layout_pads_v0 JSON for the chip")
    ap.add_argument("--out", required=True, help="Write JSON report here")
    ap.add_argument("--seed-k", type=int, default=0, help="Which suggested_nodes[k] to use as pad seed (default 0)")
    ap.add_argument(
        "--only",
        default="",
        help="Optional regex to filter anchors to sweep (e.g. '^(CLK1|CLK2|D[0-3]_PAD)$')",
    )
    # Keep defaults aligned with scripts/remap_anchors_to_incident_nodes_v1.py
    ap.add_argument("--max-dist", type=float, default=450.0)
    ap.add_argument("--min-incident", type=int, default=1)
    ap.add_argument("--area-ratio-weight", type=float, default=0.08)
    ap.add_argument("--prefer-gate-regex", default=r"^(CLK1|CLK2|SYNC)$")
    ap.add_argument("--max-dst-bbox-area", type=float, default=1_500_000.0)
    ap.add_argument("--allow-large-dst-regex", default=r"^(CLK1)$")
    ap.add_argument("--max-transistor-bbox-area", type=int, default=500_000)
    ap.add_argument("--max-transistor-bbox-dim", type=int, default=1200)
    args = ap.parse_args()

    chip = str(args.chip)
    anchors_path = (ROOT / args.anchors).resolve()
    net_path = (ROOT / args.netlist_v0).resolve()
    pads_path = (ROOT / args.layout_pads_v0).resolve()
    out_path = (ROOT / args.out).resolve()

    anchors = _load(anchors_path)
    net = _load(net_path)
    layout_pads = _load(pads_path)

    aroot = anchors.get("anchors")
    if not isinstance(aroot, dict) or not isinstance(aroot.get(chip), dict):
        raise SystemExit(f"anchors missing chip={chip}")
    block: dict[str, Any] = aroot[chip]

    only_re = re.compile(str(args.only)) if str(args.only).strip() else None

    nodes = _index_nodes(net)
    inc = _build_incidence(
        net=net,
        max_tx_bbox_area=int(args.max_transistor_bbox_area),
        max_tx_bbox_dim=int(args.max_transistor_bbox_dim),
    )
    candidate_nodes: list[tuple[int, dict[str, Any], dict[str, int]]] = []
    for node in sorted(nodes.keys()):
        ns = nodes[node]
        candidate_nodes.append((int(node), ns, inc.get(int(node), {"gate": 0, "terminal": 0, "total": 0})))

    pad_seeds = _pad_seed_nodes(layout_pads, k=int(args.seed_k))
    pad_seed_nodes = [p["seed_node"] for p in pad_seeds]
    if len(set(pad_seed_nodes)) < len(pad_seed_nodes):
        # Not fatal, but note it; the report includes pad_idx so humans can decide.
        pass

    results: dict[str, Any] = {
        "tool": "sweep_pad_seed_remap_v0",
        "schema": 1,
        "inputs": {
            "chip": chip,
            "anchors": str(Path(args.anchors)),
            "netlist_v0": str(Path(args.netlist_v0)),
            "layout_pads_v0": str(Path(args.layout_pads_v0)),
        },
        "params": {
            "seed_k": int(args.seed_k),
            "only": str(args.only),
            "max_dist": float(args.max_dist),
            "min_incident": int(args.min_incident),
            "area_ratio_weight": float(args.area_ratio_weight),
            "prefer_gate_regex": str(args.prefer_gate_regex),
            "max_dst_bbox_area": float(args.max_dst_bbox_area),
            "allow_large_dst_regex": str(args.allow_large_dst_regex),
            "max_transistor_bbox_area": int(args.max_transistor_bbox_area),
            "max_transistor_bbox_dim": int(args.max_transistor_bbox_dim),
        },
        "pads": pad_seeds,
        "anchors": {},
    }

    for name, row in sorted(block.items()):
        if not isinstance(name, str) or not isinstance(row, dict):
            continue
        if only_re and not only_re.search(name):
            continue

        anchor_rows: list[dict[str, Any]] = []
        for p in pad_seeds:
            seed = int(p["seed_node"])
            src_ns = nodes.get(seed)
            if not isinstance(src_ns, dict):
                continue
            src_bbox = _best_bbox(src_ns)
            if not isinstance(src_bbox, dict):
                continue
            dst_node, meta = _pick_candidate(
                src_bbox=src_bbox,
                candidates=candidate_nodes,
                max_dist=float(args.max_dist),
                min_incident=int(args.min_incident),
                area_ratio_weight=float(args.area_ratio_weight),
                prefer_gate=bool(re.match(str(args.prefer_gate_regex), str(name))),
                max_dst_bbox_area=None
                if re.match(str(args.allow_large_dst_regex), str(name))
                else float(args.max_dst_bbox_area),
            )
            dst_inc = None
            if dst_node is not None:
                dst_inc = inc.get(int(dst_node), {"gate": 0, "terminal": 0, "total": 0})
            anchor_rows.append(
                {
                    "pad_idx_perimeter_ccw": int(p["pad_idx_perimeter_ccw"]),
                    "edge": str(p["edge"]),
                    "seed_node": int(seed),
                    "dst_node": None if dst_node is None else int(dst_node),
                    "dst_incident": dst_inc,
                    **meta,
                }
            )

        results["anchors"][name] = {
            "current": {
                "layout_node": row.get("layout_node"),
                "layout_node_src": row.get("layout_node_src"),
            },
            "sweep": anchor_rows,
        }

    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("w", encoding="utf-8") as f:
        json.dump(results, f, indent=2, sort_keys=True)
        f.write("\n")

    print(f"wrote {out_path}")


if __name__ == "__main__":
    main()
