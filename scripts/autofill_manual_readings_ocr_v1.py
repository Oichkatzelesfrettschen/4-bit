#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable

import cv2  # type: ignore[import-not-found]
import pytesseract  # type: ignore[import-not-found]

ROOT = Path(__file__).resolve().parents[1]

WHITELIST = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789~()+&/._-[]"
TESSERACT_TIMEOUT_S = 4.0


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _normalize_token(t: str) -> str:
    return re.sub(r"\\s+", "", t.strip()).upper()


@dataclass(frozen=True)
class OcrAttempt:
    text: str
    conf: float
    psm: int
    invert: bool
    scale: int

    @property
    def score(self) -> float:
        # Reward confidence heavily; reward longer tokens slightly.
        return float(self.conf) + 2.0 * float(len(self.text))


def _prep(img_bgr: Any, *, invert: bool, scale: int) -> Any:
    # If the crop has debug overlays (red boxes, blue labels), remove strongly-colored pixels.
    # This keeps real black/white die markings while stripping UI text.
    if getattr(img_bgr, "ndim", 0) == 3:
        b, g, r = cv2.split(img_bgr)
        mx = cv2.max(cv2.max(b, g), r)
        mn = cv2.min(cv2.min(b, g), r)
        sat = (mx.astype("int16") - mn.astype("int16")) > 40
        if sat.any():
            img_bgr = img_bgr.copy()
            img_bgr[sat] = 255

    gray = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2GRAY)
    if scale != 1:
        gray = cv2.resize(gray, None, fx=float(scale), fy=float(scale), interpolation=cv2.INTER_CUBIC)
    if invert:
        gray = cv2.bitwise_not(gray)
    # Add a border so Tesseract doesn't treat edge-clipped glyphs as noise.
    gray = cv2.copyMakeBorder(gray, 24, 24, 24, 24, cv2.BORDER_CONSTANT, value=255)
    # Light denoise then binarize.
    blur = cv2.GaussianBlur(gray, (3, 3), 0)
    _, th = cv2.threshold(blur, 0, 255, cv2.THRESH_BINARY + cv2.THRESH_OTSU)
    return th


def _tesseract_best(img: Any, *, psm: int, whitelist: str, timeout_s: float) -> tuple[str, float]:
    cfg = [
        "--oem",
        "1",
        "--psm",
        str(int(psm)),
        "-c",
        f"tessedit_char_whitelist={whitelist}",
        "-c",
        "load_system_dawg=0",
        "-c",
        "load_freq_dawg=0",
    ]
    data = pytesseract.image_to_data(
        img,
        config=" ".join(cfg),
        output_type=pytesseract.Output.DICT,
        timeout=float(timeout_s),
    )
    texts: list[str] = []
    confs: list[float] = []
    for t, c in zip(data.get("text", []) or [], data.get("conf", []) or []):
        if t is None:
            continue
        s = str(t).strip()
        if not s:
            continue
        try:
            cf = float(c)
        except Exception:
            cf = -1.0
        if cf < 0:
            continue
        texts.append(s)
        confs.append(cf)
    if not texts:
        return "", -1.0
    out = _normalize_token("".join(texts))
    if not out:
        return "", -1.0
    conf = float(sum(confs) / max(1, len(confs)))
    return out, conf


def _ocr_crop(path: Path) -> OcrAttempt:
    img = cv2.imread(str(path), cv2.IMREAD_COLOR)
    if img is None:
        return OcrAttempt(text="", conf=-1.0, psm=0, invert=False, scale=1)

    best = OcrAttempt(text="", conf=-1.0, psm=0, invert=False, scale=1)
    # First pass: general whitelist.
    for invert in (False, True):
        for scale in (2, 3, 4):
            prep = _prep(img, invert=invert, scale=scale)
            for psm in (8, 7, 6, 10, 11, 13):
                text, conf = _tesseract_best(prep, psm=psm, whitelist=WHITELIST, timeout_s=TESSERACT_TIMEOUT_S)
                cand = OcrAttempt(text=text, conf=conf, psm=int(psm), invert=bool(invert), scale=int(scale))
                if cand.text and cand.score > best.score:
                    best = cand
    # Second pass: numeric-only (helps die-markings like 4002/4004 on the periphery).
    for invert in (False, True):
        for scale in (2, 3, 4):
            prep = _prep(img, invert=invert, scale=scale)
            for psm in (6, 7, 8, 10, 11, 13):
                text, conf = _tesseract_best(prep, psm=psm, whitelist="0123456789", timeout_s=TESSERACT_TIMEOUT_S)
                cand = OcrAttempt(text=text, conf=conf, psm=int(psm), invert=bool(invert), scale=int(scale))
                if cand.text and cand.score > best.score:
                    best = cand
    return best


