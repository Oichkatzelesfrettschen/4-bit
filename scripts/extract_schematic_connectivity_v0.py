#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
from collections import defaultdict
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
    return (dark if dark.mean() < light.mean() else light).astype(np.uint8)


def try_font(size: int) -> ImageFont.ImageFont:
    try:
        return ImageFont.truetype("DejaVuSans.ttf", size=size)
    except Exception:
        return ImageFont.load_default()


_N4 = ((1, 0), (-1, 0), (0, 1), (0, -1))


def _neighbors4(x: int, y: int, *, w: int, h: int) -> list[tuple[int, int, int, int]]:
    out: list[tuple[int, int, int, int]] = []
    for dx, dy in _N4:
        nx, ny = x + dx, y + dy
        if 0 <= nx < w and 0 <= ny < h:
            out.append((nx, ny, dx, dy))
    return out


def _degree4(skel: np.ndarray, x: int, y: int) -> int:
    h, w = skel.shape
    deg = 0
    for nx, ny, _dx, _dy in _neighbors4(x, y, w=w, h=h):
        if skel[ny, nx]:
            deg += 1
    return deg


def _dot_score(mask: np.ndarray, x: int, y: int, *, r: int) -> float:
    h, w = mask.shape
    x0 = max(0, x - r)
    y0 = max(0, y - r)
    x1 = min(w, x + r + 1)
    y1 = min(h, y + r + 1)
    win = mask[y0:y1, x0:x1]
    return float(win.mean())


def _find_seed(skel: np.ndarray, x: int, y: int, *, max_r: int) -> tuple[int, int] | None:
    h, w = skel.shape
    if 0 <= x < w and 0 <= y < h and skel[y, x]:
        return (x, y)
    for r in range(1, max_r + 1):
        x0 = max(0, x - r)
        y0 = max(0, y - r)
        x1 = min(w, x + r + 1)
        y1 = min(h, y + r + 1)
        ys, xs = np.where(skel[y0:y1, x0:x1] > 0)
        if xs.size == 0:
            continue
        # Choose nearest pixel (deterministic tie-break by y,x)
        best = None
        for yy, xx in sorted(zip(ys.tolist(), xs.tolist()), key=lambda t: (t[0], t[1])):
            px, py = x0 + int(xx), y0 + int(yy)
            d = (px - x) * (px - x) + (py - y) * (py - y)
            if best is None or d < best[0]:
                best = (d, px, py)
        assert best is not None
        return (int(best[1]), int(best[2]))
    return None


def trace_net(
    *,
    skel: np.ndarray,
    mask: np.ndarray,
    seed: tuple[int, int],
    max_states: int,
    dot_r: int,
    dot_thresh: float,
    point_by_pos: dict[tuple[int, int], list[str]],
) -> dict[str, object]:
    """
    Trace schematic connectivity on a thinned skeleton, treating 4-way crossings without a junction dot as non-connecting.

    We keep direction in the state to allow “go straight through crossing” behavior without incorrectly turning.
    """
    h, w = skel.shape
    sx, sy = seed
    # state: (x, y, dx, dy) where (dx,dy) is incoming direction; (0,0) for start.
    q: list[tuple[int, int, int, int]] = [(sx, sy, 0, 0)]
    seen: set[tuple[int, int, int, int]] = set()
    pixels: set[tuple[int, int]] = set()
    hits: set[str] = set()
    junctions = 0
    crossings = 0

    while q and len(seen) < max_states:
        x, y, in_dx, in_dy = q.pop()
        st = (x, y, in_dx, in_dy)
        if st in seen:
            continue
        seen.add(st)
        if not skel[y, x]:
            continue
        pixels.add((x, y))
        for name in point_by_pos.get((x, y), []):
            hits.add(name)

        deg = _degree4(skel, x, y)
        if deg >= 3:
            # decide if this is a junction dot vs an unmarked crossing
            score = _dot_score(mask, x, y, r=dot_r)
            is_dot = score >= dot_thresh
            if is_dot:
                junctions += 1
            else:
                crossings += 1

        for nx, ny, dx, dy in _neighbors4(x, y, w=w, h=h):
            if not skel[ny, nx]:
                continue
            # Don't go backwards.
            if in_dx != 0 or in_dy != 0:
                if dx == -in_dx and dy == -in_dy:
                    continue

            deg_here = _degree4(skel, x, y)
            if deg_here >= 3:
                score = _dot_score(mask, x, y, r=dot_r)
                is_dot = score >= dot_thresh
                if not is_dot and in_dx != 0 and in_dy != 0:
                    # At an unmarked crossing, do not turn: go straight only.
                    if (dx, dy) != (in_dx, in_dy):
                        continue
            q.append((nx, ny, dx, dy))

    xs = [p[0] for p in pixels]
    ys = [p[1] for p in pixels]
    bb = None
    if xs and ys:
        bb = {"x0": min(xs), "y0": min(ys), "x1": max(xs), "y1": max(ys)}

    return {
        "seed": {"x": int(sx), "y": int(sy)},
        "counts": {
            "states": int(len(seen)),
            "pixels": int(len(pixels)),
            "junctions_est": int(junctions),
            "crossings_est": int(crossings),
        },
        "bbox": bb,
        "hits": sorted(hits),
        # store pixels only for debugging, potentially huge; callers can opt-in to render instead.
    }


