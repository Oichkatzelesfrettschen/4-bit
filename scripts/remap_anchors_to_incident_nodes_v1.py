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


def _parse_pad_pin_template_bboxes(md_text: str) -> dict[int, dict[str, int]]:
    """
    Parse (pad_idx -> bbox) from pad_pin_template_v0.md.

    We use these bboxes as remap seeds because the template's pad_idx ordering is
    "perimeter CCW" and may not match the raw index order in layout_pads_v0 JSON.
    """
    out: dict[int, dict[str, int]] = {}
    in_table = False
    for raw in md_text.splitlines():
        line = raw.strip()
        if not line:
            if in_table and out:
                break
            continue
        if line.startswith("| pad_idx |"):
            in_table = True
            continue
        if not in_table:
            continue
        if line.startswith("|---"):
            continue
        if not (line.startswith("|") and line.endswith("|")):
            continue
        parts = [p.strip() for p in line.strip("|").split("|")]
        if len(parts) < 3:
            continue
        pad_idx_s, _edge, bbox_s = parts[:3]
        try:
            pad_idx = int(pad_idx_s)
        except ValueError:
            continue
        m = re.match(r"^\((\d+),(\d+),(\d+),(\d+)\)$", bbox_s)
        if not m:
            continue
        x0, y0, x1, y1 = (int(m.group(1)), int(m.group(2)), int(m.group(3)), int(m.group(4)))
        out[int(pad_idx)] = {"x0": x0, "y0": y0, "x1": x1, "y1": y1}
    return out


def _bbox_area_wh(bb: dict[str, Any] | None) -> tuple[int, int, int]:
    if not isinstance(bb, dict):
        return 0, 0, 0
    w = int(bb.get("w", 0) or 0)
    h = int(bb.get("h", 0) or 0)
    return w * h, w, h


