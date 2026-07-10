#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def _rel(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def _expected_from_filename(path: Path) -> str:
    # Common patterns:
    # - edge labels: 007_T_node0_conf92.0.png → T
    # - pad crops: box_100_node_595.png (expected unknown; leave empty)
    m = re.search(r"_([A-Za-z0-9]+)_node", path.name)
    if m:
        return (m.group(1) or "").upper()
    return ""


def _classify(token: str) -> str:
    t = (token or "").strip().upper()
    if not t:
        return "unknown"
    if t.isdigit():
        return "digits"
    if len(t) == 1 and t.isalnum():
        return "glyph"
    if t.isalnum():
        return "alnum"
    return "other"


def main() -> int:
    p = argparse.ArgumentParser(description="Build a small OCR crop manifest from existing evidence images (v0).")
    p.add_argument("--glob", action="append", default=["docs/evidence/layout_edge_labels_v0/*/crops/*.png"])
    p.add_argument("--limit", type=int, default=500, help="Max total crops to include")
    p.add_argument("--out", type=Path, default=ROOT / "docs/evidence/ocr_crops_v0/manifest.json")
    args = p.parse_args()

    paths: list[Path] = []
    for g in args.glob:
        paths.extend(sorted(ROOT.glob(str(g))))
    paths = paths[: max(0, int(args.limit))]

    rows = []
    for path in paths:
        exp = _expected_from_filename(path)
        rows.append({"path": _rel(path), "expected": exp, "class": _classify(exp)})

    out = args.out
    if not out.is_absolute():
        out = (ROOT / out).resolve()
    out.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "schema": {"version": 0, "description": "OCR crop manifest built from existing labeled evidence images."},
        "tool": "scripts/build_ocr_crop_manifest_v0.py",
        "params": {"glob": list(args.glob), "limit": int(args.limit)},
        "counts": {"rows": int(len(rows))},
        "rows": rows,
    }
    out.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(_rel(out))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

