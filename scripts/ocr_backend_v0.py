#!/usr/bin/env python3
from __future__ import annotations

import dataclasses
import functools
import json
import math
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Iterable, Protocol

import numpy as np
import cv2  # type: ignore

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
    preprocess_label_preset_v0,
 )

ROOT = Path(__file__).resolve().parents[1]


def _norm_square_binary(img: np.ndarray, *, out_size: int) -> np.ndarray:
    """
    Normalize a binary (0/255) glyph image to a square canvas before resizing.

    This improves template matching stability for multi-character tokens by reducing
    sensitivity to small crop offsets and aspect ratio differences.
    """
    ys, xs = np.where(img > 0)
    if ys.size == 0 or xs.size == 0:
        return cv2.resize(img, (out_size, out_size), interpolation=cv2.INTER_NEAREST)
    x0, x1 = int(xs.min()), int(xs.max())
    y0, y1 = int(ys.min()), int(ys.max())
    margin = 2
    x0 = max(0, x0 - margin)
    y0 = max(0, y0 - margin)
    x1 = min(int(img.shape[1]) - 1, x1 + margin)
    y1 = min(int(img.shape[0]) - 1, y1 + margin)
    cropped = img[y0 : y1 + 1, x0 : x1 + 1]
    h, w = cropped.shape[:2]
    side = max(h, w)
    pad_y = side - h
    pad_x = side - w
    top = pad_y // 2
    bottom = pad_y - top
    left = pad_x // 2
    right = pad_x - left
    sq = cv2.copyMakeBorder(cropped, top, bottom, left, right, cv2.BORDER_CONSTANT, value=0)
    return cv2.resize(sq, (out_size, out_size), interpolation=cv2.INTER_NEAREST)


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


