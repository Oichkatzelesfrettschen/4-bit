#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def main() -> int:
    p = argparse.ArgumentParser(description="Build an OCR manifest for layout pad-label human crops (v0).")
    p.add_argument("--chip", required=True, choices=["4001", "4002", "4003", "4004"])
    p.add_argument("--in-dir", type=Path, default=ROOT / "docs" / "evidence" / "layout_pad_labels_v0")
    p.add_argument("--out", type=Path, required=True)
    args = p.parse_args()

    chip = str(args.chip)
    in_dir = args.in_dir
    if not in_dir.is_absolute():
        in_dir = (ROOT / in_dir).resolve()
    crops_dir = in_dir / chip / "human_crops"
    if not crops_dir.is_dir():
        raise SystemExit(f"missing crops dir: {crops_dir}")

    entries = []
    for img in sorted(crops_dir.glob("box_*.png")):
        entries.append({"id": img.stem, "image": rel_or_abs(img), "expected": ""})

    out = args.out
    if not out.is_absolute():
        out = (ROOT / out).resolve()
    out.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "schema": {"version": 0, "description": "Unlabeled OCR manifest for pad-label human crops (v0)."},
        "inputs": {"crops_dir": rel_or_abs(crops_dir)},
        "chip": chip,
        "counts": {"entries": int(len(entries))},
        "entries": entries,
    }
    out.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

