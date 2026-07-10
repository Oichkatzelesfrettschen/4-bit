#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path

import numpy as np
from PIL import Image

PNM_EXTS = {".pbm", ".pgm", ".ppm", ".pnm"}


def load_gray(path: Path) -> np.ndarray:
    """
    Load an image as uint8 grayscale.

    PIL supports PBM/PGM/PPM; we normalize all inputs to "L" so downstream
    OCR/preprocessing code doesn't need to special-case formats.
    """
    img = Image.open(path)
    return np.asarray(img.convert("L"))


def convert_pnm_to_png(inp: Path, out: Path) -> None:
    """
    Convert a PNM variant (PBM/PGM/PPM/PNM) to PNG.

    Useful because some tooling (including Codex image attachment) does not
    support the portable bitmap family.
    """
    img = Image.open(inp)
    if img.mode not in ("1", "L", "RGB", "RGBA"):
        img = img.convert("RGB")
    out.parent.mkdir(parents=True, exist_ok=True)
    img.save(out, format="PNG", optimize=False, compress_level=9)


def ensure_viewable(path: Path, *, out_dir: Path) -> Path:
    """
    If `path` is a PNM variant, convert it to PNG under `out_dir` and return the PNG path.
    Otherwise return the original path.
    """
    if path.suffix.lower() not in PNM_EXTS:
        return path
    out = out_dir / path.with_suffix(".png").name
    convert_pnm_to_png(path, out)
    return out