def _prefer_clean_crop(path: Path) -> Path:
    """
    The repo has two crop classes:
    - `human_crops/box_XXX_node_YYY.png` (often includes an overlay label like 'box#...')
    - `crops/box_XXX_NA.png` (clean, best for OCR)

    Prefer the clean crop when possible.
    """
    s = str(path)
    if "/human_crops/" not in s:
        return path
    m = re.search(r"box_(\\d{3})_node_\\d+\\.png$", path.name)
    if not m:
        return path
    idx = m.group(1)
    base = path.parent.parent
    # Prefer raw crop (original cutout from metal) first.
    raw = base / "crops_raw" / f"box_{idx}.png"
    if raw.exists():
        return raw
    # Then prefer mask crop (hole-extracted letters), if non-empty.
    mask = base / "crops_mask" / f"box_{idx}_NA.png"
    if mask.exists():
        try:
            img = cv2.imread(str(mask), cv2.IMREAD_GRAYSCALE)
            if img is not None and int(img.min()) != int(img.max()):
                return mask
        except Exception:
            pass
    # Legacy detector crops (OCR-masked, but may be empty for some chips).
    legacy = base / "crops" / f"box_{idx}_NA.png"
    if legacy.exists():
        try:
            img = cv2.imread(str(legacy), cv2.IMREAD_GRAYSCALE)
            if img is not None and int(img.min()) != int(img.max()):
                return legacy
        except Exception:
            pass
    return path


def _dedupe_notes(note: str) -> str:
    bits = [b.strip() for b in re.split(r"[;\n]+", note) if b.strip()]
    seen: set[str] = set()
    out: list[str] = []
    for b in bits:
        if b in seen:
            continue
        seen.add(b)
        out.append(b)
    return "; ".join(out).strip()


def _note_looks_corrupted(note: str) -> bool:
    # A previous buggy splitter could break "conf"/"inv" into "co; f" / "i; v".
    t = note
    return ("co;" in t) or ("i; v" in t) or ("AUTO_OCR_V1 co;" in t)


def _looks_like_die_marking(token: str) -> bool:
    t = _normalize_token(token)
    if t in {"4001", "4002", "4003", "4004"}:
        return True
    # Common misreads of the die id due to edge-clipping or missing glyphs.
    if t in {"1001", "002", "004", "400"}:
        return True
    return False


def _looks_like_debug_overlay(token: str) -> bool:
    t = _normalize_token(token)
    if not t:
        return False
    # Typical overlay text: "box#17 node=672"
    if "BOX" in t and "NODE" in t:
        return True
    if t.startswith("BOX") and any(ch.isdigit() for ch in t):
        return True
    return False


def _iter_names_for_chip(anchors: dict[str, Any], chip: str) -> set[str]:
    aroot = anchors.get("anchors")
    if not isinstance(aroot, dict) or not isinstance(aroot.get(chip), dict):
        return set()
    return {str(k) for k in aroot[chip].keys()}


def _map_token_to_anchor(token: str, *, anchor_names: set[str]) -> str | None:
    t = _normalize_token(token)
    if not t:
        return None
    if _looks_like_die_marking(t):
        return None
    if t in anchor_names:
        return t
    # Common pad naming: D0..D3 in print, D0_PAD..D3_PAD in anchor list.
    m = re.match(r"^D([0-3])$", t)
    if m:
        cand = f"D{m.group(1)}_PAD"
        if cand in anchor_names:
            return cand
    # Some dies abbreviate CLOCK as CLK.
    if t == "CLK" and "CLOCK" in anchor_names:
        return "CLOCK"
    return None


