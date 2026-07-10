#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _bbox_iou(a: dict, b: dict) -> float:
    ax0, ay0, ax1, ay1 = int(a["x0"]), int(a["y0"]), int(a["x1"]), int(a["y1"])
    bx0, by0, bx1, by1 = int(b["x0"]), int(b["y0"]), int(b["x1"]), int(b["y1"])
    ix0, iy0 = max(ax0, bx0), max(ay0, by0)
    ix1, iy1 = min(ax1, bx1), min(ay1, by1)
    iw, ih = max(0, ix1 - ix0), max(0, iy1 - iy0)
    inter = iw * ih
    if inter <= 0:
        return 0.0
    a_area = max(0, ax1 - ax0) * max(0, ay1 - ay0)
    b_area = max(0, bx1 - bx0) * max(0, by1 - by0)
    denom = a_area + b_area - inter
    return float(inter) / float(denom) if denom > 0 else 0.0


def _node_index_by_metal_bbox(net: dict) -> list[tuple[int, dict, str | None]]:
    out: list[tuple[int, dict, str | None]] = []
    for ns in net.get("node_stats", []):
        if not isinstance(ns, dict) or not isinstance(ns.get("node"), int):
            continue
        mb = ns.get("metal_bbox")
        if not isinstance(mb, dict):
            continue
        out.append((int(ns["node"]), mb, ns.get("node_uid") if isinstance(ns.get("node_uid"), str) else None))
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description="Remap anchor layout_node IDs between netlist_v0 runs via metal_bbox IoU (v0).")
    ap.add_argument("--chip", default="4004")
    ap.add_argument("--anchors", type=Path, required=True, help="Anchors JSON (schematic_layout_anchors_v0.json)")
    ap.add_argument("--src-netlist-v0", type=Path, required=True, help="Source netlist_v0 (where anchors currently point)")
    ap.add_argument("--dst-netlist-v0", type=Path, required=True, help="Destination netlist_v0 (new extraction)")
    ap.add_argument("--out", type=Path, required=True, help="Write remapped anchors JSON here")
    ap.add_argument("--min-iou", type=float, default=0.15, help="Minimum IoU to accept a remap")
    args = ap.parse_args()

    chip = str(args.chip).strip()
    anchors_path = (ROOT / args.anchors).resolve() if not args.anchors.is_absolute() else args.anchors
    src_path = (ROOT / args.src_netlist_v0).resolve() if not args.src_netlist_v0.is_absolute() else args.src_netlist_v0
    dst_path = (ROOT / args.dst_netlist_v0).resolve() if not args.dst_netlist_v0.is_absolute() else args.dst_netlist_v0

    anchors = _load(anchors_path)
    src = _load(src_path)
    dst = _load(dst_path)

    aroot = anchors.get("anchors")
    if not isinstance(aroot, dict) or not isinstance(aroot.get(chip), dict):
        raise SystemExit(f"anchors missing chip={chip}")

    src_stats = {int(ns["node"]): ns for ns in src.get("node_stats", []) if isinstance(ns, dict) and isinstance(ns.get("node"), int)}
    dst_index = _node_index_by_metal_bbox(dst)

    remapped = json.loads(json.dumps(anchors))  # deep copy via json
    block = remapped["anchors"][chip]
    for _name, row in block.items():
        if not isinstance(row, dict):
            continue
        n = row.get("layout_node")
        if not isinstance(n, int):
            continue
        ns = src_stats.get(int(n))
        mb = ns.get("metal_bbox") if isinstance(ns, dict) else None
        if not isinstance(mb, dict):
            continue
        best = None
        for dst_node, dst_mb, dst_uid in dst_index:
            iou = _bbox_iou(mb, dst_mb)
            if best is None or iou > best["iou"]:
                best = {"dst_node": dst_node, "dst_uid": dst_uid, "iou": float(iou)}
        if best is None or float(best["iou"]) < float(args.min_iou):
            row["remap_v0"] = {"ok": False, "reason": "no_bbox_match", "src_node": int(n)}
            continue
        row["layout_node_src"] = int(n)
        row["layout_node"] = int(best["dst_node"])
        if best.get("dst_uid"):
            row["layout_node_uid"] = str(best["dst_uid"])
        row["remap_v0"] = {"ok": True, "iou": float(best["iou"]), "src_node": int(n), "dst_node": int(best["dst_node"])}

    out = args.out
    if not out.is_absolute():
        out = (ROOT / out).resolve()
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(remapped, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(str(out))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

