#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image


ROOT = Path(__file__).resolve().parents[1]


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Convert emulator bitmap artifacts (bmp/pbm/ppm/pgm) to PNG for viewing/annotation tools."
    )
    ap.add_argument(
        "--in-dir",
        type=Path,
        default=ROOT / "docs" / "emulators",
        help="Directory to scan for images.",
    )
    ap.add_argument(
        "--out-dir",
        type=Path,
        default=None,
        help="Output directory (defaults to --in-dir).",
    )
    ap.add_argument("--force", action="store_true", help="Overwrite existing PNGs.")
    args = ap.parse_args()

    in_dir = (ROOT / args.in_dir).resolve() if not args.in_dir.is_absolute() else args.in_dir
    out_dir = (ROOT / args.out_dir).resolve() if args.out_dir and not args.out_dir.is_absolute() else (args.out_dir or in_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    exts = {".bmp", ".pbm", ".pgm", ".ppm"}
    converted = 0
    skipped = 0
    failed = 0

    for src in sorted(in_dir.glob("*")):
        if src.suffix.lower() not in exts:
            continue
        dst = out_dir / (src.stem + ".png")
        if dst.exists() and not args.force:
            skipped += 1
            continue
        try:
            img = Image.open(src)
            img.save(dst, format="PNG", optimize=False, compress_level=9)
            converted += 1
        except Exception:
            failed += 1

    print(
        f"converted={converted} skipped={skipped} failed={failed} in_dir={rel_or_abs(in_dir)} out_dir={rel_or_abs(out_dir)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
