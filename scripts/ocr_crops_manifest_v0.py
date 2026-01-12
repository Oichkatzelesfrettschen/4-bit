#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class Entry:
    id: str
    image: str
    expected: str


_CROP_RE = re.compile(r"^(?P<idx>\d+?)_(?P<tok>[A-Z0-9]+?)_node(?P<node>\d+?)_conf(?P<conf>[-0-9.]+)\.png$")


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def main() -> int:
    p = argparse.ArgumentParser(description="Build a labeled OCR manifest from crop filenames (v0).")
    p.add_argument("--crops-dir", type=Path, required=True, help="Directory containing labeled crops like *_TOK_nodeN_confX.png")
    p.add_argument("--out", type=Path, required=True, help="Output JSON path")
    p.add_argument(
        "--dedupe",
        action="store_true",
        help="De-duplicate by (expected, node) keeping the highest-conf crop.",
    )
    args = p.parse_args()

    crops_dir = args.crops_dir
    if not crops_dir.is_dir():
        raise SystemExit(f"not a directory: {crops_dir}")

    entries: list[Entry] = []
    best_by_key: dict[tuple[str, int], tuple[float, Path]] = {}
    for img in sorted(crops_dir.glob("*.png")):
        m = _CROP_RE.match(img.name)
        if not m:
            continue
        tok = str(m.group("tok"))
        node = int(m.group("node"))
        conf = float(m.group("conf"))
        if args.dedupe:
            key = (tok, node)
            cur = best_by_key.get(key)
            if cur is None or conf > cur[0]:
                best_by_key[key] = (conf, img)
        else:
            entries.append(Entry(id=img.stem, image=rel_or_abs(img), expected=tok))

    if args.dedupe:
        for (tok, node), (_, img) in sorted(best_by_key.items(), key=lambda kv: (kv[0][0], kv[0][1])):
            entries.append(Entry(id=img.stem, image=rel_or_abs(img), expected=tok))

    out = args.out
    out.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "schema": {"version": 0, "description": "Labeled OCR crops manifest (derived from filenames)."},
        "inputs": {"crops_dir": rel_or_abs(crops_dir)},
        "counts": {"entries": len(entries)},
        "entries": [e.__dict__ for e in entries],
    }
    out.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

