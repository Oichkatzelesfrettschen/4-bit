#!/usr/bin/env python3
from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class OcrPreset:
    """
    OCR call parameters that are stable across backends.

    Backends may ignore some fields (e.g. ONNX backends can ignore PSM),
    but we keep the interface uniform so reports are comparable.
    """

    psms: tuple[int, ...]
    oem: int
    min_len: int
    max_len: int


def preset_layout_edge_label(*, expected: str | None = None) -> OcrPreset:
    """
    The layout edge labels are tiny (often 1–4 chars) and can be either
    single-glyph or short tokens like CLK1.
    """
    exp = (expected or "").strip()
    if len(exp) <= 1:
        # Single char: treat as “single char” first; allow fallbacks.
        return OcrPreset(psms=(10, 7, 13), oem=1, min_len=1, max_len=1)
    if len(exp) == 2:
        return OcrPreset(psms=(10, 7, 8), oem=1, min_len=1, max_len=2)
    # Longer tokens: treat as single line/word; avoid a full PSM sweep for speed.
    return OcrPreset(psms=(7, 11), oem=1, min_len=1, max_len=4)
