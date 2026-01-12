#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path

import cv2
import numpy as np
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class ChipSpec:
    chip: str
    schematic_bmp: Path
    signals_txt: Path


def specs() -> dict[str, ChipSpec]:
    emu = ROOT / "docs" / "emulators"
    return {
        "4001": ChipSpec("4001", emu / "i4001-schematic.bmp", emu / "i4001-signals.txt"),
        "4002": ChipSpec("4002", emu / "i4002-schematic.bmp", emu / "i4002-signals.txt"),
        "4003": ChipSpec("4003", emu / "i4003-schematic.bmp", emu / "i4003-signals.txt"),
        "4004": ChipSpec("4004", emu / "i4004-schematic.bmp", emu / "i4004-signals.txt"),
    }


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def parse_signals_txt(path: Path) -> list[dict[str, object]]:
    out: list[dict[str, object]] = []
    for raw in path.read_text(errors="replace").splitlines():
        line = raw.strip().strip("\r")
        if not line or line.startswith(";"):
            continue
        parts = [p.strip() for p in line.split(",")]
        if len(parts) != 3:
            continue
        try:
            x = int(parts[0])
            y = int(parts[1])
        except ValueError:
            continue
        out.append({"x": x, "y": y, "name": parts[2]})
    return out


def load_mask(gray: np.ndarray, *, threshold: int, polarity: str) -> np.ndarray:
    if polarity == "dark":
        return (gray < threshold).astype(np.uint8)
    if polarity == "light":
        return (gray > threshold).astype(np.uint8)
    if polarity != "auto":
        raise ValueError(f"unknown polarity: {polarity}")

    light = gray > threshold
    dark = ~light
    # Prefer the minority class as the “ink”.
    return (dark if dark.mean() < light.mean() else light).astype(np.uint8)


def connected_components(mask_u8: np.ndarray) -> tuple[int, np.ndarray, np.ndarray, np.ndarray]:
    u8 = np.where(mask_u8 > 0, 255, 0).astype(np.uint8)
    return cv2.connectedComponentsWithStats(u8, connectivity=8)


def _mode_label(labels: np.ndarray) -> int | None:
    vals = labels.ravel()
    vals = vals[vals > 0]
    if vals.size == 0:
        return None
    counts = Counter(int(v) for v in vals.tolist())
    # deterministic tie-break
    return sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))[0][0]


def try_font(size: int) -> ImageFont.ImageFont:
    try:
        return ImageFont.truetype("DejaVuSans.ttf", size=size)
    except Exception:
        return ImageFont.load_default()


