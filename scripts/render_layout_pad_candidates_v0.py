#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def load_netlist_v0(chip: str) -> dict:
    p = ROOT / "docs" / "evidence" / "netlists_v0" / f"{chip.lower()}_netlist_v0.json"
    return json.loads(p.read_text(encoding="utf-8"))


def load_candidates(chip: str) -> dict:
    p = ROOT / "docs" / "evidence" / "layout_pad_candidates_v0" / f"{chip.lower()}_layout_pad_candidates_v0.json"
    return json.loads(p.read_text(encoding="utf-8"))


def try_font(size: int) -> ImageFont.ImageFont:
    # Prefer a default bitmap font if truetype is unavailable.
    try:
        return ImageFont.truetype("DejaVuSans.ttf", size=size)
    except Exception:
        return ImageFont.load_default()


def main() -> int:
    p = argparse.ArgumentParser(description="Render overlays/crops for layout pad candidate nodes (v0).")
    p.add_argument("--chip", action="append", choices=["4001", "4002", "4003", "4004"], help="Chip (repeatable)")
    p.add_argument("--all", action="store_true", help="All supported chips")
    p.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "layout_pad_candidates_v0")
    p.add_argument("--max-crops", type=int, default=24, help="Max per-node crops to write per chip")
    p.add_argument("--pad", type=int, default=30, help="Crop padding in pixels")
    p.add_argument("--downscale", type=int, default=2, help="Downscale factor for overview overlay (>=1)")
    args = p.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = {"4001", "4002", "4003", "4004"}
    if not selected:
        p.error("select --all or at least one --chip")

    out_root = Path(args.out_dir)
    out_root.mkdir(parents=True, exist_ok=True)

    font = try_font(16)

    for chip in sorted(selected):
        net = load_netlist_v0(chip)
        cand = load_candidates(chip)

        metal_path = ROOT / net["inputs"]["metal_bmp"]
        base = Image.open(metal_path).convert("L")
        # In these bitmaps: dark = features.
        rgb = Image.merge("RGB", (base, base, base))
        draw = ImageDraw.Draw(rgb)

        candidates = cand.get("candidates", [])
        if not isinstance(candidates, list):
            candidates = []

        # Make per-chip output dir.
        chip_dir = out_root / chip
        crops_dir = chip_dir / "crops"
        chip_dir.mkdir(parents=True, exist_ok=True)
        crops_dir.mkdir(parents=True, exist_ok=True)

        # Draw boxes + labels for top candidates.
        for i, c in enumerate(candidates[: int(args.max_crops)], start=1):
            bb = c.get("metal_bbox")
            if not isinstance(bb, dict):
                continue
            x0, y0, x1, y1 = (int(bb["x0"]), int(bb["y0"]), int(bb["x1"]), int(bb["y1"]))
            node = int(c.get("node"))
            # Rectangle
            draw.rectangle([x0, y0, x1, y1], outline=(255, 0, 0), width=2)
            # Label background
            label = f"{i}:{node}"
            tw, th = draw.textbbox((0, 0), label, font=font)[2:]
            draw.rectangle([x0, max(0, y0 - th - 2), x0 + tw + 6, y0], fill=(255, 255, 255))
            draw.text((x0 + 3, max(0, y0 - th - 1)), label, fill=(0, 0, 0), font=font)

            # Crop for inspection.
            pad = int(args.pad)
            cx0 = max(0, x0 - pad)
            cy0 = max(0, y0 - pad)
            cx1 = min(rgb.size[0], x1 + pad)
            cy1 = min(rgb.size[1], y1 + pad)
            crop = rgb.crop((cx0, cy0, cx1, cy1))
            crop.save(crops_dir / f"rank_{i:02d}_node_{node}.png")

        # Save overview overlay (optionally downscaled for repo size/readability).
        down = max(1, int(args.downscale))
        out_overlay = chip_dir / f"{chip.lower()}_metal_pad_candidates_v0.png"
        if down > 1:
            small = rgb.resize((rgb.size[0] // down, rgb.size[1] // down), resample=Image.Resampling.NEAREST)
            small.save(out_overlay)
        else:
            rgb.save(out_overlay)

        # Also save a tiny manifest per chip.
        (chip_dir / "render_manifest.json").write_text(
            json.dumps(
                {
                    "chip": chip,
                    "tool": "scripts/render_layout_pad_candidates_v0.py",
                    "inputs": {
                        "netlist_v0": rel_or_abs(metal_path),
                        "candidates": rel_or_abs(
                            ROOT / "docs" / "evidence" / "layout_pad_candidates_v0" / f"{chip.lower()}_layout_pad_candidates_v0.json"
                        ),
                    },
                    "outputs": {
                        "overlay": rel_or_abs(out_overlay),
                        "crops_dir": rel_or_abs(crops_dir),
                    },
                    "params": {"max_crops": int(args.max_crops), "pad": int(args.pad), "downscale": down},
                },
                indent=2,
                sort_keys=True,
            )
            + "\n",
            encoding="utf-8",
        )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

