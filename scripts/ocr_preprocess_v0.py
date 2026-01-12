#!/usr/bin/env python3
from __future__ import annotations

import dataclasses
import re
from typing import Iterable

import cv2
import numpy as np
import pytesseract

from pathlib import Path


def _ensure_tesseract_cmd() -> None:
    # Some environments have `tesseract` installed but not discoverable by subprocess PATH.
    # Setting this explicitly avoids flaky `TesseractNotFoundError`.
    t = Path("/usr/bin/tesseract")
    if t.exists():
        pytesseract.pytesseract.tesseract_cmd = str(t)


_ensure_tesseract_cmd()


def normalize_token(t: str) -> str:
    t = (t or "").strip().upper()
    t = re.sub(r"[^A-Z0-9]", "", t)
    # Common tiny-glyph confusions
    if t == "O":
        return "0"
    if t == "I":
        return "1"
    return t


def token_is_plausible(t: str, *, min_len: int = 1, max_len: int = 4) -> bool:
    if not t:
        return False
    if not (min_len <= len(t) <= max_len):
        return False
    return bool(re.fullmatch(r"[A-Z0-9]+", t))


def add_border(gray: np.ndarray, px: int) -> np.ndarray:
    if px <= 0:
        return gray
    return cv2.copyMakeBorder(gray, px, px, px, px, cv2.BORDER_CONSTANT, value=255)


def clahe(gray: np.ndarray, *, clip_limit: float = 2.0, tile_grid_size: tuple[int, int] = (8, 8)) -> np.ndarray:
    c = cv2.createCLAHE(clipLimit=float(clip_limit), tileGridSize=tile_grid_size)
    return c.apply(gray)


def otsu(gray: np.ndarray) -> np.ndarray:
    return cv2.threshold(gray, 0, 255, cv2.THRESH_BINARY + cv2.THRESH_OTSU)[1]


def adaptive(gray: np.ndarray, *, block_size: int = 35, c: int = 5) -> np.ndarray:
    return cv2.adaptiveThreshold(
        gray,
        255,
        cv2.ADAPTIVE_THRESH_GAUSSIAN_C,
        cv2.THRESH_BINARY,
        int(block_size) | 1,
        int(c),
    )


def resize(gray: np.ndarray, *, scale: int) -> np.ndarray:
    if scale <= 1:
        return gray
    return cv2.resize(gray, (gray.shape[1] * scale, gray.shape[0] * scale), interpolation=cv2.INTER_NEAREST)


def resize_cuda(gray: np.ndarray, *, scale: int) -> np.ndarray:
    """
    Optional GPU-accelerated resize via OpenCV CUDA.
    Falls back to CPU if CUDA is unavailable.
    """
    if scale <= 1:
        return gray
    try:
        if not hasattr(cv2, "cuda"):
            return resize(gray, scale=scale)
        if cv2.cuda.getCudaEnabledDeviceCount() <= 0:
            return resize(gray, scale=scale)
        g = cv2.cuda_GpuMat()
        g.upload(gray)
        out = cv2.cuda.resize(
            g,
            (gray.shape[1] * scale, gray.shape[0] * scale),
            interpolation=cv2.INTER_NEAREST,
        )
        return out.download()
    except Exception:
        return resize(gray, scale=scale)


def extract_dense_component(gray: np.ndarray) -> np.ndarray:
    """
    Try to isolate a dense “label bubble” from a crop containing thin wiring.
    Returns a sub-ROI if it finds a plausible dense component, else returns the input.
    """
    if gray.size == 0:
        return gray
    _, inv = cv2.threshold(gray, 210, 255, cv2.THRESH_BINARY_INV)
    k = cv2.getStructuringElement(cv2.MORPH_RECT, (7, 7))
    opened = cv2.morphologyEx(inv, cv2.MORPH_OPEN, k, iterations=1)
    contours, _ = cv2.findContours(opened, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)

    best = None
    for cnt in contours:
        x, y, w, h = cv2.boundingRect(cnt)
        if w < 18 or h < 18:
            continue
        # Reject “wire-like” slivers; labels are not extremely thin.
        aspect = float(min(w, h)) / float(max(w, h))
        if aspect < 0.15:
            continue
        area = float(cv2.contourArea(cnt))
        fill = area / float(max(1, w * h))
        if fill < 0.18:
            continue
        # Prefer large dense components; fill is a gate, not the primary rank.
        score = area * fill
        if best is None or (score, area) > (best["score"], best["area"]):
            best = {"x": x, "y": y, "w": w, "h": h, "fill": fill, "area": area, "score": score}

    if best is None:
        return gray
    x0, y0 = int(best["x"]), int(best["y"])
    x1, y1 = x0 + int(best["w"]), y0 + int(best["h"])
    return gray[y0:y1, x0:x1]


def head_crop(gray: np.ndarray, *, frac: float = 0.42) -> np.ndarray:
    if gray.size == 0:
        return gray
    h, w = gray.shape[:2]
    hh = max(1, int(round(float(h) * float(frac))))
    return gray[0:hh, 0:w]