def _build_incidence(*, net: dict[str, Any], max_tx_bbox_area: int, max_tx_bbox_dim: int) -> dict[int, dict[str, int]]:
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
        # Keep incidence aligned with build_netlist_v1 filters; ignore obviously broken transistor bboxes.
        area, w, h = _bbox_area_wh(tx.get("bbox"))
        if area > int(max_tx_bbox_area) or w > int(max_tx_bbox_dim) or h > int(max_tx_bbox_dim):
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
    forbidden_nodes: set[int],
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
        if int(node) in forbidden_nodes:
            continue
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
    ap.add_argument(
        "--pads-v0",
        type=Path,
        default=None,
        help="Optional layout_pads_v0 JSON; when present and anchors were seeded via pad_pin_template_v0, use pad bboxes as remap seeds and enforce unique dst nodes per pad.",
    )
    ap.add_argument(
        "--pad-max-dst-bbox-area",
        type=float,
        default=120_000.0,
        help="When seeding from a pad bbox, further cap candidate bbox area to avoid mapping onto large power/bus nets.",
    )
    ap.add_argument(
        "--max-transistor-bbox-area",
        type=int,
        default=500_000,
        help="Ignore transistor candidates with bbox area larger than this when computing node incidence (px^2).",
    )
    ap.add_argument(
        "--max-transistor-bbox-dim",
        type=int,
        default=1200,
        help="Ignore transistor candidates with bbox width/height larger than this when computing node incidence (px).",
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
    inc = _build_incidence(
        net=net,
        max_tx_bbox_area=int(args.max_transistor_bbox_area),
        max_tx_bbox_dim=int(args.max_transistor_bbox_dim),
    )

    pad_bboxes: dict[int, dict[str, int]] = {}
    if args.pads_v0:
        pads_path = (ROOT / args.pads_v0).resolve() if not args.pads_v0.is_absolute() else args.pads_v0
        pads = _load(pads_path)
        plist = pads.get("pads")
        if not isinstance(plist, list):
            raise SystemExit(f"expected pads list: {pads_path}")
        for idx, p in enumerate(plist):
            if not isinstance(p, dict):
                continue
            bb = p.get("bbox")
            if not isinstance(bb, dict):
                continue
            if {"x0", "y0", "x1", "y1"} <= set(bb.keys()):
                pad_bboxes[int(idx)] = {"x0": int(bb["x0"]), "y0": int(bb["y0"]), "x1": int(bb["x1"]), "y1": int(bb["y1"])}

    template_bbox_cache: dict[str, dict[int, dict[str, int]]] = {}

    # Candidate list is deterministic: sorted by node id.
    candidate_nodes: list[tuple[int, dict[str, Any], dict[str, int]]] = []
    for node in sorted(nodes.keys()):
        ns = nodes[node]
        candidate_nodes.append((int(node), ns, inc.get(int(node), {"gate": 0, "terminal": 0, "total": 0})))

    remapped = json.loads(json.dumps(anchors))  # deep copy via json
    block = remapped["anchors"][chip]

    # Group pad-seeded anchors (pad_pin_template_v0) by pad_idx; these should remap to a
    # unique dst_node per pad (aliases are allowed within the same pad_idx).
    pad_groups: dict[int, list[str]] = {}
    for name, row in block.items():
        if not isinstance(row, dict):
            continue
        seed = row.get("layout_seed_v0")
        if not isinstance(seed, dict) or seed.get("kind") != "pad_pin_template_v0":
            continue
        pad_idx = seed.get("pad_idx")
        if isinstance(pad_idx, int):
            pad_groups.setdefault(int(pad_idx), []).append(str(name))

    used_dst_nodes: set[int] = set()
    pad_group_dst: dict[int, int] = {}

    def _remap_one(*, name: str, row: dict[str, Any], forbidden: set[int]) -> tuple[int | None, dict[str, Any]]:
        src_node = row.get("layout_node")
        if not isinstance(src_node, int):
            return None, {"ok": False, "reason": "missing_src_node", "src_node": None}

        src_ns = nodes.get(int(src_node))
        src_uid = src_ns.get("node_uid") if isinstance(src_ns, dict) else None

        seed_bbox = anchor_bboxes.get(str(name))
        seed_source = "src_node_bbox"
        src_bbox: dict[str, Any] | None = None

        seed = row.get("layout_seed_v0")
        if isinstance(seed, dict) and seed.get("kind") == "pad_pin_template_v0" and isinstance(seed.get("pad_idx"), int):
            tpl_s = seed.get("template")
            tpl_path: Path | None = None
            if isinstance(tpl_s, str) and tpl_s.strip():
                tpl_path = (ROOT / tpl_s).resolve()
            if tpl_path and tpl_path.exists():
                cache_key = str(tpl_path)
                if cache_key not in template_bbox_cache:
                    template_bbox_cache[cache_key] = _parse_pad_pin_template_bboxes(tpl_path.read_text(encoding="utf-8"))
                bb = template_bbox_cache[cache_key].get(int(seed["pad_idx"]))
                if isinstance(bb, dict):
                    src_bbox = bb
                    seed_source = "pad_pin_template_bbox"
            if src_bbox is None:
                pb = pad_bboxes.get(int(seed["pad_idx"]))
                if isinstance(pb, dict):
                    src_bbox = pb
                    seed_source = "layout_pads_v0_index_bbox"
        if src_bbox is None and isinstance(seed_bbox, dict):
            src_bbox = seed_bbox
            seed_source = "anchor_bboxes"
        if src_bbox is None and isinstance(src_ns, dict):
            src_bbox = _best_bbox(src_ns)
            seed_source = "src_node_bbox"
        if not isinstance(src_bbox, dict):
            return None, {"ok": False, "reason": "missing_src_bbox", "src_node": int(src_node)}

        src_inc = inc.get(int(src_node), {"gate": 0, "terminal": 0, "total": 0})
        if int(src_inc.get("total", 0)) >= int(args.min_incident):
            return int(src_node), {
                "ok": True,
                "reason": "already_incident",
                "src_node": int(src_node),
                "src_node_uid": src_uid,
                "dst_node": int(src_node),
                "incident": {
                    "total": int(src_inc.get("total", 0)),
                    "gate": int(src_inc.get("gate", 0)),
                    "terminal": int(src_inc.get("terminal", 0)),
                },
                "seed_source": str(seed_source),
                "src_bbox": src_bbox,
            }

        max_dst_bbox_area = None if re.match(str(args.allow_large_dst_regex), str(name)) else float(args.max_dst_bbox_area)
        if seed_source in ("pad_pin_template_bbox", "layout_pads_v0_index_bbox"):
            max_dst_bbox_area = min(float(max_dst_bbox_area or float("inf")), float(args.pad_max_dst_bbox_area))

        dst_node, meta = _pick_candidate(
            src_bbox=src_bbox,
            candidates=candidate_nodes,
            forbidden_nodes=forbidden,
            max_dist=float(args.max_dist),
            min_incident=int(args.min_incident),
            area_ratio_weight=float(args.area_ratio_weight),
            prefer_gate=bool(re.match(str(args.prefer_gate_regex), str(name))),
            max_dst_bbox_area=max_dst_bbox_area,
        )
        if dst_node is None:
            return None, {
                "ok": False,
                "reason": "no_incident_candidate_within_max_dist",
                "src_node": int(src_node),
                "src_node_uid": src_uid,
                "seed_source": str(seed_source),
                "src_bbox": src_bbox,
            }

        dst_ns = nodes.get(int(dst_node), {})
        dst_uid = dst_ns.get("node_uid") if isinstance(dst_ns, dict) else None
        return int(dst_node), {
            "ok": True,
            "reason": "nearest_incident",
            "src_node": int(src_node),
            "src_node_uid": src_uid,
            "dst_node": int(dst_node),
            "dst_node_uid": dst_uid,
            "seed_source": str(seed_source),
            "max_dist": float(args.max_dist),
            "min_incident": int(args.min_incident),
            "area_ratio_weight": float(args.area_ratio_weight),
            "prefer_gate_regex": str(args.prefer_gate_regex),
            "max_dst_bbox_area": float(max_dst_bbox_area) if max_dst_bbox_area is not None else None,
            "allow_large_dst_regex": str(args.allow_large_dst_regex),
            "src_bbox": src_bbox,
            **meta,
        }

    # 1) Remap pad groups first (enforce unique dst nodes per pad_idx).
    for pad_idx in sorted(pad_groups.keys()):
        names = sorted(pad_groups[pad_idx])
        leader = names[0]
        leader_row = block.get(leader)
        if not isinstance(leader_row, dict):
            continue
        dst, meta = _remap_one(name=leader, row=leader_row, forbidden=set(used_dst_nodes))
        if dst is None:
            leader_row["remap_v1"] = meta
            continue
        pad_group_dst[int(pad_idx)] = int(dst)
        used_dst_nodes.add(int(dst))

        dst_ns = nodes.get(int(dst), {})
        dst_uid = dst_ns.get("node_uid") if isinstance(dst_ns, dict) else None
        src_node = int(leader_row.get("layout_node"))
        leader_row["layout_node_src"] = int(src_node)
        leader_row["layout_node"] = int(dst)
        if isinstance(dst_uid, str):
            leader_row["layout_node_uid"] = str(dst_uid)
        leader_row["remap_v1"] = meta

        for alias in names[1:]:
            alias_row = block.get(alias)
            if not isinstance(alias_row, dict):
                continue
            alias_src = alias_row.get("layout_node")
            if isinstance(alias_src, int):
                alias_row["layout_node_src"] = int(alias_src)
            alias_row["layout_node"] = int(dst)
            if isinstance(dst_uid, str):
                alias_row["layout_node_uid"] = str(dst_uid)
            alias_row["remap_v1"] = {
                "ok": True,
                "reason": "pad_group_alias",
                "pad_idx": int(pad_idx),
                "dst_node": int(dst),
                "dst_node_uid": dst_uid,
                "leader": str(leader),
            }

    # 2) Remap remaining anchors (non pad-pin template), allowing them to collide if necessary.
    for name, row in block.items():
        if not isinstance(row, dict):
            continue
        seed = row.get("layout_seed_v0")
        if isinstance(seed, dict) and seed.get("kind") == "pad_pin_template_v0" and isinstance(seed.get("pad_idx"), int):
            # handled above
            continue
        src_node = row.get("layout_node")
        if not isinstance(src_node, int):
            continue
        dst, meta = _remap_one(name=str(name), row=row, forbidden=set())
        if dst is None:
            row["remap_v1"] = meta
            continue
        dst_ns = nodes.get(int(dst), {})
        dst_uid = dst_ns.get("node_uid") if isinstance(dst_ns, dict) else None
        row["layout_node_src"] = int(src_node)
        row["layout_node"] = int(dst)
        if isinstance(dst_uid, str):
            row["layout_node_uid"] = str(dst_uid)
        row["remap_v1"] = meta

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
                "max_transistor_bbox_area": int(args.max_transistor_bbox_area),
                "max_transistor_bbox_dim": int(args.max_transistor_bbox_dim),
            }
        )
    out.write_text(json.dumps(remapped, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(str(out))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