def _parse_table(lines: list[str]) -> tuple[int, int]:
    start = -1
    end = -1
    for i, line in enumerate(lines):
        if line.strip().startswith("| idx |") and "ocr_best" in line:
            start = i + 2  # skip header + separator
            break
    if start < 0:
        return -1, -1
    for j in range(start, len(lines)):
        if not lines[j].strip().startswith("|"):
            end = j
            break
    if end < 0:
        end = len(lines)
    return start, end


def _split_row(line: str) -> list[str]:
    parts = [p.strip() for p in line.strip().strip("|").split("|")]
    return parts


def _format_row(cols: list[str]) -> str:
    return "| " + " | ".join(cols) + " |\n"


def main() -> int:
    ap = argparse.ArgumentParser(description="Autofill manual_readings_v0.md with improved OCR (tesseract, v1).")
    ap.add_argument("--chip", required=True, choices=["4001", "4002", "4003", "4004"])
    ap.add_argument(
        "--manual",
        type=Path,
        default=None,
        help="Manual readings md path (defaults to docs/evidence/layout_pad_labels_v0/<chip>/manual_readings_v0.md).",
    )
    ap.add_argument(
        "--anchors",
        type=Path,
        default=ROOT / "docs" / "evidence" / "schematic_layout_anchors_v0.json",
        help="Anchors v0 JSON to use for mapping OCR tokens to anchor_name.",
    )
    ap.add_argument("--min-conf", type=float, default=75.0, help="Minimum OCR confidence to auto-assign anchor_name.")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    chip = str(args.chip)
    manual = (
        (ROOT / args.manual).resolve()
        if args.manual is not None and not args.manual.is_absolute()
        else (args.manual if args.manual is not None else ROOT / "docs" / "evidence" / "layout_pad_labels_v0" / chip / "manual_readings_v0.md")
    )
    anchors_path = (ROOT / args.anchors).resolve() if not args.anchors.is_absolute() else args.anchors
    anchors = _load_json(anchors_path)
    anchor_names = _iter_names_for_chip(anchors, chip)

    lines = manual.read_text(encoding="utf-8").splitlines(keepends=True)
    start, end = _parse_table(lines)
    if start < 0:
        raise SystemExit(f"could not find candidates table in {manual}")

    updated = 0
    for i in range(start, end):
        if not lines[i].strip().startswith("|"):
            continue
        cols = _split_row(lines[i])
        if len(cols) != 7:
            continue
        idx_s, suggested_node, ocr_best, crop, printed_label, anchor_name, notes = cols
        try:
            idx = int(idx_s)
        except Exception:
            continue
        m = re.search(r"`([^`]+)`", crop)
        crop_path = Path(m.group(1)) if m else None
        if crop_path is None:
            continue
        crop_abs = crop_path if crop_path.is_absolute() else (ROOT / crop_path).resolve()
        crop_for_ocr = _prefer_clean_crop(crop_abs)
        att = _ocr_crop(crop_for_ocr)
        if not att.text:
            continue

        ocr_best_new = f"`{att.text}`"
        anchor_candidate = _map_token_to_anchor(att.text, anchor_names=anchor_names) if att.conf >= float(args.min_conf) else None
        changed = False
        if ocr_best.strip() != ocr_best_new:
            cols[2] = ocr_best_new
            changed = True

        if not printed_label.strip() or _looks_like_debug_overlay(printed_label):
            cols[4] = att.text
            changed = True

        if not anchor_name.strip() and anchor_candidate is not None:
            cols[5] = anchor_candidate
            changed = True

        note_str = notes.strip()
        if _note_looks_corrupted(note_str):
            note_str = ""
        note_str = _dedupe_notes(note_str)
        auto_note = f"AUTO_OCR_V1 conf={att.conf:.1f} psm={att.psm} inv={int(att.invert)} scale={att.scale}"
        if auto_note not in note_str:
            cols[6] = _dedupe_notes((note_str + ("; " if note_str else "") + auto_note).strip())
            changed = True

        if changed:
            lines[i] = _format_row(cols)
            updated += 1

    if not args.dry_run and updated:
        manual.write_text("".join(lines), encoding="utf-8")

    print(json.dumps({"manual": str(manual), "updated_rows": int(updated)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
