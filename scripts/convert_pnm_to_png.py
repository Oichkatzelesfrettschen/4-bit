#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image


def convert_one(inp: Path, out: Path) -> None:
    img = Image.open(inp)
    # PBM/PGM/PPM can open in a variety of modes; normalize so PNG writers behave.
    if img.mode not in ("1", "L", "RGB", "RGBA"):
        img = img.convert("RGB")
    out.parent.mkdir(parents=True, exist_ok=True)
    img.save(out, format="PNG", optimize=False, compress_level=9)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Convert PBM/PGM/PPM/PNM images to PNG (handy for tools that can't display PNM variants)."
    )
    parser.add_argument("inputs", nargs="+", type=Path, help="Input files or directories")
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path.cwd(),
        help="Output directory (defaults to current directory)",
    )
    parser.add_argument(
        "--recursive",
        action="store_true",
        help="Recurse into directories",
    )
    args = parser.parse_args()

    exts = {".pbm", ".pgm", ".ppm", ".pnm"}

    inputs: list[Path] = []
    for p in args.inputs:
        if p.is_dir():
            glob = "**/*" if args.recursive else "*"
            inputs.extend([q for q in p.glob(glob) if q.is_file() and q.suffix.lower() in exts])
        else:
            inputs.append(p)

    for inp in sorted(set(inputs)):
        if inp.suffix.lower() not in exts:
            continue
        out = args.out_dir / f"{inp.stem}.png"
        convert_one(inp, out)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

