#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
from dataclasses import dataclass
from pathlib import Path

import cv2
import numpy as np
from PIL import Image


@dataclass(frozen=True)
class ChipSpec:
    chip: str
    poly_bmp: Path
    diffusion_bmp: Path


ROOT = Path(__file__).resolve().parents[1]


def specs() -> dict[str, ChipSpec]:
    def emu(p: str) -> Path:
        return ROOT / "docs" / "emulators" / p

    return {
        "4001": ChipSpec("4001", emu("i4001-poly.bmp"), emu("i4001-diffusion.bmp")),
        "4002": ChipSpec("4002", emu("i4002-poly.bmp"), emu("i4002-diffusion.bmp")),
        "4003": ChipSpec("4003", emu("i4003-poly.bmp"), emu("i4003-diffusion.bmp")),
        "4004": ChipSpec("4004", emu("i4004-poly.bmp"), emu("i4004-diffusion.bmp")),
    }


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def load_mask(path: Path, threshold: int) -> np.ndarray:
    img = Image.open(path).convert("L")
    arr = np.asarray(img)
    return arr > threshold


def connected_components(mask: np.ndarray) -> tuple[int, np.ndarray, np.ndarray, np.ndarray]:
    # OpenCV expects uint8 0/255 for connected components.
    u8 = np.where(mask, 255, 0).astype(np.uint8)
    return cv2.connectedComponentsWithStats(u8, connectivity=8)


def main() -> int:
    parser = argparse.ArgumentParser(description="Extract transistor candidates from poly/diffusion intersections.")
    parser.add_argument("--chip", action="append", choices=sorted(specs().keys()), help="Chip to extract (repeatable)")
    parser.add_argument("--all", action="store_true", help="Extract for all supported chips")
    parser.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "transistors")
    parser.add_argument("--threshold", type=int, default=128, help="Threshold for layer masks")
    parser.add_argument("--min-area", type=int, default=4, help="Filter out tiny components")
    args = parser.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = set(specs().keys())
    if not selected:
        parser.error("select --all or at least one --chip")

    args.out_dir.mkdir(parents=True, exist_ok=True)

    manifest: dict[str, object] = {
        "tool": "scripts/extract_transistors.py",
        "params": {"threshold": args.threshold, "min_area": args.min_area},
        "outputs": [],
    }

    for chip in sorted(selected):
        spec = specs()[chip]
        poly = load_mask(spec.poly_bmp, threshold=args.threshold)
        diffusion = load_mask(spec.diffusion_bmp, threshold=args.threshold)
        if poly.shape != diffusion.shape:
            raise SystemExit(f"{chip}: shape mismatch poly={poly.shape} diffusion={diffusion.shape}")

        inter = poly & diffusion
        n, labels, stats, centroids = connected_components(inter)

        comps: list[dict[str, object]] = []
        for label in range(1, n):  # 0 is background
            x, y, w, h, area = stats[label].tolist()
            if area < args.min_area:
                continue
            cx, cy = centroids[label].tolist()
            comps.append(
                {
                    "id": label,
                    "bbox": {"x": int(x), "y": int(y), "w": int(w), "h": int(h)},
                    "centroid": {"x": float(cx), "y": float(cy)},
                    "area_px": int(area),
                }
            )

        out_json = args.out_dir / f"{chip.lower()}_poly_diffusion_transistors.json"
        payload = {
            "chip": chip,
            "inputs": {
                "poly_bmp": str(spec.poly_bmp.relative_to(ROOT)),
                "diffusion_bmp": str(spec.diffusion_bmp.relative_to(ROOT)),
                "poly_sha256": sha256(spec.poly_bmp),
                "diffusion_sha256": sha256(spec.diffusion_bmp),
            },
            "params": {"threshold": args.threshold, "min_area": args.min_area},
            "counts": {"components_total": int(n - 1), "components_kept": int(len(comps))},
            "components": comps,
        }
        out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        manifest["outputs"].append({"chip": chip, "output": str(out_json.relative_to(ROOT))})

    (args.out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

