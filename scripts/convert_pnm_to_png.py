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
    parser.add_argument(
        "--flat",
        action="store_true",
        help="Flatten output names (default preserves relative paths under --out-dir).",
    )
    args = parser.parse_args()

    exts = {".pbm", ".pgm", ".ppm", ".pnm"}

    roots: list[Path] = [p for p in args.inputs if p.is_dir()]
    files: list[Path] = [p for p in args.inputs if not p.is_dir()]
    for root in roots:
        glob = "**/*" if args.recursive else "*"
        files.extend([q for q in root.glob(glob) if q.is_file() and q.suffix.lower() in exts])

    for inp in sorted(set(files)):
        if inp.suffix.lower() not in exts:
            continue
        out: Path
        if args.flat or not roots:
            out = args.out_dir / f"{inp.stem}.png"
        else:
            # Preserve relative paths to avoid collisions when converting many pages like `page-001.pbm`.
            root = next((r for r in roots if inp.is_relative_to(r)), None)
            if root is None:
                out = args.out_dir / f"{inp.stem}.png"
            else:
                rel = inp.relative_to(root)
                out = args.out_dir / root.name / rel.with_suffix(".png")
        convert_one(inp, out)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
