#!/usr/bin/env python3
from __future__ import annotations

import dataclasses
import functools
import json
import os
import re
from pathlib import Path
from typing import Iterable, Protocol

import numpy as np

try:
    import onnxruntime as ort  # type: ignore
except Exception:  # pragma: no cover
    ort = None  # type: ignore

import pytesseract

from ocr_preprocess_v0 import (
    OcrResult,
    crop_label_text_roi,
    extract_dense_component,
    normalize_token,
    ocr_best_token,
    preprocess_label_for_ocr,
 )

ROOT = Path(__file__).resolve().parents[1]


class Backend(Protocol):
    name: str

    def best_token(
        self,
        gray: np.ndarray,
        *,
        whitelist: str,
        psms: Iterable[int],
        oem: int,
        min_len: int,
        max_len: int,
    ) -> OcrResult: ...


def _ensure_tesseract_cmd() -> None:
    t = Path("/usr/bin/tesseract")
    if t.exists():
        pytesseract.pytesseract.tesseract_cmd = str(t)


_ensure_tesseract_cmd()


def _rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


@dataclasses.dataclass(frozen=True)
class TesseractBackend:
    name: str = "tesseract"

    def best_token(
        self,
        gray: np.ndarray,
        *,
        whitelist: str,
        psms: Iterable[int],
        oem: int,
        min_len: int,
        max_len: int,
    ) -> OcrResult:
        # Standardized “tiny label” policy for our edge-label crops:
        # - try both polarities (some crops are white-on-black masks, others are not)
        # - isolate the dense callout + its glyph ROI to reduce wiring noise
        # - upscale aggressively (Tesseract benefits from large glyphs)
        # - adaptive threshold tends to outperform Otsu on these solid masks
        best = OcrResult(token="", conf=-1.0, psm=0, invert=False, scale=1)
        dense = extract_dense_component(gray)
        roi = crop_label_text_roi(dense)

        def run_one(*, invert: bool, scale: int, threshold: str, morph: str | None) -> OcrResult | None:
            pre = preprocess_label_for_ocr(
                roi,
                scale=scale,
                invert=invert,
                use_clahe=True,
                threshold=threshold,
                border=10,
                morph=morph,
            )
            try:
                cand = ocr_best_token(
                    pre,
                    whitelist=whitelist,
                    psms=psms,
                    oem=oem,
                    min_len=min_len,
                    max_len=max_len,
                    timeout_s=2.0,
                )
            except Exception:
                return None
            return OcrResult(
                token=cand.token,
                conf=cand.conf,
                psm=cand.psm,
                invert=invert,
                scale=scale,
            )

        # Two-stage policy: keep the common fast path small, then escalate only if needed.
        # Typical label crops are white-on-black with strong contrast, so `invert=True` and
        # adaptive threshold + close tends to win quickly.
        for invert in (True, False):
            for scale in (5, 3):
                cand = run_one(invert=invert, scale=scale, threshold="adaptive", morph="close")
                if cand is None:
                    continue
                if (cand.conf, len(cand.token)) > (best.conf, len(best.token)):
                    best = cand
                # Early exit: high confidence and plausible length is good enough.
                if best.conf >= 85.0 and min_len <= len(best.token) <= max_len:
                    return best

        # Escalation: alternative thresholding/morphology for edge cases (e.g. tiny 1-glyph labels).
        for invert in (True, False):
            for scale in (5, 3):
                for threshold, morph in (("adaptive", None), ("otsu", "close"), ("otsu", None)):
                    cand = run_one(invert=invert, scale=scale, threshold=threshold, morph=morph)
                    if cand is None:
                        continue
                    if (cand.conf, len(cand.token)) > (best.conf, len(best.token)):
                        best = cand
                    if best.conf >= 90.0 and min_len <= len(best.token) <= max_len:
                        return best

        return best


def _ctc_greedy_decode(class_ids: np.ndarray, *, alphabet: str, blank_id: int) -> str:
    out: list[str] = []
    prev = blank_id
    for cid in class_ids.tolist():
        if cid == blank_id or cid == prev:
            prev = cid
            continue
        if 0 <= cid < len(alphabet):
            out.append(alphabet[cid])
        prev = cid
    return "".join(out)


def _softmax(x: np.ndarray, axis: int = -1) -> np.ndarray:
    x = x - np.max(x, axis=axis, keepdims=True)
    e = np.exp(x)
    return e / np.sum(e, axis=axis, keepdims=True)


