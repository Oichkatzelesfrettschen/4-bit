#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
import re
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


def _load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _rect_center(b: dict[str, Any]) -> tuple[float, float]:
    return (0.5 * (float(b["x0"]) + float(b["x1"])), 0.5 * (float(b["y0"]) + float(b["y1"])))


def _rect_distance(a: dict[str, Any], b: dict[str, Any]) -> float:
    ax0, ay0, ax1, ay1 = float(a["x0"]), float(a["y0"]), float(a["x1"]), float(a["y1"])
    bx0, by0, bx1, by1 = float(b["x0"]), float(b["y0"]), float(b["x1"]), float(b["y1"])
    dx = 0.0
    if ax1 < bx0:
        dx = bx0 - ax1
    elif bx1 < ax0:
        dx = ax0 - bx1
    dy = 0.0
    if ay1 < by0:
        dy = by0 - ay1
    elif by1 < ay0:
        dy = ay0 - by1
    return math.hypot(dx, dy)


def _best_bbox(ns: dict[str, Any]) -> dict[str, Any] | None:
    # Prefer metal bbox; fall back to poly/diffusion bboxes when metal is absent.
    for key in ("metal_bbox", "poly_bbox", "diffusion_bbox"):
        b = ns.get(key)
        if isinstance(b, dict) and {"x0", "y0", "x1", "y1"} <= set(b.keys()):
            return b
    return None


def _build_incidence(net: dict[str, Any]) -> dict[int, dict[str, int]]:
    out: dict[int, dict[str, int]] = {}
    devices = net.get("devices")
    if not isinstance(devices, dict):
        return out
    txs = devices.get("transistors")
    if not isinstance(txs, list):
        return out
    for tx in txs:
        if not isinstance(tx, dict):
            continue
        g = tx.get("gate_node")
        a = tx.get("a_node")
        b = tx.get("b_node")
        for node, kind in ((g, "gate"), (a, "terminal"), (b, "terminal")):
            if not isinstance(node, int):
                continue
            cur = out.setdefault(int(node), {"gate": 0, "terminal": 0, "total": 0})
            cur[kind] += 1
            cur["total"] += 1
    return out


def _index_nodes(net: dict[str, Any]) -> dict[int, dict[str, Any]]:
    stats = net.get("node_stats")
    if not isinstance(stats, list):
        return {}
    out: dict[int, dict[str, Any]] = {}
    for ns in stats:
        if not isinstance(ns, dict) or not isinstance(ns.get("node"), int):
            continue
        out[int(ns["node"])] = ns
    return out


