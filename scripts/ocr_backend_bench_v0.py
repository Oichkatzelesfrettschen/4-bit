#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
import time
from pathlib import Path
from typing import Any

import cv2  # type: ignore

from ocr_backend_v0 import resolve_backend

ROOT = Path(__file__).resolve().parents[1]


def _rel(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def _expected_from_filename(path: Path) -> str:
    # Example: 007_T_node0_conf92.0.png → T
    m = re.search(r"_([A-Za-z0-9]+)_node", path.name)
    return (m.group(1) if m else "").upper()


def main() -> int:
    p = argparse.ArgumentParser(description="Benchmark OCR backend stack on labeled edge-label crops (v0).")
    p.add_argument("--glob", default="docs/evidence/layout_edge_labels_v0/*/crops/*.png")
    p.add_argument(
        "--backend",
        default="tesseract_cli_fast",
        choices=["auto", "tesseract", "tesseract_cli", "tesseract_cli_fast", "onnx", "template"],
    )
    p.add_argument("--onnx-model", type=Path, default=None)
    p.add_argument("--prefer-cuda", action="store_true")
    p.add_argument("--out", type=Path, default=ROOT / "docs" / "evidence" / "ocr_benchmarks_v0" / "backend_bench_v0.json")
    p.add_argument("--limit", type=int, default=200)
    args = p.parse_args()

    paths = sorted(ROOT.glob(str(args.glob)))
    if args.limit > 0:
        paths = paths[: int(args.limit)]

    backend = resolve_backend(backend=args.backend, onnx_model=args.onnx_model, prefer_cuda=bool(args.prefer_cuda))
    rows: list[dict[str, Any]] = []
    ok = 0
    total = 0

    for path in paths:
        exp = _expected_from_filename(path)
        img = cv2.imread(str(path), cv2.IMREAD_GRAYSCALE)
        if img is None:
            continue
        t0 = time.perf_counter()
        res = backend.best_token(
            img,
            whitelist="ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
            psms=(6, 7, 8, 10, 11, 13),
            oem=1,
            min_len=1,
            max_len=max(1, len(exp) or 4),
        )
        dt_ms = (time.perf_counter() - t0) * 1000.0
        total += 1
        match = bool(exp) and res.token == exp
        ok += 1 if match else 0
        rows.append(
            {
                "path": _rel(path),
                "expected": exp,
                "got": res.token,
                "conf": float(res.conf),
                "psm": int(res.psm),
                "invert": bool(res.invert),
                "scale": int(res.scale),
                "ms": float(dt_ms),
                "ok": bool(match),
                "backend": getattr(backend, "name", "unknown"),
            }
        )

    out = {
        "schema": {"version": 0, "description": "OCR backend micro-benchmark on pre-labeled edge crops."},
        "tool": "scripts/ocr_backend_bench_v0.py",
        "params": {
            "glob": str(args.glob),
            "backend": str(args.backend),
            "onnx_model": _rel(args.onnx_model) if args.onnx_model else None,
            "prefer_cuda": bool(args.prefer_cuda),
            "limit": int(args.limit),
        },
        "results": {
            "total": int(total),
            "correct": int(ok),
            "accuracy": float(ok / total) if total else 0.0,
        },
        "rows": rows,
    }

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(out, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(str(args.out))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
