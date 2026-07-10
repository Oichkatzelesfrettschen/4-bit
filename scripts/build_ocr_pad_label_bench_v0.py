#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


def _rel(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


_RE_EXPECTED = re.compile(r"_([A-Za-z0-9]+)_node", flags=re.IGNORECASE)
_RE_CHIP = re.compile(r"/(400[1-4])/", flags=re.IGNORECASE)
_RE_CONF = re.compile(r"_conf([0-9]+(?:\\.[0-9]+)?)", flags=re.IGNORECASE)


@dataclass(frozen=True)
class Crop:
    path: Path
    token: str
    chip: str | None
    conf: float


def _parse_crop(path: Path) -> Crop | None:
    m = _RE_EXPECTED.search(path.name)
    if not m:
        return None
    tok = re.sub(r"[^A-Za-z0-9]", "", (m.group(1) or "").upper())
    if not tok:
        return None
    mchip = _RE_CHIP.search(path.as_posix())
    chip = (mchip.group(1) if mchip else None)
    mconf = _RE_CONF.search(path.name)
    conf = float(mconf.group(1)) if mconf else -1.0
    return Crop(path=path, token=tok, chip=chip, conf=conf)


def main() -> int:
    p = argparse.ArgumentParser(description="Build a small OCR benchmark JSON from labeled crop filenames (v0).")
    p.add_argument(
        "--glob",
        action="append",
        default=["docs/evidence/layout_edge_labels_v0/*/crops/*.png"],
        help="Glob(s) to include (repeatable). Expected token must be encoded in filename as *_<TOK>_node*.",
    )
    p.add_argument("--per-token", type=int, default=8, help="Max samples per expected token")
    p.add_argument("--limit", type=int, default=250, help="Max total items (after per-token filtering)")
    p.add_argument(
        "--out",
        type=Path,
        default=ROOT / "docs" / "evidence" / "ocr_benchmarks_v0" / "pad_label_tokens_v0.json",
        help="Output benchmark JSON path",
    )
    args = p.parse_args()

    crops: list[Crop] = []
    for g in args.glob:
        for pth in sorted(ROOT.glob(str(g))):
            if not pth.is_file():
                continue
            c = _parse_crop(pth)
            if c is None:
                continue
            crops.append(c)

    # Prefer high-confidence crops when available.
    crops.sort(key=lambda c: (c.token, c.conf), reverse=True)

    per_tok: dict[str, list[Crop]] = {}
    for c in crops:
        bucket = per_tok.setdefault(c.token, [])
        if len(bucket) >= int(args.per_token):
            continue
        bucket.append(c)

    items: list[dict[str, Any]] = []
    for tok in sorted(per_tok.keys()):
        for c in per_tok[tok]:
            items.append(
                {
                    "id": f"{c.chip or 'unknown'}/{tok}/{c.path.name}",
                    "expected": tok,
                    "image": _rel(c.path),
                }
            )
            if 0 < int(args.limit) <= len(items):
                break
        if 0 < int(args.limit) <= len(items):
            break

    out = {
        "schema": {
            "version": 0,
            "description": "Pad/edge label OCR micro-benchmark built from pre-labeled crop filenames.",
        },
        "tool": "scripts/build_ocr_pad_label_bench_v0.py",
        "params": {
            "glob": list(args.glob),
            "per_token": int(args.per_token),
            "limit": int(args.limit),
        },
        "items": items,
    }

    out_path = args.out
    if not out_path.is_absolute():
        out_path = (ROOT / out_path).resolve()
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(out, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(_rel(out_path))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

