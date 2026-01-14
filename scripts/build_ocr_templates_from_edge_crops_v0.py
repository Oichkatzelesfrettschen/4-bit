#!/usr/bin/env python3
"""
Build OCR templates from existing edge-label crop PNGs.

This complements build_ocr_templates_v0.py (which uses confirmed crops from
manual_readings_v0.md). Edge-label crops already encode the expected token in
their filename, so we can bootstrap a template bank for tokens like:
  T, C, S, G, V, RM, R0..R3, 01, 02, ...

We intentionally keep ONE best template per token (highest parsed 'conf' from
the filename) to avoid ambiguous overwrites in TemplateDirBackend.
"""

from __future__ import annotations

import argparse
import re
import shutil
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Candidate:
    path: Path
    token: str
    conf: float


def _parse_token(path: Path) -> str | None:
    # Typical crop name: 003_R2_node518_conf79.0.png
    m = re.search(r"_([A-Z0-9]+)_NODE", path.name.upper())
    if not m:
        return None
    tok = m.group(1).strip().upper()
    return tok or None


def _parse_conf(path: Path) -> float:
    m = re.search(r"CONF([0-9]+(?:\\.[0-9]+)?)", path.name.upper())
    if not m:
        return 0.0
    try:
        return float(m.group(1))
    except ValueError:
        return 0.0


def main() -> int:
    ap = argparse.ArgumentParser(description="Build template OCR directory from edge-label crops (v0).")
    ap.add_argument(
        "--glob",
        default="docs/evidence/layout_edge_labels_v0/**/crops/*.png",
        help="Glob of edge-label crop PNGs (default: docs/evidence/layout_edge_labels_v0/**/crops/*.png).",
    )
    ap.add_argument(
        "--out-dir",
        default="docs/evidence/ocr_models/templates_v0",
        help="Directory to write templates into (set OCR_TEMPLATE_DIR to this).",
    )
    ap.add_argument("--min-conf", type=float, default=0.0, help="Minimum parsed conf (from filename) to accept.")
    args = ap.parse_args()

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    best: dict[str, Candidate] = {}
    paths = sorted(Path(".").glob(args.glob))
    for p in paths:
        if not p.is_file():
            continue
        tok = _parse_token(p)
        if tok is None:
            continue
        conf = _parse_conf(p)
        if conf < float(args.min_conf):
            continue
        prev = best.get(tok)
        if prev is None or conf > prev.conf:
            best[tok] = Candidate(path=p, token=tok, conf=conf)

    if not best:
        print(f"no usable crops found for glob {args.glob}")
        return 2

    wrote = 0
    for tok, cand in sorted(best.items()):
        dst = out_dir / f"tok_{tok}_edge_conf{cand.conf:.1f}.png"
        shutil.copyfile(cand.path, dst)
        wrote += 1

    print(f"wrote {wrote} edge-label templates to {out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

