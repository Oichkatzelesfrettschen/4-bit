#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]


def _load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _best_bbox(ns: dict[str, Any]) -> dict[str, Any] | None:
    for key in ("metal_bbox", "poly_bbox", "diffusion_bbox"):
        b = ns.get(key)
        if isinstance(b, dict) and {"x0", "y0", "x1", "y1"} <= set(b.keys()):
            return b
    return None


def _crop_with_margin(img: Image.Image, bbox: dict[str, Any], margin: int) -> tuple[Image.Image, tuple[int, int]]:
    x0, y0, x1, y1 = int(bbox["x0"]), int(bbox["y0"]), int(bbox["x1"]), int(bbox["y1"])
    x0m, y0m = max(0, x0 - margin), max(0, y0 - margin)
    x1m, y1m = min(img.width, x1 + margin), min(img.height, y1 + margin)
    return img.crop((x0m, y0m, x1m, y1m)), (x0m, y0m)


def _draw_bbox(draw: ImageDraw.ImageDraw, bbox: dict[str, Any], offset: tuple[int, int], color: tuple[int, int, int]) -> None:
    ox, oy = offset
    x0, y0, x1, y1 = int(bbox["x0"]) - ox, int(bbox["y0"]) - oy, int(bbox["x1"]) - ox, int(bbox["y1"]) - oy
    draw.rectangle((x0, y0, x1, y1), outline=color, width=3)


def main() -> int:
    ap = argparse.ArgumentParser(description="Render per-anchor crops showing src/dst node bboxes (v1).")
    ap.add_argument("--chip", default="4004")
    ap.add_argument("--anchors", type=Path, required=True, help="Anchors JSON (v1 recommended)")
    ap.add_argument("--netlist-v0", type=Path, required=True, help="netlist_v0 JSON (node_stats)")
    ap.add_argument("--image", type=Path, required=True, help="Layout image (e.g., docs/emulators/i4004-metal.bmp)")
    ap.add_argument("--out-dir", type=Path, required=True)
    ap.add_argument("--margin", type=int, default=80)
    args = ap.parse_args()

    chip = str(args.chip).strip()
    anchors_path = (ROOT / args.anchors).resolve() if not args.anchors.is_absolute() else args.anchors
    net_path = (ROOT / args.netlist_v0).resolve() if not args.netlist_v0.is_absolute() else args.netlist_v0
    img_path = (ROOT / args.image).resolve() if not args.image.is_absolute() else args.image

    anchors = _load(anchors_path)
    net = _load(net_path)

    aroot = anchors.get("anchors")
    if not isinstance(aroot, dict) or not isinstance(aroot.get(chip), dict):
        raise SystemExit(f"anchors missing chip={chip}")

    nodes = {int(ns["node"]): ns for ns in net.get("node_stats", []) if isinstance(ns, dict) and isinstance(ns.get("node"), int)}
    img = Image.open(img_path).convert("RGB")

    out_dir = args.out_dir
    if not out_dir.is_absolute():
        out_dir = (ROOT / out_dir).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    block = aroot[chip]
    for name, row in block.items():
        if not isinstance(row, dict) or not isinstance(row.get("layout_node"), int):
            continue
        dst_node = int(row["layout_node"])
        src_node = int(row.get("layout_node_src", dst_node))
        src_ns = nodes.get(src_node)
        dst_ns = nodes.get(dst_node)
        if not isinstance(src_ns, dict) or not isinstance(dst_ns, dict):
            continue
        src_bbox = _best_bbox(src_ns)
        dst_bbox = _best_bbox(dst_ns)
        if not isinstance(src_bbox, dict) or not isinstance(dst_bbox, dict):
            continue

        for kind, node, bbox, color in (
            ("src", src_node, src_bbox, (255, 0, 0)),
            ("dst", dst_node, dst_bbox, (0, 200, 0)),
        ):
            crop, offset = _crop_with_margin(img, bbox, int(args.margin))
            draw = ImageDraw.Draw(crop)
            _draw_bbox(draw, bbox, offset, color)
            out_path = out_dir / f"{chip}_{name}_{kind}_node{node}.png"
            crop.save(out_path)

    print(str(out_dir))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