@dataclasses.dataclass(frozen=True)
class OnnxCtcBackend:
    """
    Optional ONNX CTC recognizer.

    Contract (expected, but not enforced):
    - Input: single-channel image as float32 in [0,1], shape (1, 1, H, W) or (1, H, W, 1)
    - Output: logits over classes including a blank, shape (1, T, C) or (T, 1, C) or (T, C)

    This is intentionally lightweight: it enables GPU-backed experiments without committing large models.
    """

    model_path: Path
    providers: tuple[str, ...]
    alphabet: str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    blank_id: int = 36  # after A-Z0-9 (len=36)
    name: str = "onnx-ctc"

    def __post_init__(self) -> None:
        if ort is None:
            raise RuntimeError("onnxruntime not available")
        if not self.model_path.exists():
            raise FileNotFoundError(str(self.model_path))
        # Optional sidecar config: `<model>.json`
        # {
        #   "alphabet": "...",
        #   "blank_id": 36
        # }
        cfg_path = self.model_path.with_suffix(self.model_path.suffix + ".json")
        if cfg_path.exists():
            data = json.loads(cfg_path.read_text(encoding="utf-8"))
            if isinstance(data, dict):
                alphabet = data.get("alphabet")
                blank_id = data.get("blank_id")
                if isinstance(alphabet, str) and alphabet:
                    object.__setattr__(self, "alphabet", alphabet)
                if isinstance(blank_id, int) and blank_id >= 0:
                    object.__setattr__(self, "blank_id", blank_id)

    def _session(self) -> "ort.InferenceSession":
        assert ort is not None
        return _get_onnx_session(str(self.model_path), self.providers)

    def _prepare_input(self, gray: np.ndarray) -> tuple[str, np.ndarray]:
        sess = self._session()
        inp = sess.get_inputs()[0]
        name = str(inp.name)

        g = gray.astype(np.float32)
        # Normalize to [0,1]
        if g.max() > 1.5:
            g = g / 255.0

        # Default to NCHW.
        x = g[None, None, :, :]
        # If model expects NHWC, convert.
        shp = inp.shape
        if len(shp) == 4 and shp[-1] == 1:
            x = np.transpose(x, (0, 2, 3, 1))
        return name, x

    def _infer_logits(self, gray: np.ndarray) -> np.ndarray:
        sess = self._session()
        inp_name, x = self._prepare_input(gray)
        outs = sess.run(None, {inp_name: x})
        if not outs:
            raise RuntimeError("onnx produced no outputs")
        y = np.asarray(outs[0])
        # normalize common shapes to (T, C)
        if y.ndim == 3:
            # (1, T, C) or (T, 1, C)
            if y.shape[0] == 1:
                y = y[0]
            elif y.shape[1] == 1:
                y = y[:, 0, :]
        if y.ndim == 2:
            return y
        raise RuntimeError(f"unexpected logits shape: {y.shape}")

    def best_token(
        self,
        gray: np.ndarray,
        *,
        whitelist: str,
        psms: Iterable[int],
        oem: int,
        min_len: int,
        max_len: int,
    ) -> OcrResult:
        # The ONNX model is expected to be a word-level recognizer; PSM/OEM do not apply.
        # We keep the signature compatible with the tesseract backend.
        _ = (psms, oem)

        # Preprocess to a stable binarized, scaled crop (helps most recognizers).
        pre = preprocess_label_for_ocr(
            gray,
            scale=3,
            invert=True,
            use_clahe=True,
            threshold="adaptive",
            border=8,
            morph="close",
        )
        logits = self._infer_logits(pre)
        probs = _softmax(logits, axis=-1)
        class_ids = np.argmax(probs, axis=-1)
        raw = _ctc_greedy_decode(class_ids, alphabet=self.alphabet, blank_id=self.blank_id)

        tok = normalize_token(raw)
        tok = re.sub(rf"[^{re.escape(whitelist)}]", "", tok)
        if not (min_len <= len(tok) <= max_len):
            tok = ""

        # Heuristic confidence: mean max prob across time steps.
        conf = float(np.mean(np.max(probs, axis=-1)) * 100.0) if tok else -1.0
        return OcrResult(token=tok, conf=conf, psm=-1, invert=True, scale=3)


def resolve_backend(
    *,
    backend: str,
    onnx_model: Path | None,
    prefer_cuda: bool,
) -> Backend:
    """
    Proper fallback order for throughput:
    1) ONNX CTC model on TensorRT/CUDA (if model + provider available)
    2) ONNX CTC model on CPU/DNNL (if model available)
    3) Tesseract (CPU)
    """
    b = (backend or "auto").strip().lower()
    if b not in ("auto", "tesseract", "onnx"):
        raise ValueError(f"unknown backend: {backend}")

    def try_backend(providers: tuple[str, ...]) -> OnnxCtcBackend | None:
        if onnx_model is None:
            return None
        try:
            cand = OnnxCtcBackend(onnx_model, providers=providers)
            # Force session init now so we can fall back cleanly if CUDA EP is misconfigured.
            _ = cand._session()
            return cand
        except Exception:
            return None

    if b in ("auto", "onnx"):
        if onnx_model is None:
            env = os.environ.get("OCR_ONNX_MODEL", "").strip()
            if env:
                onnx_model = Path(env)
        if onnx_model is not None and ort is not None and onnx_model.exists():
            avail = tuple(ort.get_available_providers())
            # Prefer acceleration EPs when present, but always include CPU as a safety net
            # to avoid hard failures on misconfigured CUDA/TensorRT installs.
            if prefer_cuda:
                for providers in (
                    ("TensorrtExecutionProvider", "CUDAExecutionProvider", "CPUExecutionProvider"),
                    ("CUDAExecutionProvider", "CPUExecutionProvider"),
                ):
                    if all(p in avail for p in providers if p != "CPUExecutionProvider"):
                        cand = try_backend(providers)
                        if cand is not None:
                            return cand
            for providers in (("DnnlExecutionProvider",), ("CPUExecutionProvider",)):
                if providers[0] not in avail:
                    continue
                cand = try_backend(providers)
                if cand is not None:
                    return cand
        if b == "onnx":
            raise SystemExit(
                "requested --backend onnx but no usable ONNX model/providers found "
                f"(set OCR_ONNX_MODEL or pass --onnx-model; model={_rel_or_abs(onnx_model) if onnx_model else 'none'})"
            )

    return TesseractBackend()


@functools.lru_cache(maxsize=8)
def _get_onnx_session(model_path: str, providers: tuple[str, ...]) -> "ort.InferenceSession":
    assert ort is not None
    sess_opts = ort.SessionOptions()
    # Keep logs quiet for batch OCR runs.
    sess_opts.log_severity_level = 3
    return ort.InferenceSession(model_path, sess_options=sess_opts, providers=list(providers))