def _pick_candidate(
    *,
    src_bbox: dict[str, Any],
    candidates: list[tuple[int, dict[str, Any], dict[str, int]]],
    max_dist: float,
    min_incident: int,
    area_ratio_weight: float,
    prefer_gate: bool,
    max_dst_bbox_area: float | None,
) -> tuple[int | None, dict[str, Any]]:
    best_node: int | None = None
    best_key: tuple[float, float, float, int, int, int] | None = None
    best_meta: dict[str, Any] = {"reason": "no_candidate"}
    sx, sy = _rect_center(src_bbox)
    sx0, sy0, sx1, sy1 = float(src_bbox["x0"]), float(src_bbox["y0"]), float(src_bbox["x1"]), float(src_bbox["y1"])
    src_area = max(0.0, sx1 - sx0) * max(0.0, sy1 - sy0)

    for node, ns, inc in candidates:
        total = int(inc.get("total", 0))
        gate = int(inc.get("gate", 0))
        if total < int(min_incident):
            continue
        bb = _best_bbox(ns)
        if not isinstance(bb, dict):
            continue
        bx0, by0, bx1, by1 = float(bb["x0"]), float(bb["y0"]), float(bb["x1"]), float(bb["y1"])
        bbox_area = max(0.0, bx1 - bx0) * max(0.0, by1 - by0)
        if max_dst_bbox_area is not None and bbox_area > float(max_dst_bbox_area):
            continue
        rect_d = _rect_distance(src_bbox, bb)
        if rect_d > float(max_dist):
            continue
        cx, cy = _rect_center(bb)
        center_d = math.hypot(cx - sx, cy - sy)
        area_ratio = bbox_area / max(1.0, src_area)
        penalty = float(area_ratio_weight) * float(area_ratio)
        cost = float(rect_d) + penalty
        # Prefer locality first (avoid huge overlapping power buses), then device incidence.
        gate_priority = -gate if prefer_gate else 0
        key = (float(cost), float(center_d), float(bbox_area), int(gate_priority), -total, int(node))
        if best_key is None or key < best_key:
            best_key = key
            best_node = int(node)
            best_meta = {
                "rect_distance": float(rect_d),
                "center_distance": float(center_d),
                "dst_bbox_area": float(bbox_area),
                "dst_area_ratio": float(area_ratio),
                "dst_area_penalty": float(penalty),
                "cost": float(cost),
                "incident": {"total": total, "gate": gate, "terminal": int(inc.get("terminal", 0))},
                "prefer_gate": bool(prefer_gate),
            }
    return best_node, best_meta


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Remap anchor layout nodes to nearby transistor-incident nodes (v1)."
    )
    ap.add_argument("--chip", default="4004", help="Chip number (4001/4002/4003/4004)")
    ap.add_argument("--anchors", type=Path, required=True, help="Anchors JSON (schematic_layout_anchors_v0.json)")
    ap.add_argument("--netlist-v0", type=Path, required=True, help="netlist_v0 JSON (node_stats + devices)")
    ap.add_argument("--out", type=Path, required=True, help="Write anchors JSON here (v1 output)")
    ap.add_argument(
        "--anchor-bboxes",
        type=Path,
        default=None,
        help="Optional anchor label bbox map JSON; used as remap seed instead of source node bbox when present",
    )
    ap.add_argument("--max-dist", type=float, default=450.0, help="Max bbox-to-bbox distance to search (pixels)")
    ap.add_argument("--min-incident", type=int, default=1, help="Require at least this many incident transistor endpoints")
    ap.add_argument(
        "--area-ratio-weight",
        type=float,
        default=0.08,
        help="Penalty weight for candidate bbox_area / src_bbox_area (discourages huge power/bus nets)",
    )
    ap.add_argument(
        "--prefer-gate-regex",
        default=r"^(CLK1|CLK2|SYNC)$",
        help="Regex of anchor names that should prefer gate incidence when remapping",
    )
    ap.add_argument(
        "--max-dst-bbox-area",
        type=float,
        default=1_500_000.0,
        help="Skip candidate nodes whose bbox area exceeds this (unless allow-large regex matches)",
    )
    ap.add_argument(
        "--allow-large-dst-regex",
        default=r"^(CLK1)$",
        help="Regex of anchor names allowed to remap onto very large bbox nodes (e.g., clock distribution)",
    )
    args = ap.parse_args()

    chip = str(args.chip).strip()
    anchors_path = (ROOT / args.anchors).resolve() if not args.anchors.is_absolute() else args.anchors
    net_path = (ROOT / args.netlist_v0).resolve() if not args.netlist_v0.is_absolute() else args.netlist_v0

    anchors = _load(anchors_path)
    net = _load(net_path)

    anchor_bboxes: dict[str, dict[str, int]] = {}
    if args.anchor_bboxes:
        bb_path = (ROOT / args.anchor_bboxes).resolve() if not args.anchor_bboxes.is_absolute() else args.anchor_bboxes
        bb = _load(bb_path)
        broot = bb.get("anchors")
        if isinstance(broot, dict) and isinstance(broot.get(chip), dict):
            for k, v in broot[chip].items():
                if not isinstance(k, str) or not isinstance(v, dict):
                    continue
                if {"x0", "y0", "x1", "y1"} <= set(v.keys()):
                    anchor_bboxes[str(k)] = {"x0": int(v["x0"]), "y0": int(v["y0"]), "x1": int(v["x1"]), "y1": int(v["y1"])}

    aroot = anchors.get("anchors")
    if not isinstance(aroot, dict) or not isinstance(aroot.get(chip), dict):
        raise SystemExit(f"anchors missing chip={chip}")

    nodes = _index_nodes(net)
    inc = _build_incidence(net)

    # Candidate list is deterministic: sorted by node id.
    candidate_nodes: list[tuple[int, dict[str, Any], dict[str, int]]] = []
    for node in sorted(nodes.keys()):
        ns = nodes[node]
        candidate_nodes.append((int(node), ns, inc.get(int(node), {"gate": 0, "terminal": 0, "total": 0})))

    remapped = json.loads(json.dumps(anchors))  # deep copy via json
    block = remapped["anchors"][chip]

    for name, row in block.items():
        if not isinstance(row, dict):
            continue
        src_node = row.get("layout_node")
        if not isinstance(src_node, int):
            continue
        src_ns = nodes.get(int(src_node))
        if not isinstance(src_ns, dict):
            row["remap_v1"] = {"ok": False, "reason": "missing_src_node_stats", "src_node": int(src_node)}
            continue
        seed_bbox = anchor_bboxes.get(str(name))
        if isinstance(seed_bbox, dict):
            src_bbox = seed_bbox
            seed_source = "anchor_bboxes"
        else:
            src_bbox = _best_bbox(src_ns)
            seed_source = "src_node_bbox"
        if not isinstance(src_bbox, dict):
            row["remap_v1"] = {"ok": False, "reason": "missing_src_bbox", "src_node": int(src_node)}
            continue

        src_inc = inc.get(int(src_node), {"gate": 0, "terminal": 0, "total": 0})
        if int(src_inc.get("total", 0)) >= int(args.min_incident):
            row["remap_v1"] = {
                "ok": True,
                "reason": "already_incident",
                "src_node": int(src_node),
                "dst_node": int(src_node),
                "incident": {
                    "total": int(src_inc.get("total", 0)),
                    "gate": int(src_inc.get("gate", 0)),
                    "terminal": int(src_inc.get("terminal", 0)),
                },
            }
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
        if dst_node is None:
            row["remap_v1"] = {"ok": False, "reason": "no_incident_candidate_within_max_dist", "src_node": int(src_node)}
            continue

        dst_ns = nodes.get(int(dst_node), {})
        dst_uid = dst_ns.get("node_uid") if isinstance(dst_ns, dict) else None

        row["layout_node_src"] = int(src_node)
        row["layout_node"] = int(dst_node)
        if isinstance(dst_uid, str):
            row["layout_node_uid"] = str(dst_uid)
        row["remap_v1"] = {
            "ok": True,
            "reason": "nearest_incident",
            "src_node": int(src_node),
            "src_node_uid": src_ns.get("node_uid"),
            "dst_node": int(dst_node),
            "dst_node_uid": dst_uid,
            "seed_source": str(seed_source),
            "max_dist": float(args.max_dist),
            "min_incident": int(args.min_incident),
            "area_ratio_weight": float(args.area_ratio_weight),
            "prefer_gate_regex": str(args.prefer_gate_regex),
            "max_dst_bbox_area": float(args.max_dst_bbox_area),
            "allow_large_dst_regex": str(args.allow_large_dst_regex),
            "src_bbox": src_bbox,
            **meta,
        }

    out = args.out
    if not out.is_absolute():
        out = (ROOT / out).resolve()
    out.parent.mkdir(parents=True, exist_ok=True)
    remapped["notes"] = list(remapped.get("notes") or [])
    if isinstance(remapped["notes"], list):
        remapped["notes"].append(
            {
                "kind": "anchors_v1",
                "chip": chip,
                "netlist_v0": str(net_path.relative_to(ROOT)) if net_path.is_relative_to(ROOT) else str(net_path),
                "method": "remap_anchors_to_incident_nodes_v1",
                "max_dist": float(args.max_dist),
                "min_incident": int(args.min_incident),
                "area_ratio_weight": float(args.area_ratio_weight),
                "prefer_gate_regex": str(args.prefer_gate_regex),
                "max_dst_bbox_area": float(args.max_dst_bbox_area),
                "allow_large_dst_regex": str(args.allow_large_dst_regex),
            }
        )
    out.write_text(json.dumps(remapped, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(str(out))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