def main() -> int:
    p = argparse.ArgumentParser(
        description="Extract schematic-space pixel connectivity components and assign signals.txt points to component IDs (v0)."
    )
    p.add_argument("--chip", action="append", choices=sorted(specs().keys()), help="Chip to extract (repeatable)")
    p.add_argument("--all", action="store_true", help="Extract for all supported chips")
    p.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "schematic_wirenets_v0")
    p.add_argument("--threshold", type=int, default=200, help="Binarization threshold for schematic bitmap")
    p.add_argument(
        "--polarity",
        choices=["auto", "dark", "light"],
        default="auto",
        help="Mask polarity: 'dark' means ink is dark pixels, 'light' means ink is light pixels.",
    )
    p.add_argument("--sample", type=int, default=9, help="Odd-sized sampling window around each point")
    p.add_argument("--render", action="store_true", help="Render a debug overlay image per chip")
    args = p.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = set(specs().keys())
    if not selected:
        p.error("select --all or at least one --chip")

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    manifest: dict[str, object] = {"tool": "scripts/extract_schematic_wirenets_v0.py", "params": {}, "outputs": []}

    sample = int(args.sample)
    if sample <= 0 or sample % 2 == 0:
        raise SystemExit("--sample must be a positive odd integer")
    rad = sample // 2

    for chip in sorted(selected):
        spec = specs()[chip]
        img = cv2.imread(str(spec.schematic_bmp), cv2.IMREAD_GRAYSCALE)
        if img is None:
            raise SystemExit(f"failed to read {spec.schematic_bmp}")
        h, w = img.shape
        mask = load_mask(img, threshold=int(args.threshold), polarity=str(args.polarity))

        n, labels, stats, _centroids = connected_components(mask)
        comp_count = int(n - 1)

        pts = parse_signals_txt(spec.signals_txt)
        points: list[dict[str, object]] = []
        by_comp: dict[int, list[int]] = defaultdict(list)
        unmapped: list[int] = []
        for idx, pt in enumerate(pts):
            x = int(pt["x"])
            y = int(pt["y"])
            x0 = max(0, x - rad)
            y0 = max(0, y - rad)
            x1 = min(w, x + rad + 1)
            y1 = min(h, y + rad + 1)
            lab = _mode_label(labels[y0:y1, x0:x1])
            if lab is None:
                unmapped.append(idx)
            else:
                by_comp[int(lab)].append(idx)
            points.append(
                {
                    "idx": idx,
                    "x": x,
                    "y": y,
                    "name": str(pt["name"]),
                    "schematic_component": int(lab) if lab is not None else None,
                }
            )

        # Only emit component stats for components that are referenced by at least one point.
        comps = []
        for lab, idxs in sorted(by_comp.items(), key=lambda kv: kv[0]):
            x, y, ww, hh, area = (int(v) for v in stats[int(lab)].tolist())
            comps.append({"id": int(lab), "bbox": {"x": x, "y": y, "w": ww, "h": hh}, "area": area, "point_indices": idxs})

        # Build schematic “nets” as (component_id → set of signal names).
        nets = []
        for lab, idxs in sorted(by_comp.items(), key=lambda kv: kv[0]):
            names = sorted({str(points[i]["name"]) for i in idxs})
            nets.append({"schematic_component": int(lab), "names": names, "point_indices": idxs})

        out_chip = out_dir / chip
        out_chip.mkdir(parents=True, exist_ok=True)
        out_json = out_chip / f"{chip.lower()}_schematic_wirenets_v0.json"
        payload = {
            "chip": chip,
            "schema": {
                "version": 0,
                "description": "Schematic bitmap connected-components and signals.txt point→component assignment.",
            },
            "inputs": {
                "schematic_bmp": rel_or_abs(spec.schematic_bmp),
                "signals_txt": rel_or_abs(spec.signals_txt),
                "sha256": {"schematic_bmp": sha256(spec.schematic_bmp), "signals_txt": sha256(spec.signals_txt)},
            },
            "params": {"threshold": int(args.threshold), "polarity": str(args.polarity), "sample": int(sample)},
            "counts": {
                "components_total": int(comp_count),
                "components_referenced": int(len(comps)),
                "nets": int(len(nets)),
                "signal_points": int(len(points)),
                "points_unmapped": int(len(unmapped)),
            },
            "components": comps,
            "nets": nets,
            "points": points,
            "unmapped_point_indices": unmapped,
        }
        out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")

        out_png = None
        if bool(args.render):
            pil = Image.fromarray(img).convert("RGB")
            d = ImageDraw.Draw(pil)
            font = try_font(14)
            # Draw component bboxes for referenced components only.
            for c in comps:
                bb = c["bbox"]
                x0 = int(bb["x"])
                y0 = int(bb["y"])
                x1 = x0 + int(bb["w"])
                y1 = y0 + int(bb["h"])
                d.rectangle([x0, y0, x1, y1], outline=(0, 200, 255), width=2)
                d.text((x0 + 2, y0 + 2), str(c["id"]), fill=(0, 140, 200), font=font)
            # Draw points.
            for r in points:
                x = int(r["x"])
                y = int(r["y"])
                lab = r.get("schematic_component")
                color = (0, 255, 0) if lab is not None else (255, 0, 0)
                d.ellipse([x - 2, y - 2, x + 2, y + 2], fill=color)
            out_png = out_chip / f"{chip.lower()}_schematic_wirenets_v0.png"
            pil.save(out_png, format="PNG", optimize=False, compress_level=9)

        manifest["outputs"].append(
            {
                "chip": chip,
                "output_json": rel_or_abs(out_json),
                "output_png": rel_or_abs(out_png) if out_png else None,
            }
        )

    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
