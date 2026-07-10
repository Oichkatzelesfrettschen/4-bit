#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def load_mask(path: Path, *, threshold: int = 128) -> np.ndarray:
    img = Image.open(path).convert("L")
    arr = np.asarray(img)
    # i400x bitmaps in this repo are typically “ink on paper” (features are dark).
    return arr < int(threshold)


def bbox_contains(bb: dict, x: int, y: int) -> bool:
    return int(bb["x0"]) <= x < int(bb["x1"]) and int(bb["y0"]) <= y < int(bb["y1"])


def bbox_area(bb: dict) -> int:
    return max(0, int(bb["x1"]) - int(bb["x0"])) * max(0, int(bb["y1"]) - int(bb["y0"]))


@dataclass(frozen=True)
class Ray:
    start_x: int
    start_y: int
    dx: int
    dy: int


def choose_ray_for_detection(det: dict, *, w: int, h: int) -> Ray:
    bb = det["bbox"]
    x0, y0, ww, hh = int(bb["x"]), int(bb["y"]), int(bb["w"]), int(bb["h"])
    x1, y1 = x0 + ww, y0 + hh
    # Determine which edge the label is on by nearest border.
    dist_left = x0
    dist_right = w - x1
    dist_top = y0
    dist_bottom = h - y1
    m = min(dist_left, dist_right, dist_top, dist_bottom)
    if m == dist_left:
        return Ray(start_x=x1, start_y=y0 + hh // 2, dx=1, dy=0)
    if m == dist_right:
        return Ray(start_x=x0, start_y=y0 + hh // 2, dx=-1, dy=0)
    if m == dist_top:
        return Ray(start_x=x0 + ww // 2, start_y=y1, dx=0, dy=1)
    return Ray(start_x=x0 + ww // 2, start_y=y0, dx=0, dy=-1)


def incidence_counts(netlist_v0: dict) -> dict[int, int]:
    counts: dict[int, int] = {}
    trans = netlist_v0.get("devices", {}).get("transistors", [])
    if not isinstance(trans, list):
        return counts
    for t in trans:
        if not isinstance(t, dict):
            continue
        for k in ("gate_node", "a_node", "b_node"):
            n = t.get(k)
            if isinstance(n, int):
                counts[n] = counts.get(n, 0) + 1
    return counts


def main() -> int:
    ap = argparse.ArgumentParser(description="Propose layout anchors by ray-casting from edge label boxes into metal mask (v0).")
    ap.add_argument("--chip", default="4004")
    ap.add_argument(
        "--edge-labels",
        type=Path,
        default=ROOT / "docs" / "evidence" / "layout_edge_labels_v0" / "4004" / "4004_layout_edge_labels_v0.json",
    )
    ap.add_argument(
        "--netlist-v0",
        type=Path,
        default=ROOT / "docs" / "evidence" / "netlists_v0" / "4004_netlist_v0.json",
    )
    ap.add_argument("--threshold", type=int, default=128, help="Threshold for metal bitmap")
    ap.add_argument("--max-steps", type=int, default=900, help="Max ray steps")
    ap.add_argument("--skip-margin", type=int, default=6, help="Skip this many pixels beyond the label bbox before searching")
    ap.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "anchor_proposals_v0")
    args = ap.parse_args()

    chip = str(args.chip).strip()
    edge_path = (ROOT / args.edge_labels).resolve() if not args.edge_labels.is_absolute() else args.edge_labels
    net_path = (ROOT / args.netlist_v0).resolve() if not args.netlist_v0.is_absolute() else args.netlist_v0
    edge = json.loads(edge_path.read_text(encoding="utf-8"))
    net = json.loads(net_path.read_text(encoding="utf-8"))

    metal_bmp = ROOT / "docs" / "emulators" / f"i{chip}-metal.bmp"
    metal = load_mask(metal_bmp, threshold=int(args.threshold))
    h, w = metal.shape

    node_stats = [ns for ns in net.get("node_stats", []) if isinstance(ns, dict)]
    metal_nodes = [
        {
            "node": int(ns["node"]),
            "bbox": ns.get("metal_bbox"),
            "node_uid": ns.get("node_uid"),
        }
        for ns in node_stats
        if isinstance(ns.get("node"), int) and isinstance(ns.get("metal_bbox"), dict)
    ]
    inc = incidence_counts(net)

    dets = edge.get("detections", [])
    if not isinstance(dets, list):
        raise SystemExit("edge labels missing detections[]")

    results: list[dict[str, object]] = []
    for det in dets:
        if not isinstance(det, dict):
            continue
        tok = str(det.get("token", "")).strip().upper()
        bb = det.get("bbox")
        if not tok or not isinstance(bb, dict):
            continue
        ray = choose_ray_for_detection(det, w=w, h=h)
        # Skip a small margin beyond the bbox edge so we don't immediately “hit” the label shape.
        x = int(ray.start_x + ray.dx * int(args.skip_margin))
        y = int(ray.start_y + ray.dy * int(args.skip_margin))
        hit = None
        for step in range(int(args.max_steps)):
            if x < 0 or y < 0 or x >= w or y >= h:
                break
            # Avoid “hitting” the label box itself.
            if int(bb.get("x", 0)) <= x < int(bb.get("x", 0) + bb.get("w", 0)) and int(bb.get("y", 0)) <= y < int(bb.get("y", 0) + bb.get("h", 0)):
                x += ray.dx
                y += ray.dy
                continue
            if metal[y, x]:
                # Pick the smallest metal_bbox that contains the point (best localized node).
                containing = [n for n in metal_nodes if bbox_contains(n["bbox"], x, y)]  # type: ignore[arg-type]
                if containing:
                    containing.sort(key=lambda n: bbox_area(n["bbox"]))  # type: ignore[arg-type]
                    n0 = containing[0]
                    hit = {
                        "x": int(x),
                        "y": int(y),
                        "node": int(n0["node"]),
                        "node_uid": n0.get("node_uid"),
                        "node_incidence": int(inc.get(int(n0["node"]), 0)),
                        "node_bbox": n0["bbox"],
                        "steps": int(step + 1),
                    }
                    break
            x += ray.dx
            y += ray.dy

        results.append(
            {
                "token": tok,
                "label_bbox": {"x": int(bb.get("x", 0)), "y": int(bb.get("y", 0)), "w": int(bb.get("w", 0)), "h": int(bb.get("h", 0))},
                "ray": {"x": int(ray.start_x), "y": int(ray.start_y), "dx": int(ray.dx), "dy": int(ray.dy)},
                "hit": hit,
            }
        )

    out_dir = Path(args.out_dir) / chip
    out_dir.mkdir(parents=True, exist_ok=True)
    out_json = out_dir / f"{chip}_edge_label_anchor_proposals_v0.json"
    out_md = out_dir / f"{chip}_edge_label_anchor_proposals_v0.md"
    out_json.write_text(
        json.dumps(
            {
                "schema": {"version": 0, "description": "Ray-cast anchor proposals from edge labels into metal mask."},
                "chip": chip,
                "inputs": {"edge_labels": rel_or_abs(edge_path), "netlist_v0": rel_or_abs(net_path), "metal_bmp": rel_or_abs(metal_bmp)},
                "params": {"threshold": int(args.threshold), "max_steps": int(args.max_steps), "skip_margin": int(args.skip_margin)},
                "results": results,
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )

    lines = [
        f"# Edge-label Anchor Proposals v0 ({chip})",
        "",
        f"- Edge labels: `{rel_or_abs(edge_path)}`",
        f"- Netlist v0: `{rel_or_abs(net_path)}`",
        "",
        "| token | hit_node | steps | incidence |",
        "|---|---:|---:|---:|",
    ]
    for r in results:
        hit = r.get("hit") or {}
        lines.append(
            "| {tok} | {node} | {steps} | {inc} |".format(
                tok=str(r.get("token", "")),
                node=int(hit.get("node", -1)) if isinstance(hit, dict) and hit.get("node") is not None else -1,
                steps=int(hit.get("steps", 0)) if isinstance(hit, dict) and hit.get("steps") is not None else 0,
                inc=int(hit.get("node_incidence", 0)) if isinstance(hit, dict) and hit.get("node_incidence") is not None else 0,
            )
        )
    out_md.write_text("\n".join(lines) + "\n", encoding="utf-8")

    print(rel_or_abs(out_json))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
