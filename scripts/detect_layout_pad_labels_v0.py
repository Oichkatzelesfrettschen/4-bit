#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
from dataclasses import dataclass
from pathlib import Path

import cv2
import numpy as np
import pytesseract
from PIL import Image, ImageDraw, ImageOps


ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class ChipSpec:
    chip: str
    metal_bmp: Path


def specs() -> dict[str, ChipSpec]:
    emu = ROOT / "docs" / "emulators"
    return {
        "4001": ChipSpec("4001", emu / "i4001-metal.bmp"),
        "4002": ChipSpec("4002", emu / "i4002-metal.bmp"),
        "4003": ChipSpec("4003", emu / "i4003-metal.bmp"),
        "4004": ChipSpec("4004", emu / "i4004-metal.bmp"),
    }


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def ocr_label(img: Image.Image, psm: int) -> dict[str, object]:
    whitelist = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789~+-/|()&._"
    cfg = f"--psm {psm} -l eng -c tessedit_char_whitelist={whitelist}"
    raw = pytesseract.image_to_string(img, config=cfg)
    txt = " ".join(raw.strip().split())
    return {"raw": raw, "text": txt, "psm": psm}

def fill_holes(mask: np.ndarray) -> np.ndarray:
    """
    Fill holes in a binary mask (True=foreground). Returns a boolean array.
    """
    u8 = (mask.astype(np.uint8)) * 255
    h, w = u8.shape
    flood = u8.copy()
    cv2.floodFill(flood, None, (0, 0), 255)
    flood_inv = cv2.bitwise_not(flood)
    filled = cv2.bitwise_or(u8, flood_inv)
    return filled > 0