def _whitelist_kind(whitelist: str) -> str:
    wl = "".join(ch for ch in (whitelist or "") if ch.strip())
    if not wl:
        return "mixed"
    s = set(wl)
    digits = set("0123456789")
    letters = set("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
    if s.issubset(digits):
        return "digits"
    if s.issubset(letters):
        return "letters"
    if s.issubset(letters | digits):
        return "alnum"
    return "mixed"


def _select_preset_v0(*, whitelist: str, max_len: int) -> str:
    """
    Heuristic preprocessing preset selection.

    The caller still controls `invert` and `scale`; this only chooses a filter family.
    """
    forced = os.environ.get("OCR_PRESET", "").strip()
    if forced:
        return forced
    kind = _whitelist_kind(whitelist)
    if kind == "digits":
        return "digits_tiny" if int(max_len) <= 2 else "edge_label_light"
    if int(max_len) == 1:
        return "glyph_single"
    return "edge_label"


def _ensure_tesseract_cmd() -> None:
    t = Path("/usr/bin/tesseract")
    if t.exists():
        pytesseract.pytesseract.tesseract_cmd = str(t)


_ensure_tesseract_cmd()


def _rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def _tesseract_cli_best_token_one_psm(
    gray_bin: np.ndarray,
    *,
    whitelist: str,
    psm: int,
    oem: int,
    min_len: int,
    max_len: int,
    timeout_s: float,
) -> OcrResult:
    """
    Run the system `tesseract` binary and parse TSV output to obtain a single best token.

    This avoids `pytesseract` overhead for large batch runs while keeping the same engine and config knobs.
    """

    tcmd = Path(pytesseract.pytesseract.tesseract_cmd or "tesseract")
    if not tcmd.exists():
        return OcrResult(token="", conf=-1.0, psm=0, invert=False, scale=1)

    # Write an in-memory PNG to a temp file (tesseract CLI works with file paths).
    # Use PNG to preserve crisp 1-bit masks; JPEG harms tiny glyphs.
    ok, buf = cv2.imencode(".png", gray_bin)
    if not ok:
        return OcrResult(token="", conf=-1.0, psm=0, invert=False, scale=1)

    with tempfile.NamedTemporaryFile(suffix=".png") as tmp:
        tmp.write(buf.tobytes())
        tmp.flush()
        cmd = [
            str(tcmd),
            tmp.name,
            "stdout",
            "--psm",
            str(int(psm)),
            "--oem",
            str(int(oem)),
            "-l",
            "eng",
            "-c",
            f"tessedit_char_whitelist={whitelist}",
            "-c",
            "user_defined_dpi=300",
            "tsv",
        ]
        try:
            proc = subprocess.run(
                cmd,
                check=False,
                capture_output=True,
                text=True,
                timeout=float(timeout_s),
            )
        except Exception:
            return OcrResult(token="", conf=-1.0, psm=int(psm), invert=False, scale=1)
        if proc.returncode != 0 or not proc.stdout:
            return OcrResult(token="", conf=-1.0, psm=int(psm), invert=False, scale=1)

        # Parse TSV: header + rows; conf is numeric, -1 for non-words.
        texts: list[str] = []
        confs: list[float] = []
        for line in proc.stdout.splitlines()[1:]:
            parts = line.split("\t")
            if len(parts) < 12:
                continue
            conf_raw, text_raw = parts[10], parts[11]
            tok = normalize_token(text_raw or "")
            if not (min_len <= len(tok) <= max_len):
                continue
            try:
                conf = float(conf_raw)
            except Exception:
                conf = -1.0
            if conf < 0:
                continue
            texts.append(tok)
            confs.append(conf)
        if not texts:
            return OcrResult(token="", conf=-1.0, psm=int(psm), invert=False, scale=1)
        out = normalize_token("".join(texts))
        if not (min_len <= len(out) <= max_len):
            return OcrResult(token="", conf=-1.0, psm=int(psm), invert=False, scale=1)
        conf = float(sum(confs) / max(1, len(confs)))
        return OcrResult(token=out, conf=conf, psm=int(psm), invert=False, scale=1)


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
        preset = _select_preset_v0(whitelist=whitelist, max_len=max_len)
        preset = _select_preset_v0(whitelist=whitelist, max_len=max_len)

        def run_one(*, invert: bool, scale: int, threshold: str, morph: str | None) -> OcrResult | None:
            # Use preset defaults for deterministic behavior, but allow local overrides when exploring.
            if threshold == "adaptive" and morph == "close":
                pre = preprocess_label_preset_v0(roi, preset=preset, scale=scale, invert=invert)
            elif threshold == "adaptive" and morph is None:
                pre = preprocess_label_preset_v0(roi, preset="edge_label_light", scale=scale, invert=invert)
            else:
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
            scales = (7, 5, 3) if invert else (5, 3)
            for scale in scales:
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
                for threshold, morph in (
                    ("adaptive", None),
                    ("otsu", "close"),
                    ("otsu", None),
                    # Edge cases: very thin strokes can benefit from dilation/close.
                    ("adaptive", "dilate"),
                    ("otsu", "dilate"),
                ):
                    cand = run_one(invert=invert, scale=scale, threshold=threshold, morph=morph)
                    if cand is None:
                        continue
                    if (cand.conf, len(cand.token)) > (best.conf, len(best.token)):
                        best = cand
                    if best.conf >= 90.0 and min_len <= len(best.token) <= max_len:
                        return best

        return best


@dataclasses.dataclass(frozen=True)
class TesseractCliBackend:
    name: str = "tesseract-cli"

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
        best = OcrResult(token="", conf=-1.0, psm=0, invert=False, scale=1)
        dense = extract_dense_component(gray)
        roi = crop_label_text_roi(dense)

        def run_one(*, invert: bool, scale: int, threshold: str, morph: str | None) -> OcrResult | None:
            if threshold == "adaptive" and morph == "close":
                pre = preprocess_label_preset_v0(roi, preset="edge_label", scale=scale, invert=invert)
            elif threshold == "adaptive" and morph is None:
                pre = preprocess_label_preset_v0(roi, preset="edge_label_light", scale=scale, invert=invert)
            else:
                pre = preprocess_label_for_ocr(
                    roi,
                    scale=scale,
                    invert=invert,
                    use_clahe=True,
                    threshold=threshold,
                    border=10,
                    morph=morph,
                )
            best_local = OcrResult(token="", conf=-1.0, psm=0, invert=invert, scale=scale)
            for psm in psms:
                cand = _tesseract_cli_best_token_one_psm(
                    pre,
                    whitelist=whitelist,
                    psm=int(psm),
                    oem=oem,
                    min_len=min_len,
                    max_len=max_len,
                    timeout_s=2.0,
                )
                if (cand.conf, len(cand.token)) > (best_local.conf, len(best_local.token)):
                    best_local = OcrResult(token=cand.token, conf=cand.conf, psm=cand.psm, invert=invert, scale=scale)
            return best_local

        for invert in (True, False):
            scales = (7, 5, 3) if invert else (5, 3)
            for scale in scales:
                cand = run_one(invert=invert, scale=scale, threshold="adaptive", morph="close")
                if cand is None:
                    continue
                if (cand.conf, len(cand.token)) > (best.conf, len(best.token)):
                    best = cand
                if best.conf >= 85.0 and min_len <= len(best.token) <= max_len:
                    return best

        for invert in (True, False):
            for scale in (7, 5, 3):
                for threshold in ("adaptive", "otsu"):
                    for morph in ("close", "open", "dilate", "erode", None):
                        cand = run_one(invert=invert, scale=scale, threshold=threshold, morph=morph)
                        if cand is None:
                            continue
                        if (cand.conf, len(cand.token)) > (best.conf, len(best.token)):
                            best = cand
                        if best.conf >= 92.0 and min_len <= len(best.token) <= max_len:
                            return best

        return best


@dataclasses.dataclass(frozen=True)
class TesseractCliFastBackend:
    """
    Throughput-oriented Tesseract CLI backend.

    This runs a very small variant set to keep batch OCR and micro-benchmarks fast.
    Use the full `tesseract` stack when accuracy is more important than speed.
    """

    name: str = "tesseract-cli-fast"
    psm: int = 8
    scale: int = 5
    invert: bool = True

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
        _ = psms  # fixed PSM for speed
        dense = extract_dense_component(gray)
        roi = crop_label_text_roi(dense)
        preset = _select_preset_v0(whitelist=whitelist, max_len=max_len)
        if _whitelist_kind(whitelist) == "digits" and int(max_len) <= 2:
            psm = 10
            scale = max(6, int(self.scale))
        else:
            psm = int(self.psm)
            scale = int(self.scale)
        pre = preprocess_label_preset_v0(roi, preset=preset, scale=int(scale), invert=bool(self.invert))
        cand = _tesseract_cli_best_token_one_psm(
            pre,
            whitelist=whitelist,
            psm=int(psm),
            oem=oem,
            min_len=min_len,
            max_len=max_len,
            timeout_s=2.0,
        )
        return OcrResult(
            token=cand.token,
            conf=cand.conf,
            psm=cand.psm,
            invert=bool(self.invert),
            scale=int(scale),
        )


@dataclasses.dataclass(frozen=True)
class HuMomentsBackend:
    """
    Lightweight OpenCV/SIMD-ish fallback for *single-glyph* cases.

    This is not intended to replace Tesseract/ONNX. It exists to provide a deterministic
    last-resort guess when OCR produces no tokens at all.
    """

    name: str = "opencv-hu"

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
        _ = (psms, oem)
        if max_len != 1:
            return OcrResult(token="", conf=-1.0, psm=-1, invert=False, scale=1)

        # Restrict to single glyph candidates.
        candidates = [c for c in whitelist if c.strip()]
        if not candidates:
            return OcrResult(token="", conf=-1.0, psm=-1, invert=False, scale=1)

        dense = extract_dense_component(gray)
        roi = crop_label_text_roi(dense)
        pre = preprocess_label_for_ocr(
            roi,
            scale=6,
            invert=True,
            use_clahe=True,
            threshold="adaptive",
            border=16,
            morph="close",
        )

        def hu(x: np.ndarray) -> np.ndarray:
            m = cv2.moments(x)
            h = cv2.HuMoments(m).flatten()
            # log scale for stability
            out = np.zeros_like(h)
            for i, v in enumerate(h):
                out[i] = -np.sign(v) * np.log10(abs(v) + 1e-30)
            return out

        target = hu(pre)

        best_tok = ""
        best_dist = 1e9
        for ch in candidates:
            canvas = np.full((64, 64), 255, dtype=np.uint8)
            cv2.putText(
                canvas,
                ch,
                (10, 52),
                cv2.FONT_HERSHEY_SIMPLEX,
                1.8,
                0,
                4,
                lineType=cv2.LINE_AA,
            )
            proto = cv2.threshold(canvas, 200, 255, cv2.THRESH_BINARY_INV)[1]
            dist = float(np.linalg.norm(target - hu(proto)))
            if dist < best_dist:
                best_dist = dist
                best_tok = ch

        # Convert distance to a pseudo-confidence.
        # Smaller is better; this scale is empirical and meant for fallback triage only.
        if not best_tok:
            return OcrResult(token="", conf=-1.0, psm=-1, invert=True, scale=6)
        conf = max(0.0, 100.0 - 20.0 * best_dist)
        tok = normalize_token(best_tok)
        if not (min_len <= len(tok) <= max_len):
            tok = ""
        return OcrResult(token=tok, conf=float(conf) if tok else -1.0, psm=-1, invert=True, scale=6)


@dataclasses.dataclass(frozen=True)
class TemplateDirBackend:
    """
    Template-matching fallback for single-glyph crops using real, repo-provided glyph templates.

    Expected template directory contents:
    - PNG files named like `<TOKEN>.png` (e.g. `R.png`, `0.png`, `D.png`), OR
    - files containing `tok_<TOKEN>` in the filename.

    The goal is to outperform synthetic-font fallbacks (Hu moments) on the PMOS-era glyph style.
    """

    template_dir: Path
    name: str = "template-dir"
    _templates: dict[str, tuple[np.ndarray, ...]] = dataclasses.field(default_factory=dict, repr=False)

    @staticmethod
    def _preset_for_token(tok: str) -> str:
        t = (tok or "").strip().upper()
        if t.isdigit():
            return "digits_tiny"
        if len(t) == 1:
            return "glyph_single"
        return "edge_label"

    @staticmethod
    def _preset_for_request(*, allowed: set[str], max_len: int) -> str:
        if allowed and all(ch.isdigit() for ch in allowed) and int(max_len) <= 2:
            return "digits_tiny"
        if int(max_len) == 1:
            return "glyph_single"
        return "edge_label"

    def __post_init__(self) -> None:
        tdir = self.template_dir
        if not tdir.exists() or not tdir.is_dir():
            raise FileNotFoundError(str(tdir))
        templates: dict[str, list[np.ndarray]] = {}

        for p in sorted(tdir.glob("*.png")):
            tok = p.stem.strip().upper()
            m = re.search(r"tok_([A-Z0-9]+)", p.name, flags=re.IGNORECASE)
            if m:
                tok = m.group(1).upper()
            if not tok:
                continue
            img = cv2.imread(str(p), cv2.IMREAD_GRAYSCALE)
            if img is None:
                continue
            preset = self._preset_for_token(tok)
            pre = preprocess_label_preset_v0(img, preset=preset, scale=4, invert=True)
            pre = _norm_square_binary(pre, out_size=72)
            templates.setdefault(tok, []).append(pre)

        if not templates:
            raise RuntimeError(f"no usable templates in {tdir}")
        packed: dict[str, tuple[np.ndarray, ...]] = {k: tuple(v) for k, v in templates.items() if v}
        if not packed:
            raise RuntimeError(f"no usable templates in {tdir}")
        object.__setattr__(self, "_templates", packed)

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
        _ = (psms, oem)
        allowed = {c for c in (whitelist or "") if c.strip()}
        if not allowed or int(max_len) <= 0:
            return OcrResult(token="", conf=-1.0, psm=-1, invert=False, scale=1)

        # Edge/pad label crops are expected to already be tight; additional
        # connected-component pruning can sometimes select wiring instead of text.
        roi = crop_label_text_roi(gray)
        preset = self._preset_for_request(allowed=allowed, max_len=int(max_len))
        pre = preprocess_label_preset_v0(roi, preset=preset, scale=4, invert=True)
        pre = _norm_square_binary(pre, out_size=72)

        best_tok = ""
        best_score = -1.0
        for tok, tmpls in self._templates.items():
            if not (int(min_len) <= len(tok) <= int(max_len)):
                continue
            if any(ch not in allowed for ch in tok):
                continue
            for tmpl in tmpls:
                res = cv2.matchTemplate(pre, tmpl, cv2.TM_CCOEFF_NORMED)
                score = float(res[0, 0]) if res.size else -1.0
                if math.isnan(score):
                    continue
                if score > best_score:
                    best_score = score
                    best_tok = tok

        best_tok = normalize_token(best_tok)
        if not (min_len <= len(best_tok) <= max_len):
            return OcrResult(token="", conf=-1.0, psm=-1, invert=True, scale=4)

        conf = max(0.0, min(100.0, (best_score + 1.0) * 50.0))
        return OcrResult(token=best_tok, conf=float(conf), psm=-1, invert=True, scale=4)


@dataclasses.dataclass(frozen=True)
class CompositeBackend:
    name: str
    backends: tuple[Backend, ...]
    early_conf: float = 88.0

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
        # Optional voting mode: run all backends, then pick the token that is most agreed upon.
        # This helps stabilize results when a fast backend returns a high-confidence but wrong
        # token on very small/ambiguous crops.
        vote = os.environ.get("OCR_VOTE", "").strip().lower() in ("1", "true", "yes", "on")

        best = OcrResult(token="", conf=-1.0, psm=0, invert=False, scale=1)
        cands: list[OcrResult] = []
        for b in self.backends:
            cand = b.best_token(
                gray,
                whitelist=whitelist,
                psms=psms,
                oem=oem,
                min_len=min_len,
                max_len=max_len,
            )
            if (cand.conf, len(cand.token)) > (best.conf, len(best.token)):
                best = cand
            if cand.token and min_len <= len(cand.token) <= max_len and cand.conf >= 0:
                cands.append(cand)
            if (not vote) and best.conf >= self.early_conf and min_len <= len(best.token) <= max_len:
                return best

        if not vote or not cands:
            return best

        # Vote by exact normalized token; tie-break by (count, best_conf, token_len).
        counts: dict[str, int] = {}
        best_by_tok: dict[str, OcrResult] = {}
        for c in cands:
            tok = normalize_token(c.token)
            if not tok:
                continue
            counts[tok] = counts.get(tok, 0) + 1
            prev = best_by_tok.get(tok)
            if prev is None or (c.conf, len(c.token)) > (prev.conf, len(prev.token)):
                best_by_tok[tok] = c

        if not counts:
            return best
        # Pick winner
        winner = max(
            counts.keys(),
            key=lambda t: (counts[t], best_by_tok[t].conf, len(t)),
        )
        return best_by_tok[winner]


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
    if b not in ("auto", "tesseract", "tesseract_cli", "tesseract_cli_fast", "onnx", "template"):
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

    onnx_backend: Backend | None = None
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
                            onnx_backend = cand
                            break
                if onnx_backend is not None:
                    if b == "onnx":
                        return onnx_backend
            for providers in (("DnnlExecutionProvider",), ("CPUExecutionProvider",)):
                if providers[0] not in avail:
                    continue
                cand = try_backend(providers)
                if cand is not None:
                    onnx_backend = cand
                    break
            if onnx_backend is not None and b == "onnx":
                return onnx_backend
        if b == "onnx":
            raise SystemExit(
                "requested --backend onnx but no usable ONNX model/providers found "
                f"(set OCR_ONNX_MODEL or pass --onnx-model; model={_rel_or_abs(onnx_model) if onnx_model else 'none'})"
            )

    if b == "template":
        tdir = os.environ.get("OCR_TEMPLATE_DIR", "").strip()
        if tdir:
            try:
                return TemplateDirBackend(Path(tdir))
            except Exception:
                pass
        return HuMomentsBackend()

    # Throughput-first chain:
    # - fast CLI preset (good enough for most label crops)
    # - real template-dir matching for the repo’s glyph style (if configured)
    # - full CLI sweep (when installed)
    # - pytesseract sweep (as an internal fallback)
    # - Hu moments (single-glyph last resort)
    tess_chain: list[Backend] = [TesseractCliFastBackend()]

    # Prefer real templates early: they are cheap and often more accurate than Tesseract
    # on the PMOS-era glyph style for short tokens (e.g. R0/R1/D0/01).
    tdir = os.environ.get("OCR_TEMPLATE_DIR", "").strip()
    if tdir:
        try:
            tess_chain.append(TemplateDirBackend(Path(tdir)))
        except Exception:
            pass
    try:
        tcmd = Path(pytesseract.pytesseract.tesseract_cmd or "tesseract")
        if tcmd.exists():
            tess_chain.append(TesseractCliBackend())
    except Exception:
        pass
    tess_chain.append(TesseractBackend())
    tess_chain.append(HuMomentsBackend())

    if b == "tesseract_cli_fast":
        return CompositeBackend(
            name="tesseract-cli-fast-stack",
            backends=(TesseractCliFastBackend(), HuMomentsBackend()),
        )

    if b == "tesseract_cli":
        # Useful for benchmarks and for “fast path” scripts that want to avoid the
        # heavier multi-variant sweep in the full tesseract stack.
        return CompositeBackend(name="tesseract-cli-stack", backends=(TesseractCliBackend(), HuMomentsBackend()))

    if b == "tesseract":
        return CompositeBackend(name="tesseract-stack", backends=tuple(tess_chain))

    # auto: ONNX (if usable) → tesseract stack → Hu moments fallback.
    if onnx_backend is not None:
        return CompositeBackend(name="auto", backends=(onnx_backend, *tuple(tess_chain)))
    return CompositeBackend(name="auto", backends=tuple(tess_chain))


@functools.lru_cache(maxsize=8)
def _get_onnx_session(model_path: str, providers: tuple[str, ...]) -> "ort.InferenceSession":
    assert ort is not None
    sess_opts = ort.SessionOptions()
    # Keep logs quiet for batch OCR runs.
    sess_opts.log_severity_level = 3
    return ort.InferenceSession(model_path, sess_options=sess_opts, providers=list(providers))