def crop_label_text_roi(gray: np.ndarray) -> np.ndarray:
    """
    Further reduce a dense “label bubble” to just its text region.

    Many layout edge labels are black callouts with white glyphs; OCR works best
    when we isolate the glyph region (excluding the callout arrow + border).
    """
    if gray.size == 0:
        return gray

    # Find the “bubble” body by treating dark regions as foreground.
    _, dark = cv2.threshold(gray, 210, 255, cv2.THRESH_BINARY_INV)
    k = cv2.getStructuringElement(cv2.MORPH_RECT, (5, 5))
    dark = cv2.morphologyEx(dark, cv2.MORPH_CLOSE, k, iterations=1)
    contours, _ = cv2.findContours(dark, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    if not contours:
        return gray
    cnt = max(contours, key=cv2.contourArea)
    x, y, w, h = cv2.boundingRect(cnt)
    if w < 12 or h < 12:
        return gray

    # Inset away from the border/tip to focus on the letter field.
    pad = max(1, int(round(min(w, h) * 0.06)))
    x0 = min(gray.shape[1], max(0, x + pad))
    y0 = min(gray.shape[0], max(0, y + pad))
    x1 = min(gray.shape[1], max(x0 + 1, x + w - pad))
    y1 = min(gray.shape[0], max(y0 + 1, y + h - pad))
    bubble = gray[y0:y1, x0:x1]

    # White glyphs are high-value pixels inside the (dark) bubble.
    letters = cv2.threshold(bubble, 200, 255, cv2.THRESH_BINARY)[1]
    letters = cv2.morphologyEx(letters, cv2.MORPH_OPEN, cv2.getStructuringElement(cv2.MORPH_RECT, (3, 3)), iterations=1)
    glyph_contours, _ = cv2.findContours(letters, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    if not glyph_contours:
        return gray

    xs: list[int] = []
    ys: list[int] = []
    # Heuristics: ignore large “arrow”/background shapes touching edges; keep inset glyphs.
    edge_margin = max(1, int(round(min(bubble.shape[0], bubble.shape[1]) * 0.05)))
    max_area = float(bubble.shape[0] * bubble.shape[1]) * 0.18
    for gc in glyph_contours:
        gx, gy, gw, gh = cv2.boundingRect(gc)
        area = float(cv2.contourArea(gc))
        if area < 10:
            continue
        if area > max_area:
            continue
        if gx <= edge_margin or gy <= edge_margin:
            continue
        if (gx + gw) >= (bubble.shape[1] - edge_margin) or (gy + gh) >= (bubble.shape[0] - edge_margin):
            continue
        xs.extend([gx, gx + gw])
        ys.extend([gy, gy + gh])
    if not xs or not ys:
        # Fallback: if we filtered everything out, use the original bubble (still often OK).
        return bubble

    gx0, gx1 = max(0, min(xs)), min(bubble.shape[1], max(xs))
    gy0, gy1 = max(0, min(ys)), min(bubble.shape[0], max(ys))
    # Small padding around glyphs for OCR segmentation.
    gpad = max(1, int(round(min(bubble.shape[0], bubble.shape[1]) * 0.04)))
    gx0 = max(0, gx0 - gpad)
    gy0 = max(0, gy0 - gpad)
    gx1 = min(bubble.shape[1], gx1 + gpad)
    gy1 = min(bubble.shape[0], gy1 + gpad)

    return bubble[gy0:gy1, gx0:gx1]


@dataclasses.dataclass(frozen=True)
class OcrResult:
    token: str
    conf: float
    psm: int
    invert: bool
    scale: int


def ocr_best_token(
    gray_bin: np.ndarray,
    *,
    whitelist: str,
    psms: Iterable[int],
    oem: int = 1,
    min_len: int = 1,
    max_len: int = 4,
    timeout_s: float = 2.0,
) -> OcrResult:
    """
    OCR a binarized image and return the single best token (by confidence).
    """
    best = OcrResult(token="", conf=-1.0, psm=0, invert=False, scale=1)
    for psm in psms:
        cfg = (
            f"--psm {int(psm)} --oem {int(oem)} -l eng "
            f"-c tessedit_char_whitelist={whitelist} "
            # Encourage consistent handling for tiny glyph crops.
            "-c user_defined_dpi=300"
        )
        data = pytesseract.image_to_data(
            gray_bin,
            config=cfg,
            output_type=pytesseract.Output.DICT,
            timeout=float(timeout_s),
        )
        for txt, conf in zip(data.get("text", []), data.get("conf", [])):
            tok = normalize_token(txt or "")
            if not token_is_plausible(tok, min_len=min_len, max_len=max_len):
                continue
            try:
                c = float(conf)
            except Exception:
                c = -1.0
            if c > best.conf:
                best = OcrResult(token=tok, conf=c, psm=int(psm), invert=False, scale=1)
    return best


def preprocess_label_for_ocr(
    gray: np.ndarray,
    *,
    scale: int,
    invert: bool,
    use_clahe: bool,
    threshold: str,
    border: int,
    use_cuda: bool = False,
    morph: str | None = None,
) -> np.ndarray:
    g = gray.copy()
    if invert:
        g = 255 - g
    g = cv2.normalize(g, None, 0, 255, cv2.NORM_MINMAX)
    if use_clahe:
        g = clahe(g)
    g = resize_cuda(g, scale=scale) if use_cuda else resize(g, scale=scale)
    if threshold == "otsu":
        g = otsu(g)
    elif threshold == "adaptive":
        g = adaptive(g)
    else:
        raise ValueError(f"unknown threshold: {threshold}")
    if morph:
        k = cv2.getStructuringElement(cv2.MORPH_RECT, (2, 2))
        if morph == "close":
            g = cv2.morphologyEx(g, cv2.MORPH_CLOSE, k, iterations=1)
        elif morph == "open":
            g = cv2.morphologyEx(g, cv2.MORPH_OPEN, k, iterations=1)
        else:
            raise ValueError(f"unknown morph: {morph}")
    g = add_border(g, border)
    return g
