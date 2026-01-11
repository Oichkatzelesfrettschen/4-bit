#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parents[1]


def try_font(size: int) -> ImageFont.ImageFont:
    try:
        return ImageFont.truetype("DejaVuSans.ttf", size=size)
    except Exception:
        return ImageFont.load_default()


def main() -> int:
    p = argparse.ArgumentParser(description="Render human-readable crops around detected layout pad boxes (v0).")
    p.add_argument("--chip", required=True, choices=["4001", "4002", "4003", "4004"])
    p.add_argument("--in-dir", type=Path, default=ROOT / "docs" / "evidence" / "layout_pad_labels_v0")
    p.add_argument("--out-dir", type=Path, default=None)
    p.add_argument("--pad", type=int, default=70)
    p.add_argument("--limit", type=int, default=80)
    args = p.parse_args()

    chip = args.chip
    in_chip = Path(args.in_dir) / chip
    pad_json = in_chip / f"{chip.lower()}_layout_pad_labels_v0.json"
    sugg_json = in_chip / f"{chip.lower()}_pad_boxes_node_suggestions_v0.json"
    if not pad_json.exists() or not sugg_json.exists():
        raise SystemExit(f"missing inputs: {pad_json} or {sugg_json}")

    pad_obj = json.loads(pad_json.read_text(encoding="utf-8"))
    sugg_obj = json.loads(sugg_json.read_text(encoding="utf-8"))

    # Load metal bmp from pad json.
    metal_path = ROOT / pad_obj["inputs"]["metal_bmp"]
    img = Image.open(metal_path).convert("RGB")

    out_dir = Path(args.out_dir) if args.out_dir else (in_chip / "human_crops")
    out_dir.mkdir(parents=True, exist_ok=True)

    font = try_font(18)

    rows = sugg_obj.get("suggestions", [])
    if not isinstance(rows, list):
        rows = []

    # Sort by position (top-left).
    rows = sorted(rows, key=lambda r: (int(r["ink_bbox"]["y"]), int(r["ink_bbox"]["x"])))[: int(args.limit)]

    for i, r in enumerate(rows):
        bb = r["ink_bbox"]
        x, y, w, h = int(bb["x"]), int(bb["y"]), int(bb["w"]), int(bb["h"])
        node = int(r["suggested"]["node"])
        pad = int(args.pad)
        x0 = max(0, x - pad)
        y0 = max(0, y - pad)
        x1 = min(img.size[0], x + w + pad)
        y1 = min(img.size[1], y + h + pad)
        crop = img.crop((x0, y0, x1, y1))
        d = ImageDraw.Draw(crop)
        d.rectangle([x - x0, y - y0, x - x0 + w, y - y0 + h], outline=(255, 0, 0), width=3)
        d.text((4, 4), f"box#{i} node={node}", fill=(0, 0, 255), font=font)
        crop.save(out_dir / f"box_{i:03d}_node_{node}.png")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