def main() -> int:
    p = argparse.ArgumentParser(description="Trace schematic connectivity on a skeleton for selected signals (v0).")
    p.add_argument("--chip", required=True, choices=sorted(specs().keys()))
    p.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "schematic_connectivity_v0")
    p.add_argument("--threshold", type=int, default=200)
    p.add_argument("--polarity", choices=["auto", "dark", "light"], default="auto")
    p.add_argument("--seed-radius", type=int, default=24, help="Max radius for finding nearest skeleton pixel to a point")
    p.add_argument("--name-regex", type=str, default=".*", help="Only trace signals whose names match this regex")
    p.add_argument("--max-states", type=int, default=250_000, help="Max BFS states per traced net (safety bound)")
    p.add_argument("--dot-radius", type=int, default=2, help="Neighborhood radius for junction-dot scoring")
    p.add_argument("--dot-thresh", type=float, default=0.55, help="Mean mask density to classify a junction dot")
    p.add_argument("--render", action="store_true", help="Render a debug overlay for traced nets")
    args = p.parse_args()

    spec = specs()[args.chip]
    img = cv2.imread(str(spec.schematic_bmp), cv2.IMREAD_GRAYSCALE)
    if img is None:
        raise SystemExit(f"failed to read {spec.schematic_bmp}")
    mask = load_mask(img, threshold=int(args.threshold), polarity=str(args.polarity))

    # Skeletonize (OpenCV contrib thinning expects 0/255).
    u8 = np.where(mask > 0, 255, 0).astype(np.uint8)
    skel_u8 = cv2.ximgproc.thinning(u8)
    skel = (skel_u8 > 0).astype(np.uint8)

    points = parse_signals_txt(spec.signals_txt)
    # Map each point to a skeleton seed; build reverse map for “hits”.
    point_by_pos: dict[tuple[int, int], list[str]] = defaultdict(list)
    point_seeds: list[tuple[int, int] | None] = []
    for pt in points:
        x = int(pt["x"])
        y = int(pt["y"])
        s = _find_seed(skel, x, y, max_r=int(args.seed_radius))
        point_seeds.append(s)
        if s is not None:
            point_by_pos[s].append(str(pt["name"]))

    pat = re.compile(str(args.name_regex))
    targets: list[dict[str, object]] = []
    for idx, pt in enumerate(points):
        name = str(pt["name"])
        if not pat.search(name):
            continue
        seed = point_seeds[idx]
        if seed is None:
            targets.append({"name": name, "point": {"x": int(pt["x"]), "y": int(pt["y"])}, "error": "no_seed"})
            continue
        res = trace_net(
            skel=skel,
            mask=mask,
            seed=seed,
            max_states=int(args.max_states),
            dot_r=int(args.dot_radius),
            dot_thresh=float(args.dot_thresh),
            point_by_pos=point_by_pos,
        )
        targets.append({"name": name, "point": {"x": int(pt["x"]), "y": int(pt["y"])}, **res})

    out_chip = Path(args.out_dir) / spec.chip
    out_chip.mkdir(parents=True, exist_ok=True)
    out_json = out_chip / f"{spec.chip.lower()}_schematic_connectivity_v0.json"
    payload = {
        "chip": spec.chip,
        "schema": {"version": 0, "description": "Skeleton-based schematic connectivity traces for selected signals."},
        "inputs": {
            "schematic_bmp": rel_or_abs(spec.schematic_bmp),
            "signals_txt": rel_or_abs(spec.signals_txt),
            "sha256": {"schematic_bmp": sha256(spec.schematic_bmp), "signals_txt": sha256(spec.signals_txt)},
        },
        "params": {
            "threshold": int(args.threshold),
            "polarity": str(args.polarity),
            "seed_radius": int(args.seed_radius),
            "name_regex": str(args.name_regex),
            "max_states": int(args.max_states),
            "dot_radius": int(args.dot_radius),
            "dot_thresh": float(args.dot_thresh),
        },
        "counts": {"targets": int(len(targets))},
        "targets": targets,
    }
    out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    out_png = None
    if bool(args.render):
        pil = Image.fromarray(img).convert("RGB")
        d = ImageDraw.Draw(pil)
        font = try_font(16)
        for t in targets:
            name = str(t.get("name", ""))
            pt = t.get("point", {}) if isinstance(t.get("point"), dict) else {}
            x = int(pt.get("x", 0))
            y = int(pt.get("y", 0))
            d.ellipse([x - 3, y - 3, x + 3, y + 3], outline=(255, 0, 0), width=2)
            d.text((x + 6, y - 6), name, fill=(255, 0, 0), font=font)
            bb = t.get("bbox")
            if isinstance(bb, dict):
                d.rectangle([bb["x0"], bb["y0"], bb["x1"], bb["y1"]], outline=(0, 200, 0), width=2)
        out_png = out_chip / f"{spec.chip.lower()}_schematic_connectivity_v0.png"
        pil.save(out_png, format="PNG", optimize=False, compress_level=9)

    print(json.dumps({"out_json": str(out_json), "out_png": str(out_png) if out_png else None}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

