#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import cv2  # type: ignore

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class Box:
    x0: int
    y0: int
    x1: int
    y1: int

    @property
    def w(self) -> int:
        return max(0, self.x1 - self.x0)

    @property
    def h(self) -> int:
        return max(0, self.y1 - self.y0)

    @property
    def area(self) -> int:
        return self.w * self.h

    def center(self) -> tuple[float, float]:
        return (0.5 * (self.x0 + self.x1), 0.5 * (self.y0 + self.y1))

    def edge_distance(self, *, width: int, height: int) -> int:
        return int(min(self.x0, self.y0, width - self.x1, height - self.y1))


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def _largest_contour_bbox(mask: Any) -> Box | None:
    contours, _hier = cv2.findContours(mask, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    if not contours:
        return None
    c = max(contours, key=cv2.contourArea)
    x, y, w, h = cv2.boundingRect(c)
    return Box(int(x), int(y), int(x + w), int(y + h))


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Detect bond-pad-like blocks on metal masks (periphery components) and suggest node matches.",
    )
    ap.add_argument("--chip", required=True, help="Chip id (e.g., 4001)")
    ap.add_argument(
        "--image",
        type=Path,
        default=None,
        help="Metal image (defaults to docs/emulators/i<chip>-metal.png)",
    )
    ap.add_argument(
        "--netlist-v0",
        type=Path,
        default=None,
        help="Layout netlist_v0 JSON (defaults to docs/evidence/netlists_v0/<chip>_netlist_v0.json)",
    )
    ap.add_argument("--edge-max", type=int, default=120, help="Max distance from image edge (pixels)")
    ap.add_argument("--min-bbox-area", type=int, default=800, help="Min bbox area (px^2)")
    ap.add_argument("--max-bbox-area", type=int, default=120000, help="Max bbox area (px^2)")
    ap.add_argument("--min-fill", type=float, default=0.60, help="Min fill ratio inside bbox")
    ap.add_argument("--min-dim", type=int, default=18, help="Min width/height (px)")
    ap.add_argument(
        "--open-kernel",
        type=int,
        default=0,
        help="Optional morphological opening kernel size (odd int, px). Helps break thin metal connections to isolate pads.",
    )
    ap.add_argument("--out-json", type=Path, default=None)
    ap.add_argument("--out-png", type=Path, default=None)
    args = ap.parse_args()

    chip = str(args.chip).strip()
    img_path = (
        (ROOT / args.image).resolve()
        if args.image is not None and not args.image.is_absolute()
        else (args.image if args.image is not None else ROOT / "docs" / "emulators" / f"i{chip}-metal.png")
    )
    net_path = (
        (ROOT / args.netlist_v0).resolve()
        if args.netlist_v0 is not None and not args.netlist_v0.is_absolute()
        else (
            args.netlist_v0
            if args.netlist_v0 is not None
            else ROOT / "docs" / "evidence" / "netlists_v0" / f"{chip}_netlist_v0.json"
        )
    )

    out_json = args.out_json or (ROOT / "docs" / "evidence" / "layout_pads_v0" / chip / f"{chip}_layout_pads_v0.json")
    out_png = args.out_png or (ROOT / "docs" / "evidence" / "layout_pads_v0" / chip / f"{chip}_layout_pads_v0.png")
    if not out_json.is_absolute():
        out_json = (ROOT / out_json).resolve()
    if not out_png.is_absolute():
        out_png = (ROOT / out_png).resolve()

    img = cv2.imread(str(img_path), cv2.IMREAD_GRAYSCALE)
    if img is None:
        raise SystemExit(f"failed to read image: {img_path}")
    h, w = img.shape[:2]

    # Binary mask for metal (black) pixels.
    _thr, bw = cv2.threshold(img, 200, 255, cv2.THRESH_BINARY_INV)

    work = bw
    if int(args.open_kernel) > 0:
        k = int(args.open_kernel)
        if k % 2 == 0:
            k += 1
        kernel = cv2.getStructuringElement(cv2.MORPH_RECT, (k, k))
        work = cv2.morphologyEx(work, cv2.MORPH_OPEN, kernel)

    # Connected components on black pixels.
    num, labels, stats, _centroids = cv2.connectedComponentsWithStats(work, connectivity=8)

    # Load node_stats for suggestions (bbox-only heuristic).
    net = _load_json(net_path)
    node_stats = net.get("node_stats")
    ns_by_node: dict[int, dict[str, Any]] = {}
    if isinstance(node_stats, list):
        for ns in node_stats:
            if isinstance(ns, dict) and isinstance(ns.get("node"), int):
                ns_by_node[int(ns["node"])] = ns

    def best_nodes_for_bbox(box: Box, *, topk: int = 5) -> list[dict[str, Any]]:
        # Prefer nodes whose metal_bbox contains the pad center; this avoids always snapping to huge
        # periphery rings/buses when multiple nets overlap near the edge.
        cx, cy = box.center()
        out: list[tuple[float, float, int, dict[str, Any]]] = []
        inside: list[tuple[int, dict[str, Any]]] = []
        for node, ns in ns_by_node.items():
            mb = ns.get("metal_bbox")
            if not isinstance(mb, dict):
                continue
            x0, y0, x1, y1 = int(mb["x0"]), int(mb["y0"]), int(mb["x1"]), int(mb["y1"])
            if x0 <= cx <= x1 and y0 <= cy <= y1:
                inside.append((int(node), ns))

        candidates = inside if inside else list(ns_by_node.items())
        for node, ns in candidates:
            mb = ns.get("metal_bbox")
            if not isinstance(mb, dict):
                continue
            x0, y0, x1, y1 = int(mb["x0"]), int(mb["y0"]), int(mb["x1"]), int(mb["y1"])
            # Rect distance
            dx = 0
            if box.x1 < x0:
                dx = x0 - box.x1
            elif x1 < box.x0:
                dx = box.x0 - x1
            dy = 0
            if box.y1 < y0:
                dy = y0 - box.y1
            elif y1 < box.y0:
                dy = box.y0 - y1
            rect_d = math.hypot(dx, dy)
            # Center distance as tie-breaker
            ncx = 0.5 * (x0 + x1)
            ncy = 0.5 * (y0 + y1)
            center_d = math.hypot(ncx - cx, ncy - cy)
            # Penalize extremely huge bboxes (often global buses)
            bbox_area = max(0, x1 - x0) * max(0, y1 - y0)
            size_pen = math.log10(1.0 + bbox_area) / 10.0
            score = rect_d + 0.02 * center_d + size_pen
            if inside:
                # When inside, prioritize smallest bbox strongly.
                score = score + 0.00002 * float(bbox_area)
            out.append((score, rect_d, int(node), ns))
        out.sort(key=lambda t: (t[0], t[1], t[2]))
        best = []
        for score, rect_d, node, ns in out[:topk]:
            mb = ns.get("metal_bbox")
            best.append(
                {
                    "node": int(node),
                    "score": float(score),
                    "rect_distance": float(rect_d),
                    "metal_bbox": mb,
                    "metal_area": int(ns.get("metal_area") or 0),
                    "node_uid": ns.get("node_uid"),
                }
            )
        return best

    pads: list[dict[str, Any]] = []
    for label in range(1, int(num)):
        x, y, bw_w, bw_h, area = stats[label]
        if bw_w < int(args.min_dim) or bw_h < int(args.min_dim):
            continue
        box = Box(int(x), int(y), int(x + bw_w), int(y + bw_h))
        if box.area < int(args.min_bbox_area) or box.area > int(args.max_bbox_area):
            continue
        near = box.edge_distance(width=w, height=h)
        if near > int(args.edge_max):
            continue
        # Fill ratio inside bbox.
        roi = work[box.y0 : box.y1, box.x0 : box.x1]
        fill = float(cv2.countNonZero(roi)) / float(max(1, roi.size))
        if fill < float(args.min_fill):
            continue
        pads.append(
            {
                "cc_label": int(label),
                "bbox": {"x0": box.x0, "y0": box.y0, "x1": box.x1, "y1": box.y1, "w": box.w, "h": box.h, "area": box.area},
                "edge_distance": int(near),
                "fill": float(fill),
                "suggested_nodes": best_nodes_for_bbox(box, topk=5),
            }
        )

    def _center(p: dict[str, Any]) -> tuple[float, float]:
        b = p["bbox"]
        return (0.5 * (b["x0"] + b["x1"]), 0.5 * (b["y0"] + b["y1"]))

    # Two orderings:
    # - idx_angle_ccw: polar angle around centroid (can mis-order on rectangular peripheries).
    # - idx_perimeter_ccw: sort by nearest edge then sweep CCW around the bounding rectangle.
    if pads:
        cx = sum(_center(p)[0] for p in pads) / len(pads)
        cy = sum(_center(p)[1] for p in pads) / len(pads)
        for p in pads:
            bx, by = _center(p)
            p["angle"] = float(math.atan2(by - cy, bx - cx))
        pads.sort(key=lambda p: (p["angle"], _center(p)[1], _center(p)[0]))
        for i, p in enumerate(pads):
            p["idx_angle_ccw"] = int(i)

        # Perimeter order: group by nearest edge, then stitch together CCW.
        groups: dict[str, list[dict[str, Any]]] = {"top": [], "right": [], "bottom": [], "left": []}
        for p in pads:
            bx, by = _center(p)
            d = {
                "left": bx,
                "right": float(w) - bx,
                "top": by,
                "bottom": float(h) - by,
            }
            side = min(d.items(), key=lambda kv: kv[1])[0]
            p["nearest_edge"] = side
            groups[side].append(p)

        top = sorted(groups["top"], key=lambda p: _center(p)[0])
        right = sorted(groups["right"], key=lambda p: _center(p)[1])
        bottom = sorted(groups["bottom"], key=lambda p: _center(p)[0], reverse=True)
        left = sorted(groups["left"], key=lambda p: _center(p)[1], reverse=True)
        stitched = top + right + bottom + left
        for i, p in enumerate(stitched):
            p["idx_perimeter_ccw"] = int(i)
        # Keep original key name for backward-compat in any downstream adhoc usage.
        for p in pads:
            if "idx_ccw" not in p:
                p["idx_ccw"] = p.get("idx_perimeter_ccw", p.get("idx_angle_ccw"))

    # Render overlay for quick inspection.
    overlay = cv2.cvtColor(img, cv2.COLOR_GRAY2BGR)
    for p in pads:
        b = p["bbox"]
        x0, y0, x1, y1 = int(b["x0"]), int(b["y0"]), int(b["x1"]), int(b["y1"])
        cv2.rectangle(overlay, (x0, y0), (x1, y1), (0, 0, 255), 2)
        txt = str(p.get("idx_perimeter_ccw", p.get("idx_ccw", "?")))
        cv2.putText(overlay, txt, (x0, max(10, y0 - 4)), cv2.FONT_HERSHEY_SIMPLEX, 0.5, (255, 0, 0), 1, cv2.LINE_AA)

    out_png.parent.mkdir(parents=True, exist_ok=True)
    cv2.imwrite(str(out_png), overlay)

    payload = {
        "chip": chip,
        "counts": {"pads": int(len(pads))},
        "params": {
            "edge_max": int(args.edge_max),
            "min_bbox_area": int(args.min_bbox_area),
            "max_bbox_area": int(args.max_bbox_area),
            "min_fill": float(args.min_fill),
            "min_dim": int(args.min_dim),
        },
        "inputs": {
            "image": str(img_path.relative_to(ROOT)) if img_path.is_relative_to(ROOT) else str(img_path),
            "netlist_v0": str(net_path.relative_to(ROOT)) if net_path.is_relative_to(ROOT) else str(net_path),
        },
        "pads": pads,
        "outputs": {"overlay_png": str(out_png.relative_to(ROOT)) if out_png.is_relative_to(ROOT) else str(out_png)},
        "schema": {"version": 0, "description": "Periphery pad-like component detections with node suggestions (metal mask)."},
    }
    _write_json(out_json, payload)
    print(str(out_json))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
