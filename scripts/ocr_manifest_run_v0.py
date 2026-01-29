#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import time
from pathlib import Path

import numpy as np

from ocr_cached_backend_v0 import resolve_cached_backend
from imgio_v0 import load_gray
from ocr_presets_v0 import preset_layout_edge_label

ROOT = Path(__file__).resolve().parents[1]


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def main() -> int:
    p = argparse.ArgumentParser(description="Run OCR over a labeled crops manifest (v0).")
    p.add_argument("--manifest", type=Path, required=True, help="Manifest JSON from ocr_crops_manifest_v0.py")
    p.add_argument(
        "--backend",
        default="auto",
        choices=["auto", "tesseract", "onnx"],
        help="OCR backend (auto prefers ONNX/CUDA when configured, else Tesseract).",
    )
    p.add_argument(
        "--adaptive-whitelist",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Derive per-item whitelist from expected token (keeps OCR tight for known labels).",
    )
    p.add_argument(
        "--whitelist",
        default="ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
        help="Fallback whitelist when expected token is missing/empty.",
    )
    p.add_argument(
        "--onnx-model",
        type=Path,
        default=None,
        help="Path to ONNX model for --backend onnx/auto (or set OCR_ONNX_MODEL).",
    )
    p.add_argument(
        "--prefer-cuda",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Prefer CUDAExecutionProvider for ONNX backends when available.",
    )
    p.add_argument("--out", type=Path, default=None, help="Write report JSON here (default: stdout)")
    args = p.parse_args()

    manifest_path = args.manifest
    data = json.loads(manifest_path.read_text(encoding="utf-8"))
    entries = data.get("entries", [])
    if not isinstance(entries, list):
        raise SystemExit("manifest missing entries[]")

    backend = resolve_cached_backend(backend=str(args.backend), onnx_model=args.onnx_model, prefer_cuda=bool(args.prefer_cuda))

    fallback_whitelist = str(args.whitelist).strip().upper() or "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    results: list[dict] = []
    t0 = time.perf_counter()
    for e in entries:
        if not isinstance(e, dict):
            continue
        img_path = Path(str(e.get("image", "")))
        if not img_path.is_absolute():
            img_path = ROOT / img_path
        expected = str(e.get("expected", "")).strip().upper()
        if not img_path.exists():
            continue
        gray = load_gray(img_path)
        t_img0 = time.perf_counter()
        whitelist = fallback_whitelist
        if bool(args.adaptive_whitelist) and expected:
            # Keep digits available even if a particular expected token is letters-only.
            whitelist = "".join(sorted(set(expected + "0123456789")))
        preset = preset_layout_edge_label(expected=expected)
        r = backend.best_token(
            gray,
            whitelist=whitelist,
            psms=preset.psms,
            oem=preset.oem,
            min_len=preset.min_len,
            max_len=preset.max_len,
        )
        dt = time.perf_counter() - t_img0
        got = str(r.token or "")
        results.append(
            {
                "id": str(e.get("id", img_path.stem)),
                "image": rel_or_abs(img_path),
                "expected": expected,
                "got": got,
                "passed": bool(got == expected),
                "time_s": float(dt),
                "params": {"invert": bool(r.invert), "scale": int(r.scale), "psm": int(r.psm)},
            }
        )

    total = len(results)
    passed = sum(1 for r in results if r.get("passed"))
    payload = {
        "schema": {"version": 0, "description": "OCR run over labeled manifest (v0)."},
        "inputs": {"manifest": rel_or_abs(manifest_path)},
        "backend": str(getattr(backend, "name", "unknown")),
        "backend_detail": {"providers": getattr(backend, "providers", None)},
        "counts": {"passed": int(passed), "total": int(total)},
        "time_s": float(time.perf_counter() - t0),
        "results": results,
    }

    if args.out is None:
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        out = args.out
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
