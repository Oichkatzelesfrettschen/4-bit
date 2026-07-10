#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
from dataclasses import dataclass
from pathlib import Path

import cv2
import numpy as np
from PIL import Image, ImageDraw

from ocr_cached_backend_v0 import resolve_cached_backend
from ocr_presets_v0 import preset_layout_edge_label

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class ChipSpec:
    chip: str
    metal_image: Path


def specs() -> dict[str, ChipSpec]:
    emu = ROOT / "docs" / "emulators"
    def prefer_png(path: Path) -> Path:
        png = path.with_suffix(".png")
        return png if png.exists() else path
    return {
        "4001": ChipSpec("4001", prefer_png(emu / "i4001-metal.bmp")),
        "4002": ChipSpec("4002", prefer_png(emu / "i4002-metal.bmp")),
        "4003": ChipSpec("4003", prefer_png(emu / "i4003-metal.bmp")),
        "4004": ChipSpec("4004", prefer_png(emu / "i4004-metal.bmp")),
    }


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


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
    p.add_argument(
        "--backend",
        default="tesseract",
        choices=["tesseract", "onnx", "auto"],
        help="OCR backend (auto prefers ONNX/CUDA when configured, else Tesseract).",
    )
    p.add_argument(
        "--onnx-model",
        type=Path,
        default=None,
        help="Path to ONNX model for --backend onnx/auto (or set OCR_ONNX_MODEL).",
    )
    p.add_argument(
        "--prefer-cuda",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Prefer CUDAExecutionProvider for ONNX backends when available.",
    )
    p.add_argument("--render", action="store_true", default=True, help="Render overlay + crops")
    p.add_argument(
        "--save-raw-crops",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Save raw (unprocessed) crops alongside OCR-mask crops when --render is enabled.",
    )
    p.add_argument(
        "--limit-ocr",
        type=int,
        default=0,
        help="If >0, only run OCR for the first N detected boxes (fast preview); still renders all boxes.",
    )
    args = p.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = set(specs().keys())
    if not selected:
        p.error("select --all or at least one --chip")

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    # Default to repo-provided templates for the PMOS-era glyph style when available.
    # Override by setting OCR_TEMPLATE_DIR explicitly.
    if not os.environ.get("OCR_TEMPLATE_DIR", "").strip():
        default_tdir = (ROOT / "docs/evidence/ocr_models/templates_v0").resolve()
        if default_tdir.exists():
            os.environ["OCR_TEMPLATE_DIR"] = str(default_tdir)
    backend = resolve_cached_backend(backend=str(args.backend), onnx_model=args.onnx_model, prefer_cuda=bool(args.prefer_cuda))

    manifest: dict[str, object] = {
        "tool": "scripts/detect_layout_pad_labels_v0.py",
        "params": {
            "threshold": int(args.threshold),
            "edge_band": int(args.edge_band),
            "min_area": int(args.min_area),
            "max_area": int(args.max_area),
            "max_size": int(args.max_size),
            "pad": int(args.pad),
            "backend": str(getattr(backend, "name", "tesseract")),
        },
        "outputs": [],
    }

    for chip in sorted(selected):
        spec = specs()[chip]
        img = Image.open(spec.metal_image).convert("L")
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
        crops_raw_dir = out_dir / chip / "crops_raw"
        crops_mask_dir = out_dir / chip / "crops_mask"
        (out_dir / chip).mkdir(parents=True, exist_ok=True)
        if args.render:
            crops_dir.mkdir(parents=True, exist_ok=True)
            crops_mask_dir.mkdir(parents=True, exist_ok=True)
            if bool(args.save_raw_crops):
                crops_raw_dir.mkdir(parents=True, exist_ok=True)

        ocr_limit = int(args.limit_ocr or 0)
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

            # OCR: pad labels are short (1–3 chars) and usually high-contrast.
            gray = np.asarray(crop2.convert("L"))
            do_ocr = ocr_limit <= 0 or idx < ocr_limit
            if do_ocr:
                preset = preset_layout_edge_label(expected=None)
                r = backend.best_token(
                    gray,
                    whitelist="ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
                    psms=preset.psms,
                    oem=preset.oem,
                    min_len=1,
                    max_len=3,
                )
                best_text = str(r.token or "")
                conf = float(r.conf)
                psm = int(r.psm)
                inv = bool(r.invert)
                ocr_scale = int(r.scale)
            else:
                best_text = ""
                conf = -1.0
                psm = 0
                inv = False
                ocr_scale = int(scale)

            out_row = {
                "bbox": {"x0": int(x0), "y0": int(y0), "x1": int(x1), "y1": int(y1)},
                "ink_bbox": bb,
                "ocr": {
                    "best": best_text,
                    "conf": conf,
                    "psm": psm,
                    "invert": inv,
                    "scale": ocr_scale,
                    "backend": str(getattr(backend, "name", "tesseract")),
                },
            }
            if args.render:
                out_row["crops"] = {
                    "mask": rel_or_abs(crops_mask_dir / f"box_{idx:03d}_{best_text or 'NA'}.png"),
                    "raw": rel_or_abs(crops_raw_dir / f"box_{idx:03d}.png") if bool(args.save_raw_crops) else None,
                }
            out_boxes.append(out_row)

            if args.render:
                # Keep legacy path stable for existing reports.
                crop2.save(crops_dir / f"box_{idx:03d}_{best_text or 'NA'}.png")
                crop2.save(crops_mask_dir / f"box_{idx:03d}_{best_text or 'NA'}.png")
                if bool(args.save_raw_crops):
                    crop.save(crops_raw_dir / f"box_{idx:03d}.png", format="PNG", optimize=False, compress_level=9)

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
            "inputs": {
                "metal_image": rel_or_abs(spec.metal_image),
                "sha256": {"metal_image": sha256(spec.metal_image)},
            },
            "params": manifest["params"],
            "counts": {"candidates": int(len(out_boxes))},
            "boxes": out_boxes,
            "outputs": {
                "overlay": rel_or_abs(overlay_path) if overlay_path else None,
                "crops_dir": rel_or_abs(crops_dir) if args.render else None,
                "crops_mask_dir": rel_or_abs(crops_mask_dir) if args.render else None,
                "crops_raw_dir": rel_or_abs(crops_raw_dir) if args.render and bool(args.save_raw_crops) else None,
            },
        }
        out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        manifest["outputs"].append({"chip": chip, "output": rel_or_abs(out_json)})

    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
