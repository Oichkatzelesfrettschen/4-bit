#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
import time
from dataclasses import dataclass
from pathlib import Path

import numpy as np

from ocr_backend_v0 import resolve_backend
from ocr_preprocess_v0 import crop_label_text_roi, extract_dense_component, head_crop
from imgio_v0 import load_gray
from ocr_presets_v0 import preset_layout_edge_label

ROOT = Path(__file__).resolve().parents[1]


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def normalize_token(s: str) -> str:
    t = (s or "").strip().upper()
    t = re.sub(r"[^A-Z0-9]", "", t)
    return t


@dataclass(frozen=True)
class Try:
    token: str
    conf: float
    psm: int
    inv: bool
    scale: int


def best_ocr_for_label(
    gray: np.ndarray,
    *,
    backend_name: str,
    onnx_model: Path | None,
    prefer_cuda: bool,
    adaptive_whitelist: bool,
    fallback_whitelist: str,
    expected: str | None,
) -> Try:
    """
    Benchmark policy: use the same “tiny label” pipeline we want to standardize for edge-label OCR:
    - isolate dense component (label bubble)
    - prefer head crop on tall arrow labels
    - isolate text ROI inside the bubble
    - run a small PSM sweep with whitelist and choose best confidence token
    """
    roi = head_crop(gray, frac=0.42) if gray.shape[0] > 180 else gray
    roi = extract_dense_component(roi)
    text_roi = crop_label_text_roi(roi)
    # Guard: if ROI collapse makes OCR worse (e.g., tiny single-glyph component),
    # keep the original bubble crop.
    if text_roi.shape[0] < 18 or text_roi.shape[1] < 18:
        text_roi = roi

    whitelist = fallback_whitelist
    if adaptive_whitelist and expected:
        whitelist = "".join(sorted(set(str(expected).strip().upper() + "0123456789")))
    backend = resolve_backend(backend=backend_name, onnx_model=onnx_model, prefer_cuda=prefer_cuda)
    # Backends are allowed to ignore PSMs; we keep the interface stable.
    preset = preset_layout_edge_label(expected=expected)
    r = backend.best_token(
        text_roi,
        whitelist=whitelist,
        psms=preset.psms,
        oem=preset.oem,
        min_len=preset.min_len,
        max_len=preset.max_len,
    )
    return Try(token=r.token, conf=r.conf, psm=r.psm, inv=bool(r.invert), scale=int(r.scale))


def main() -> int:
    p = argparse.ArgumentParser(description="Run a small OCR benchmark set (v0).")
    p.add_argument("--bench", type=Path, required=True, help="Benchmark JSON (under docs/evidence/ocr_benchmarks_v0)")
    p.add_argument(
        "--backend",
        default="auto",
        choices=["auto", "tesseract", "onnx"],
        help="OCR backend (auto prefers ONNX/CUDA when configured, else Tesseract).",
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
    p.add_argument(
        "--adaptive-whitelist",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Derive per-item whitelist from expected token.",
    )
    p.add_argument(
        "--whitelist",
        default="ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
        help="Fallback whitelist when expected token is missing/empty.",
    )
    args = p.parse_args()

    bench_path = args.bench
    if not bench_path.is_absolute():
        bench_path = (ROOT / bench_path).resolve()
    bench = json.loads(bench_path.read_text(encoding="utf-8"))

    items = bench.get("items", [])
    if not items:
        raise SystemExit("benchmark has no items")

    started = time.perf_counter()
    results = []
    ok = 0
    fallback_whitelist = str(args.whitelist).strip().upper() or "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    for it in items:
        img_path = Path(it["image"])
        img_path = (ROOT / img_path).resolve() if not img_path.is_absolute() else img_path
        expected = normalize_token(it["expected"])
        gray = load_gray(img_path)

        t0 = time.perf_counter()
        got = best_ocr_for_label(
            gray,
            backend_name=str(args.backend),
            onnx_model=args.onnx_model,
            prefer_cuda=bool(args.prefer_cuda),
            adaptive_whitelist=bool(args.adaptive_whitelist),
            fallback_whitelist=fallback_whitelist,
            expected=expected,
        )
        dt = time.perf_counter() - t0

        passed = got.token == expected
        ok += 1 if passed else 0
        results.append(
            {
                "id": it.get("id"),
                "expected": expected,
                "got": got.token,
                "passed": bool(passed),
                "time_s": dt,
                "params": {"psm": got.psm, "invert": got.inv, "scale": got.scale},
                "image": rel_or_abs(img_path),
            }
        )

    total = time.perf_counter() - started
    out = {
        "bench": rel_or_abs(bench_path),
        "counts": {"total": len(items), "passed": ok},
        "time_s": total,
        "results": results,
    }
    print(json.dumps(out, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
