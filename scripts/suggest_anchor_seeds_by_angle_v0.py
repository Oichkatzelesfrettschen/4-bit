#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


def _load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _write(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def _circ(d: float) -> float:
    while d < -math.pi:
        d += 2 * math.pi
    while d > math.pi:
        d -= 2 * math.pi
    return d


def _signal_first_section(path: Path) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = raw.strip().strip("\r")
        if not line:
            continue
        if line.startswith(";"):
            break
        parts = [p.strip() for p in line.split(",")]
        if len(parts) != 3:
            continue
        x, y, name = int(parts[0]), int(parts[1]), parts[2]
        out.append({"name": name, "x": x, "y": y})
    return out


def _center(points: list[dict[str, Any]]) -> tuple[float, float]:
    xs = [float(p["x"]) for p in points]
    ys = [float(p["y"]) for p in points]
    return (sum(xs) / max(1.0, float(len(xs))), sum(ys) / max(1.0, float(len(ys))))


def _angles(points: list[dict[str, Any]]) -> dict[str, float]:
    cx, cy = _center(points)
    out: dict[str, float] = {}
    for p in points:
        out[str(p["name"])] = math.atan2(float(p["y"]) - cy, float(p["x"]) - cx)
    return out


def _layout_node_angles(chip: str, *, edge_max: int, max_area: int) -> list[dict[str, Any]]:
    net = _load(ROOT / "docs" / "evidence" / "netlists_v0" / f"{chip.lower()}_netlist_v0.json")
    shape = net["inputs"]["layout_shape"]
    w, h = int(shape["w"]), int(shape["h"])
    bbs = [n["metal_bbox"] for n in net["node_stats"] if isinstance(n, dict) and isinstance(n.get("metal_bbox"), dict)]
    minx = min(int(bb["x0"]) for bb in bbs)
    miny = min(int(bb["y0"]) for bb in bbs)
    maxx = max(int(bb["x1"]) for bb in bbs)
    maxy = max(int(bb["y1"]) for bb in bbs)
    cx0, cy0 = (minx + maxx) / 2.0, (miny + maxy) / 2.0

    out: list[dict[str, Any]] = []
    for ns in net.get("node_stats", []):
        if not isinstance(ns, dict) or not isinstance(ns.get("node"), int):
            continue
        bb = ns.get("metal_bbox")
        if not isinstance(bb, dict):
            continue
        x0, y0, x1, y1 = int(bb["x0"]), int(bb["y0"]), int(bb["x1"]), int(bb["y1"])
        dist = int(min(x0, y0, w - x1, h - y1))
        if dist > int(edge_max):
            continue
        area = int((x1 - x0) * (y1 - y0))
        if area <= 0 or area > int(max_area):
            continue
        cx, cy = (x0 + x1) / 2.0, (y0 + y1) / 2.0
        ang = float(math.atan2(cy - cy0, cx - cx0))
        out.append(
            {
                "node": int(ns["node"]),
                "angle": ang,
                "edge_distance": dist,
                "bbox_area": area,
                "bbox": {"x0": x0, "y0": y0, "x1": x1, "y1": y1},
            }
        )
    out.sort(key=lambda r: float(r["angle"]))
    return out


def _best_cyclic_alignment(sig_sorted: list[tuple[str, float]], nodes: list[dict[str, Any]]) -> dict[str, Any]:
    # Use cyclic slices of node list (wrap-around) and pick the alignment with smallest L1 error after
    # subtracting a constant offset (median of diffs).
    if not nodes or not sig_sorted:
        return {"ok": False, "reason": "empty_inputs"}
    N = len(nodes)
    M = len(sig_sorted)
    if N < M:
        return {"ok": False, "reason": "not_enough_nodes", "nodes": N, "signals": M}

    best: tuple[float, int, float, list[dict[str, Any]], list[float]] | None = None
    for start in range(N):
        seg = [nodes[(start + i) % N] for i in range(M)]
        diffs = [_circ(float(seg[i]["angle"]) - float(sig_sorted[i][1])) for i in range(M)]
        diffs2 = sorted(diffs)
        off = float(diffs2[len(diffs2) // 2])
        cost = sum(abs(_circ(d - off)) for d in diffs)
        if best is None or cost < best[0]:
            best = (float(cost), int(start), float(off), seg, diffs)

    if best is None:
        raise AssertionError("angle alignment sweep produced no candidate segment")
    cost, start, off, seg, diffs = best
    matches: list[dict[str, Any]] = []
    for (name, sa), nrow, d in zip(sig_sorted, seg, diffs, strict=False):
        matches.append(
            {
                "signal": name,
                "signal_angle": float(sa),
                "node": int(nrow["node"]),
                "node_angle": float(nrow["angle"]),
                "raw_diff": float(d),
                "diff_minus_offset": float(_circ(d - off)),
                "node_bbox": nrow["bbox"],
                "node_edge_distance": int(nrow["edge_distance"]),
                "node_bbox_area": int(nrow["bbox_area"]),
            }
        )
    return {"ok": True, "cost_l1": float(cost), "start": int(start), "offset": float(off), "matches": matches}


def main() -> int:
    ap = argparse.ArgumentParser(description="Suggest initial anchor layout nodes via cyclic angle alignment (v0).")
    ap.add_argument("--chip", required=True, choices=["4001", "4002", "4003", "4004"])
    ap.add_argument(
        "--signals",
        type=Path,
        default=None,
        help="signals.txt path (defaults to docs/emulators/i<chip>-signals.txt). Uses first section (before ';').",
    )
    ap.add_argument("--edge-max", type=int, default=120, help="Only consider nodes within this distance of layout edge.")
    ap.add_argument("--max-area", type=int, default=800_000, help="Only consider nodes with bbox area <= this.")
    ap.add_argument("--out", type=Path, default=None, help="Write suggestion JSON (defaults under docs/evidence/...).")
    args = ap.parse_args()

    chip = str(args.chip)
    sig_path = (
        (ROOT / args.signals).resolve()
        if args.signals and not args.signals.is_absolute()
        else (args.signals if args.signals else ROOT / "docs" / "emulators" / f"i{chip}-signals.txt")
    )
    signals = _signal_first_section(sig_path)
    sang = _angles(signals)
    sig_sorted = sorted(sang.items(), key=lambda kv: float(kv[1]))
    nodes = _layout_node_angles(chip, edge_max=int(args.edge_max), max_area=int(args.max_area))
    align = _best_cyclic_alignment(sig_sorted, nodes)

    out_path = (
        (ROOT / args.out).resolve()
        if args.out and not args.out.is_absolute()
        else (args.out or (ROOT / "docs" / "evidence" / "anchor_seed_suggestions_v0" / f"{chip}_angle_alignment.json"))
    )
    payload = {
        "chip": chip,
        "schema": {"version": 0, "description": "Angle-alignment suggestion mapping between schematic pins and layout node bboxes."},
        "inputs": {
            "signals_txt": str(sig_path.relative_to(ROOT)) if sig_path.is_relative_to(ROOT) else str(sig_path),
            "netlist_v0": f"docs/evidence/netlists_v0/{chip.lower()}_netlist_v0.json",
        },
        "params": {"edge_max": int(args.edge_max), "max_area": int(args.max_area)},
        "counts": {"signals": int(len(sig_sorted)), "candidate_nodes": int(len(nodes))},
        "alignment": align,
    }
    _write(out_path, payload)
    print(str(out_path.relative_to(ROOT)) if out_path.is_relative_to(ROOT) else str(out_path))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

