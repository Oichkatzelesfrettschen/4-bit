#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image


ROOT = Path(__file__).resolve().parents[1]


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def convert_one(src: Path, *, dry_run: bool) -> Path | None:
    dst = src.with_suffix(".png")
    try:
        src_mtime = src.stat().st_mtime
        dst_mtime = dst.stat().st_mtime if dst.exists() else -1
    except Exception:
        src_mtime = 0
        dst_mtime = -1

    if dst.exists() and dst_mtime >= src_mtime:
        return None

    if dry_run:
        return dst

    with Image.open(src) as im:
        # Normalize to something trivially viewable in GitHub/CLI.
        out = im.convert("RGB") if im.mode not in ("L", "RGB") else im
        out.save(dst, format="PNG", optimize=True)
    return dst


def main() -> int:
    p = argparse.ArgumentParser(description="Ensure PNG previews exist for non-preview-friendly image formats (v0).")
    p.add_argument("--root", type=Path, default=ROOT / "docs", help="Search root (default: docs/)")
    p.add_argument("--ext", action="append", default=[".bmp", ".pbm"], help="Source extension to convert (repeatable)")
    p.add_argument("--dry-run", action="store_true", help="Print planned conversions without writing")
    args = p.parse_args()

    exts = {e.lower() if e.startswith(".") else f".{e.lower()}" for e in (args.ext or [])}
    root = Path(args.root)
    if not root.is_absolute():
        root = (ROOT / root).resolve()
    if not root.exists():
        raise SystemExit(f"missing root: {root}")

    converted = 0
    for src in sorted(root.rglob("*")):
        if not src.is_file():
            continue
        if src.suffix.lower() not in exts:
            continue
        dst = convert_one(src, dry_run=bool(args.dry_run))
        if dst is None:
            continue
        converted += 1
        print(f"{rel_or_abs(src)} -> {rel_or_abs(dst)}")

    if converted == 0:
        print("no conversions needed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

