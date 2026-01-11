#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parents[1]


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def bbox_center(bb: dict[str, int]) -> tuple[float, float]:
    return (float(bb["x"] + bb["w"] / 2.0), float(bb["y"] + bb["h"] / 2.0))


def dist_point_to_bbox(px: float, py: float, bb: dict[str, int]) -> float:
    x0, y0, x1, y1 = float(bb["x0"]), float(bb["y0"]), float(bb["y0"]), float(bb["y1"])
    # bug guard: y0/y1 already set incorrectly above; do correctly:
    x0, y0, x1, y1 = float(bb["x0"]), float(bb["y0"]), float(bb["x1"]), float(bb["y1"])
    dx = 0.0
    if px < x0:
        dx = x0 - px
    elif px > x1:
        dx = px - x1
    dy = 0.0
    if py < y0:
        dy = y0 - py
    elif py > y1:
        dy = py - y1
    return math.hypot(dx, dy)


def try_font(size: int) -> ImageFont.ImageFont:
    try:
        return ImageFont.truetype("DejaVuSans.ttf", size=size)
    except Exception:
        return ImageFont.load_default()


def main() -> int:
    p = argparse.ArgumentParser(description="Suggest layout node IDs for each detected pad box (v0).")
    p.add_argument("--chip", choices=["4001", "4002", "4003", "4004"], required=True)
    p.add_argument("--pad-boxes", type=Path, default=None, help="layout_pad_labels_v0 JSON (default: computed path)")
    p.add_argument("--netlist-v0", type=Path, default=None, help="netlists_v0 JSON (default: computed path)")
    p.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "layout_pad_labels_v0")
    p.add_argument("--max-boxes", type=int, default=80, help="Limit boxes (top-left sort) to reduce clutter")
    p.add_argument("--area-penalty", type=float, default=0.003, help="Penalty multiplier for large metal bbox area")
    args = p.parse_args()

    chip = args.chip
    pad_boxes_path = (
        Path(args.pad_boxes)
        if args.pad_boxes
        else (ROOT / "docs" / "evidence" / "layout_pad_labels_v0" / chip / f"{chip.lower()}_layout_pad_labels_v0.json")
    )
    netlist_path = (
        Path(args.netlist_v0)
        if args.netlist_v0
        else (ROOT / "docs" / "evidence" / "netlists_v0" / f"{chip.lower()}_netlist_v0.json")
    )

    pad_obj = load_json(pad_boxes_path)
    net_obj = load_json(netlist_path)

    nodes = [n for n in net_obj.get("node_stats", []) if isinstance(n, dict) and isinstance(n.get("metal_bbox"), dict)]
    if not nodes:
        raise SystemExit("netlist_v0 missing node_stats metal_bbox; re-run extract_netlist_v0.py")

    # For rendering
    metal_path = ROOT / net_obj["inputs"]["metal_bmp"]
    metal_img = Image.open(metal_path).convert("RGB")
    draw = ImageDraw.Draw(metal_img)
    font = try_font(14)

    boxes = pad_obj.get("boxes", [])
    if not isinstance(boxes, list):
        boxes = []

    # Use ink_bbox (tight) for visuals, bbox (padded) for center
    boxes_sorted = sorted(boxes, key=lambda b: (int(b["ink_bbox"]["y"]), int(b["ink_bbox"]["x"])))[: int(args.max_boxes)]

    suggestions = []
    for idx, b in enumerate(boxes_sorted):
        bb = b["ink_bbox"]
        cx, cy = bbox_center(bb)
        best = None
        for n in nodes:
            mb = n["metal_bbox"]
            d = dist_point_to_bbox(cx, cy, mb)
            area = float(n.get("metal_area", 0))
            score = d + float(args.area_penalty) * math.log1p(area)
            if best is None or score < best["score"]:
                best = {
                    "node": int(n["node"]),
                    "score": float(score),
                    "dist": float(d),
                    "metal_area": int(n.get("metal_area", 0)),
                    "metal_bbox": mb,
                }
        assert best is not None
        suggestions.append(
            {
                "box_index": idx,
                "ink_bbox": bb,
                "suggested": best,
            }
        )

        # Draw the ink bbox and node id.
        x, y, w, h = int(bb["x"]), int(bb["y"]), int(bb["w"]), int(bb["h"])
        draw.rectangle([x, y, x + w, y + h], outline=(0, 0, 255), width=2)
        label = f"{best['node']}"
        draw.text((x + 2, y + 2), label, fill=(0, 0, 255), font=font)

    out_chip_dir = Path(args.out_dir) / chip
    out_chip_dir.mkdir(parents=True, exist_ok=True)

    out_json = out_chip_dir / f"{chip.lower()}_pad_boxes_node_suggestions_v0.json"
    payload = {
        "chip": chip,
        "schema": {"version": 0, "description": "Geometry-based suggested netlist_v0 nodes for each detected pad box."},
        "inputs": {"pad_boxes": rel_or_abs(pad_boxes_path), "netlist_v0": rel_or_abs(netlist_path)},
        "params": {"area_penalty": float(args.area_penalty), "max_boxes": int(args.max_boxes)},
        "suggestions": suggestions,
    }
    out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    out_overlay = out_chip_dir / f"{chip.lower()}_pad_boxes_node_suggestions_v0.png"
    metal_img.save(out_overlay)

    print(json.dumps({"out_json": rel_or_abs(out_json), "out_overlay": rel_or_abs(out_overlay)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