def main() -> int:
    p = argparse.ArgumentParser(description="Detect pad-label boxes on layout metal masks (v0).")
    p.add_argument("--chip", action="append", choices=sorted(specs().keys()), help="Chip (repeatable)")
    p.add_argument("--all", action="store_true", help="All supported chips")
    p.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "layout_pad_labels_v0")
    p.add_argument("--threshold", type=int, default=128, help="Dark/ink threshold")
    p.add_argument("--edge-band", type=int, default=280, help="Search only within this many pixels of image edge")
    p.add_argument("--min-area", type=int, default=700, help="Min component area")
    p.add_argument("--max-area", type=int, default=40000, help="Max component area")
    p.add_argument("--max-size", type=int, default=420, help="Max width/height of candidate box")
    p.add_argument("--max-aspect", type=float, default=3.0, help="Max bbox aspect ratio")
    p.add_argument("--min-fill", type=float, default=0.22, help="Min fill ratio for candidate box")
    p.add_argument("--pad", type=int, default=8, help="Crop padding for OCR")
    p.add_argument("--scale", type=int, default=4, help="Upscale factor for OCR crops")
    p.add_argument("--render", action="store_true", default=True, help="Render overlay + crops")
    args = p.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = set(specs().keys())
    if not selected:
        p.error("select --all or at least one --chip")

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    manifest: dict[str, object] = {
        "tool": "scripts/detect_layout_pad_labels_v0.py",
        "params": {
            "threshold": int(args.threshold),
            "edge_band": int(args.edge_band),
            "min_area": int(args.min_area),
            "max_area": int(args.max_area),
            "max_size": int(args.max_size),
            "pad": int(args.pad),
        },
        "outputs": [],
    }

    for chip in sorted(selected):
        spec = specs()[chip]
        img = Image.open(spec.metal_bmp).convert("L")
        arr = np.asarray(img)
        h, w = arr.shape

        # Dark pixels are "ink".
        ink = arr < int(args.threshold)

        # Restrict to edge band to avoid the dense metal interior.
        band = int(args.edge_band)
        edge_mask = np.zeros_like(ink, dtype=bool)
        edge_mask[:band, :] = True
        edge_mask[-band:, :] = True
        edge_mask[:, :band] = True
        edge_mask[:, -band:] = True
        ink_edge = ink & edge_mask

        u8 = np.where(ink_edge, 255, 0).astype(np.uint8)
        n, lab, stats, _cent = cv2.connectedComponentsWithStats(u8, connectivity=8)

        boxes: list[dict[str, object]] = []
        for i in range(1, int(n)):
            x, y, ww, hh, area = (int(v) for v in stats[i].tolist())
            if area < int(args.min_area) or area > int(args.max_area):
                continue
            if ww <= 0 or hh <= 0:
                continue
            if ww > int(args.max_size) or hh > int(args.max_size):
                continue
            bbox_area = ww * hh
            fill = float(area) / float(bbox_area)
            # Pad-label squares tend to be filled blocks with holes (letters), so fill is high.
            if fill < float(args.min_fill):
                continue
            # Prefer roughly-square-ish
            aspect = max(ww, hh) / max(1, min(ww, hh))
            if aspect > float(args.max_aspect):
                continue
            boxes.append({"id": i, "bbox": {"x": x, "y": y, "w": ww, "h": hh, "area": area, "fill": fill}})

        # OCR each box by inverting + autocontrast.
        pad = int(args.pad)
        out_boxes: list[dict[str, object]] = []
        crops_dir = out_dir / chip / "crops"
        (out_dir / chip).mkdir(parents=True, exist_ok=True)
        if args.render:
            crops_dir.mkdir(parents=True, exist_ok=True)

        for idx, b in enumerate(sorted(boxes, key=lambda r: (r["bbox"]["y"], r["bbox"]["x"]))):
            bb = b["bbox"]
            x0 = max(0, int(bb["x"]) - pad)
            y0 = max(0, int(bb["y"]) - pad)
            x1 = min(w, int(bb["x"] + bb["w"]) + pad)
            y1 = min(h, int(bb["y"] + bb["h"]) + pad)
            crop = img.crop((x0, y0, x1, y1))

            # Build a clean OCR image by extracting the "holes" (white letters) inside the dark box component.
            roi_lab = lab[y0:y1, x0:x1]
            comp_mask = roi_lab == int(b["id"])
            # Close small gaps in the component to recover the box outline, then fill holes.
            k = np.ones((5, 5), dtype=np.uint8)
            comp_u8 = (comp_mask.astype(np.uint8)) * 255
            comp_closed = cv2.morphologyEx(comp_u8, cv2.MORPH_CLOSE, k, iterations=1) > 0
            box_region = fill_holes(comp_closed)
            letter_mask = box_region & (~comp_mask)

            ocr_np = np.full((crop.size[1], crop.size[0]), 255, dtype=np.uint8)
            ocr_np[letter_mask] = 0
            # Thicken slightly + upscale to help OCR.
            ocr_u8 = ocr_np.copy()
            ocr_u8 = cv2.dilate(ocr_u8, np.ones((2, 2), dtype=np.uint8), iterations=1)
            crop2 = Image.fromarray(ocr_u8)
            scale = max(1, int(args.scale))
            if scale > 1:
                crop2 = crop2.resize((crop2.size[0] * scale, crop2.size[1] * scale), resample=Image.Resampling.NEAREST)

            # Try a couple PSMs.
            o1 = ocr_label(crop2, psm=7)  # single text line
            o2 = ocr_label(crop2, psm=8)  # single word
            o3 = ocr_label(crop2, psm=10)  # single char
            best = max([o1, o2, o3], key=lambda o: len(str(o.get("text", ""))))

            out_row = {
                "bbox": {"x0": int(x0), "y0": int(y0), "x1": int(x1), "y1": int(y1)},
                "ink_bbox": bb,
                "ocr": {"psm7": o1["text"], "psm8": o2["text"], "psm10": o3["text"], "best": best["text"]},
            }
            out_boxes.append(out_row)

            if args.render:
                crop2.save(crops_dir / f"box_{idx:03d}_{best['text'] or 'NA'}.png")

        # Render overlay on the metal mask for quick inspection.
        overlay_path = None
        if args.render:
            rgb = Image.merge("RGB", (img, img, img))
            d = ImageDraw.Draw(rgb)
            for b in out_boxes:
                bb = b["ink_bbox"]
                x, y, ww, hh = int(bb["x"]), int(bb["y"]), int(bb["w"]), int(bb["h"])
                label = str(b["ocr"]["best"])[:12]
                d.rectangle([x, y, x + ww, y + hh], outline=(255, 0, 0), width=2)
                d.text((x + 2, y + 2), label, fill=(255, 0, 0))
            overlay_path = out_dir / chip / f"{chip.lower()}_pad_label_boxes_v0.png"
            rgb.save(overlay_path)

        out_json = out_dir / chip / f"{chip.lower()}_layout_pad_labels_v0.json"
        payload = {
            "chip": chip,
            "schema": {"version": 0, "description": "Detected pad-label-like ink boxes near the layout edge, OCR’d."},
            "inputs": {"metal_bmp": rel_or_abs(spec.metal_bmp), "sha256": {"metal_bmp": sha256(spec.metal_bmp)}},
            "params": manifest["params"],
            "counts": {"candidates": int(len(out_boxes))},
            "boxes": out_boxes,
            "outputs": {
                "overlay": rel_or_abs(overlay_path) if overlay_path else None,
                "crops_dir": rel_or_abs(crops_dir) if args.render else None,
            },
        }
        out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        manifest["outputs"].append({"chip": chip, "output": rel_or_abs(out_json)})

    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
