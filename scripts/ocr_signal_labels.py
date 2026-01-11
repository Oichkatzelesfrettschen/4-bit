#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import re
from dataclasses import dataclass
from difflib import SequenceMatcher
from pathlib import Path

import cv2
import numpy as np
import pytesseract
from PIL import Image
from PIL import ImageDraw


@dataclass(frozen=True)
class SignalPoint:
    x: int
    y: int
    name: str


@dataclass(frozen=True)
class ChipSpec:
    chip: str
    schematic_bmp: Path
    signals_txt: Path


ROOT = Path(__file__).resolve().parents[1]

TESSERACT_TIMEOUT_S: float | None = None


def specs() -> dict[str, ChipSpec]:
    def emu(p: str) -> Path:
        return ROOT / "docs" / "emulators" / p

    return {
        "4001": ChipSpec("4001", emu("i4001-schematic.bmp"), emu("i4001-signals.txt")),
        "4002": ChipSpec("4002", emu("i4002-schematic.bmp"), emu("i4002-signals.txt")),
        "4003": ChipSpec("4003", emu("i4003-schematic.bmp"), emu("i4003-signals.txt")),
        "4004": ChipSpec("4004", emu("i4004-schematic.bmp"), emu("i4004-signals.txt")),
    }


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def parse_signals_txt(path: Path) -> list[SignalPoint]:
    out: list[SignalPoint] = []
    for raw in path.read_text(errors="replace").splitlines():
        line = raw.strip().strip("\r")
        if not line or line.startswith(";"):
            continue
        parts = [p.strip() for p in line.split(",")]
        if len(parts) != 3:
            continue
        try:
            x = int(parts[0])
            y = int(parts[1])
        except ValueError:
            continue
        out.append(SignalPoint(x=x, y=y, name=parts[2]))
    return out


_KEEP = re.compile(r"[^A-Z0-9_~()+&/\\-]")


def normalize_name(s: str) -> str:
    # Collapse whitespace and strip obvious noise, then uppercase and keep a conservative charset.
    s = s.strip().replace("\n", " ").replace("\t", " ")
    s = re.sub(r"\s+", " ", s)
    s = s.upper()
    s = s.replace("—", "-").replace("–", "-")
    s = s.replace(" ", "")
    s = _KEEP.sub("", s)
    return s


def classify_expected(name: str) -> str:
    # Treat complex boolean expressions as non-labels; focus verification on simple labels.
    # Examples of expressions: "(~POC)CLK2(X12+X32)~INH", "((~SC)(JIN+FIN))CLK1(...)"
    if any(ch in name for ch in "()+=* "):
        return "expression"
    if len(name) > 24:
        return "expression"
    return "label"


def is_pin_like_alias(aliases: list[str]) -> bool:
    # Pin labels in the 4004 schematic are often "01", "02", etc.
    return any(a.isdigit() and 1 <= len(a) <= 3 for a in aliases)


def ocr_pin_number(crop: Image.Image) -> str:
    # Digits-only OCR tuned for tiny pin labels like "01", "02".
    arr = np.asarray(crop.convert("L"))
    arr = cv2.resize(arr, (arr.shape[1] * 10, arr.shape[0] * 10), interpolation=cv2.INTER_CUBIC)
    blur = cv2.GaussianBlur(arr, (3, 3), 0)
    th = cv2.adaptiveThreshold(blur, 255, cv2.ADAPTIVE_THRESH_GAUSSIAN_C, cv2.THRESH_BINARY, 35, 5)
    img = Image.fromarray(th, mode="L")

    best = ""
    for psm in (7, 8, 6, 13):
        s = ocr_single_line(img, psm=psm, whitelist="0123456789")
        s = "".join(ch for ch in normalize_name(s) if ch.isdigit())
        if len(s) > len(best):
            best = s
    return best


def classify_no_match_reason(
    *,
    tokens_total: int,
    tokens_confident: int,
    max_similarity: float | None,
    components_total: int | None = None,
) -> str:
    # Split "no_token" into actionable buckets:
    # - OCR returned nothing (likely too tiny / line removal ate it / wrong region)
    # - OCR returned tokens but all below confidence threshold (too tiny / noisy)
    # - OCR returned confident tokens, but none similar to expected (likely net-name not printed here)
    if tokens_total <= 0:
        if components_total is not None and components_total <= 0:
            return "no_text_components"
        return "ocr_no_tokens"
    if tokens_confident <= 0:
        return "ocr_low_conf"
    if max_similarity is not None and max_similarity <= 0.0:
        return "not_printed_near_point"
    return "no_similar_token"

def is_short_alias(aliases: list[str]) -> bool:
    # e.g. D0, A12, IO0, 01
    for a in aliases:
        if not a:
            continue
        if len(a) <= 4 and any(ch.isdigit() for ch in a):
            return True
    return False


def is_text_alias(aliases: list[str]) -> bool:
    return any(any(ch.isalpha() for ch in a) for a in aliases if a)


def ocr_alias_near_point(region: Image.Image, *, px: int, py: int, aliases: list[str]) -> str | None:
    """
    Best-effort OCR near a reference point to match a known alias string.

    Returns the matched alias (normalized) if found.
    """
    if not aliases:
        return None
    alias_set = set(normalize_name(a) for a in aliases if a)

    # Search around the point; many printed pin labels are slightly left/up of the node.
    dxs = [0, -90, -70, -50, -30, -15, 15, 30, 50, 70, 90]
    dys = [0, -90, -70, -50, -30, -15, 15, 30, 50, 70, 90]

    whitelist = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    for dy in dys:
        for dx in dxs:
            crop = crop_around(region, x=px + dx, y=py + dy, w=140, h=140)
            arr = np.asarray(crop.convert("L"))
            arr = cv2.resize(arr, (arr.shape[1] * 10, arr.shape[0] * 10), interpolation=cv2.INTER_CUBIC)
            blur = cv2.GaussianBlur(arr, (3, 3), 0)
            th = cv2.adaptiveThreshold(blur, 255, cv2.ADAPTIVE_THRESH_GAUSSIAN_C, cv2.THRESH_BINARY, 41, 7)
            img = Image.fromarray(th, mode="L")

            # Token-based OCR tends to segment tiny labels better than plain to_string.
            for psm in (7, 8, 6, 11, 13):
                tokens = ocr_tokens(img, psm=psm, whitelist=whitelist)
                for t in tokens:
                    s = str(t.get("norm", ""))
                    if not s:
                        continue
                    # direct match
                    if s in alias_set:
                        return s
                    # substring match (e.g. OCR returns "D0.." or "01..")
                    for a in alias_set:
                        if a and a in s:
                            return a
                    # digit salvage for D{0..3} labels where OCR drops the 'D'
                    for a in alias_set:
                        if a.startswith("D") and len(a) == 2 and a[1].isdigit():
                            if a[1] in s and "D" not in s:
                                return a
    return None


def ocr_alias_by_offset_region(
    *,
    img: Image.Image,
    x: int,
    y: int,
    region_w: int,
    region_h: int,
    scale: int,
    invert: bool,
    aliases: list[str],
) -> str | None:
    if not aliases:
        return None
    alias_set = set(normalize_name(a) for a in aliases if a)
    if not alias_set:
        return None

    # For longer textual aliases (e.g. TEST), use a bigger region with offsets and token OCR.
    # This is still bounded (<= offsets * psm variants token passes) and only runs for alias targets.
    dxs = [0, -120, -90, -60, -30, 30, 60, 90, 120]
    dys = [0, -120, -90, -60, -30, 30, 60, 90, 120]
    psm_variants = (11, 6, 7, 8, 13)
    whitelist = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"

    for dy in dys:
        for dx in dxs:
            region = crop_around(img, x=x + dx, y=y + dy, w=region_w, h=region_h)
            pre = preprocess(region, scale=scale, invert=invert)
            for psm in psm_variants:
                tokens = ocr_tokens(pre, psm=psm, whitelist=whitelist)
                for t in tokens:
                    s = str(t.get("norm", ""))
                    if not s:
                        continue
                    if s in alias_set:
                        return s
                    for a in alias_set:
                        if a and a in s:
                            return a
    return None


def looks_vertical_label(img: Image.Image) -> bool:
    # Heuristic: if height >> width, likely a vertical label.
    return img.height > img.width * 1.2


def crop_around(img: Image.Image, *, x: int, y: int, w: int, h: int) -> Image.Image:
    left = max(0, x - w // 2)
    top = max(0, y - h // 2)
    right = min(img.width, left + w)
    bottom = min(img.height, top + h)
    return img.crop((left, top, right, bottom))


def preprocess(crop: Image.Image, *, scale: int, invert: bool) -> Image.Image:
    gray = np.asarray(crop.convert("L"))
    if scale != 1:
        gray = cv2.resize(gray, (gray.shape[1] * scale, gray.shape[0] * scale), interpolation=cv2.INTER_CUBIC)

    th = cv2.adaptiveThreshold(
        gray,
        255,
        cv2.ADAPTIVE_THRESH_GAUSSIAN_C,
        cv2.THRESH_BINARY,
        35,
        5,
    )
    # Remove long horizontal/vertical wires to boost OCR on small label crops.
    inv = 255 - th  # features as white on black
    h_kernel = cv2.getStructuringElement(cv2.MORPH_RECT, (max(15, gray.shape[1] // 12), 1))
    v_kernel = cv2.getStructuringElement(cv2.MORPH_RECT, (1, max(15, gray.shape[0] // 6)))
    h_lines = cv2.morphologyEx(inv, cv2.MORPH_OPEN, h_kernel)
    v_lines = cv2.morphologyEx(inv, cv2.MORPH_OPEN, v_kernel)
    lines = cv2.bitwise_or(h_lines, v_lines)
    no_lines = cv2.subtract(inv, lines)
    out = 255 - no_lines

    if invert:
        out = 255 - out
    return Image.fromarray(out, mode="L")


def rotate_variants(img: Image.Image) -> list[tuple[int, Image.Image]]:
    return [(0, img), (90, img.rotate(90, expand=True)), (180, img.rotate(180, expand=True)), (270, img.rotate(270, expand=True))]


def candidate_offsets(region_w: int, region_h: int) -> list[tuple[int, int]]:
    dx = region_w // 3
    dy = region_h // 3
    return [(0, 0), (-dx, 0), (dx, 0), (0, -dy), (0, dy)]


def ocr_tokens(pre: Image.Image, *, psm: int, whitelist: str) -> list[dict[str, object]]:
    # Use TSV so we can select the best nearby recognized token.
    config = f'--psm {psm} -c tessedit_char_whitelist="{whitelist}"'
    try:
        tsv = pytesseract.image_to_data(
            pre,
            config=config,
            output_type=pytesseract.Output.STRING,
            timeout=TESSERACT_TIMEOUT_S,
        )
    except Exception:
        return []
    lines = [ln for ln in tsv.splitlines() if ln.strip()]
    if not lines:
        return []
    header = lines[0].split("\t")
    out: list[dict[str, object]] = []
    for raw in lines[1:]:
        parts = raw.split("\t")
        if len(parts) != len(header):
            continue
        row = dict(zip(header, parts))
        text = row.get("text", "").strip()
        if not text:
            continue
        try:
            conf = float(row.get("conf", "-1"))
            left = int(row.get("left", "0"))
            top = int(row.get("top", "0"))
            width = int(row.get("width", "0"))
            height = int(row.get("height", "0"))
        except ValueError:
            continue
        out.append(
            {
                "text": text,
                "norm": normalize_name(text),
                "conf": conf,
                "bbox": {"x": left, "y": top, "w": width, "h": height},
                "center": {"x": left + width / 2.0, "y": top + height / 2.0},
            }
        )
    return out


def find_text_components(region_gray: np.ndarray) -> list[dict[str, object]]:
    """
    Find candidate text components by removing long lines and taking connected components.

    Returns bboxes in region pixel coordinates.
    """
    _, bw = cv2.threshold(region_gray, 200, 255, cv2.THRESH_BINARY_INV)

    h_kernel = cv2.getStructuringElement(cv2.MORPH_RECT, (max(25, region_gray.shape[1] // 10), 1))
    v_kernel = cv2.getStructuringElement(cv2.MORPH_RECT, (1, max(25, region_gray.shape[0] // 6)))
    h_lines = cv2.morphologyEx(bw, cv2.MORPH_OPEN, h_kernel)
    v_lines = cv2.morphologyEx(bw, cv2.MORPH_OPEN, v_kernel)
    lines = cv2.bitwise_or(h_lines, v_lines)
    bw = cv2.subtract(bw, lines)

    # Merge adjacent glyph strokes into word-ish clusters.
    bw = cv2.dilate(bw, cv2.getStructuringElement(cv2.MORPH_RECT, (3, 3)), iterations=2)

    num, _labels, stats, centroids = cv2.connectedComponentsWithStats(bw, connectivity=8)
    comps: list[dict[str, object]] = []
    for i in range(1, num):
        x, y, w, h, area = stats[i].tolist()
        if area < 120:
            continue
        if w < 3 or h < 3:
            continue
        if w > region_gray.shape[1] * 0.6 or h > region_gray.shape[0] * 0.6:
            continue
        cx, cy = centroids[i].tolist()
        comps.append(
            {
                "bbox": {"x": int(x), "y": int(y), "w": int(w), "h": int(h)},
                "center": {"x": float(cx), "y": float(cy)},
                "area": int(area),
            }
        )
    return comps


def find_text_components_oriented(region_gray: np.ndarray, *, orientation: str) -> list[dict[str, object]]:
    """
    Like find_text_components, but merges glyphs into word-ish blobs using an oriented dilation.

    orientation:
      - "h": merges horizontally (good for normal left-to-right text)
      - "v": merges vertically (good for rotated/vertical labels)
    """
    _, bw = cv2.threshold(region_gray, 200, 255, cv2.THRESH_BINARY_INV)

    # Remove long wires/lines first.
    h_kernel = cv2.getStructuringElement(cv2.MORPH_RECT, (max(25, region_gray.shape[1] // 10), 1))
    v_kernel = cv2.getStructuringElement(cv2.MORPH_RECT, (1, max(25, region_gray.shape[0] // 6)))
    h_lines = cv2.morphologyEx(bw, cv2.MORPH_OPEN, h_kernel)
    v_lines = cv2.morphologyEx(bw, cv2.MORPH_OPEN, v_kernel)
    bw = cv2.subtract(bw, cv2.bitwise_or(h_lines, v_lines))

    if orientation == "h":
        merge = cv2.getStructuringElement(cv2.MORPH_RECT, (9, 3))
    elif orientation == "v":
        merge = cv2.getStructuringElement(cv2.MORPH_RECT, (3, 9))
    else:
        raise ValueError(f"unknown orientation: {orientation}")

    bw = cv2.dilate(bw, merge, iterations=2)

    num, _labels, stats, centroids = cv2.connectedComponentsWithStats(bw, connectivity=8)
    comps: list[dict[str, object]] = []
    for i in range(1, num):
        x, y, w, h, area = stats[i].tolist()
        if area < 120:
            continue
        if w < 6 or h < 6:
            continue
        if w > region_gray.shape[1] * 0.8 or h > region_gray.shape[0] * 0.8:
            continue
        cx, cy = centroids[i].tolist()
        comps.append(
            {
                "bbox": {"x": int(x), "y": int(y), "w": int(w), "h": int(h)},
                "center": {"x": float(cx), "y": float(cy)},
                "area": int(area),
            }
        )
    return comps


def union_bbox(a: dict[str, int], b: dict[str, int]) -> dict[str, int]:
    x0 = min(int(a["x"]), int(b["x"]))
    y0 = min(int(a["y"]), int(b["y"]))
    x1 = max(int(a["x"]) + int(a["w"]), int(b["x"]) + int(b["w"]))
    y1 = max(int(a["y"]) + int(a["h"]), int(b["y"]) + int(b["h"]))
    return {"x": x0, "y": y0, "w": x1 - x0, "h": y1 - y0}


def build_union_candidates(
    comps: list[dict[str, object]],
    *,
    max_unions: int = 8,
    k_neighbors: int = 4,
) -> list[dict[str, object]]:
    # Create union boxes from nearby components to handle labels split into multiple glyph blobs.
    out: list[dict[str, object]] = []
    comps2 = list(comps)
    comps2.sort(key=lambda c: float(c.get("dist", 0.0)))
    for base in comps2[:max_unions]:
        base_bbox = base["bbox"]
        cx0 = float(base["center"]["x"])
        cy0 = float(base["center"]["y"])
        neighbors = sorted(
            comps2,
            key=lambda c: (float(c.get("dist", 0.0)), (float(c["center"]["x"]) - cx0) ** 2 + (float(c["center"]["y"]) - cy0) ** 2),
        )
        bbox = dict(base_bbox)
        for nb in neighbors[1 : 1 + k_neighbors]:
            bbox = union_bbox(bbox, nb["bbox"])
        out.append({"bbox": bbox, "dist": float(base.get("dist", 0.0)), "orientation": base.get("orientation", "h")})
    return out


def load_aliases(path: Path) -> dict[str, dict[str, list[str]]]:
    if not path.exists():
        return {}
    raw = json.loads(path.read_text(encoding="utf-8"))
    out: dict[str, dict[str, list[str]]] = {}
    if not isinstance(raw, dict):
        return out
    for chip, mapping in raw.items():
        if not isinstance(mapping, dict):
            continue
        out_chip: dict[str, list[str]] = {}
        for k, v in mapping.items():
            if not isinstance(k, str):
                continue
            if isinstance(v, list) and all(isinstance(x, str) for x in v):
                out_chip[normalize_name(k)] = [normalize_name(x) for x in v]
        out[str(chip)] = out_chip
    return out


def score_match(expected: str, observed: str) -> float:
    if not expected and not observed:
        return 1.0
    if not expected or not observed:
        return 0.0
    return SequenceMatcher(a=expected, b=observed).ratio()

def ocr_single_line(img: Image.Image, *, psm: int, whitelist: str) -> str:
    config = f'--psm {psm} -c tessedit_char_whitelist="{whitelist}"'
    try:
        return pytesseract.image_to_string(img, config=config, timeout=TESSERACT_TIMEOUT_S)
    except Exception:
        return ""


def safe_slug(name: str) -> str:
    name = normalize_name(name)
    if not name:
        return "EMPTY"
    name = name.replace("/", "_").replace("\\", "_")
    return re.sub(r"[^A-Z0-9_~()+&_-]+", "_", name)[:80]


def main() -> int:
    parser = argparse.ArgumentParser(description="Coordinate-based OCR of signal labels from i400x schematic bitmaps.")
    parser.add_argument("--chip", action="append", choices=sorted(specs().keys()), help="Chip to process (repeatable)")
    parser.add_argument("--all", action="store_true", help="Process all supported chips")
    parser.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "ocr_signal_labels")
    parser.add_argument(
        "--mode",
        choices=["crop", "region"],
        default="crop",
        help="OCR strategy: small crop around point, or large region token selection",
    )
    parser.add_argument("--region-w", type=int, default=640, help="Region width in source pixels")
    parser.add_argument("--region-h", type=int, default=240, help="Region height in source pixels")
    parser.add_argument("--crop-w", type=int, default=240, help="Crop width in source pixels (mode=crop)")
    parser.add_argument("--crop-h", type=int, default=90, help="Crop height in source pixels (mode=crop)")
    parser.add_argument("--scale", type=int, default=4, help="Upscale factor before OCR")
    parser.add_argument("--psm", type=int, default=7, help="Tesseract page segmentation mode")
    parser.add_argument("--invert", action="store_true", help="Invert colors after thresholding")
    parser.add_argument("--save-mismatches", type=int, default=60, help="Save up to N worst crops per chip")
    parser.add_argument("--min-score", type=float, default=0.85, help="Minimum ratio to consider a match OK")
    parser.add_argument("--min-conf", type=float, default=60.0, help="Minimum token confidence to consider")
    parser.add_argument("--use-distance", action="store_true", help="Prefer tokens near the reference point")
    parser.add_argument("--max-dist", type=float, default=220.0, help="Maximum pixel distance (in region coords) to consider")
    parser.add_argument("--labels-only", action="store_true", help="Only verify label-like names (skip expressions)")
    parser.add_argument("--limit", type=int, default=0, help="Only process the first N points (0 = all)")
    parser.add_argument("--name-regex", default="", help="Only process expected names matching this regex")
    parser.add_argument("--fallback", action="store_true", help="Enable expensive fallbacks (offsets/rotations/psm variants)")
    parser.add_argument(
        "--tesseract-timeout",
        type=float,
        default=2.0,
        help="Per-call timeout (seconds) for tesseract invocations; 0 disables the timeout",
    )
    parser.add_argument(
        "--aliases",
        type=Path,
        default=ROOT / "scripts" / "ocr_signal_aliases.json",
        help="Optional JSON aliases: {\"4004\": {\"CLK1\": [\"01\"]}}",
    )
    parser.add_argument("--candidates", type=int, default=6, help="How many nearby text components to try (mode=crop)")
    parser.add_argument("--pad", type=int, default=10, help="Padding around component bbox (source pixels, mode=crop)")
    parser.add_argument("--max-calls", type=int, default=120, help="Max OCR calls per point (mode=crop)")
    parser.add_argument(
        "--whitelist",
        default="ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_~()+&/-",
        help="Tesseract character whitelist",
    )
    args = parser.parse_args()

    global TESSERACT_TIMEOUT_S
    TESSERACT_TIMEOUT_S = None if float(args.tesseract_timeout) <= 0.0 else float(args.tesseract_timeout)

    selected = set(args.chip or [])
    if args.all:
        selected = set(specs().keys())
    if not selected:
        parser.error("select --all or at least one --chip")

    out_dir = args.out_dir.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    def maybe_rel(path: Path) -> str:
        p = path.resolve()
        try:
            return str(p.relative_to(ROOT))
        except ValueError:
            return str(p)
    aliases = load_aliases(args.aliases)

    manifest: dict[str, object] = {
        "tool": "scripts/ocr_signal_labels.py",
        "tesseract_version": str(pytesseract.get_tesseract_version()),
        "params": {
            "mode": args.mode,
            "region_w": args.region_w,
            "region_h": args.region_h,
            "crop_w": args.crop_w,
            "crop_h": args.crop_h,
            "scale": args.scale,
            "psm": args.psm,
            "invert": bool(args.invert),
            "min_score": args.min_score,
            "min_conf": args.min_conf,
            "use_distance": bool(args.use_distance),
            "max_dist": args.max_dist,
            "labels_only": bool(args.labels_only),
            "limit": args.limit,
            "name_regex": args.name_regex,
            "fallback": bool(args.fallback),
            "aliases": str(args.aliases),
            "candidates": args.candidates,
            "pad": args.pad,
            "whitelist": args.whitelist,
        },
        "outputs": [],
    }

    for chip in sorted(selected):
        spec = specs()[chip]
        img = Image.open(spec.schematic_bmp)
        points = sorted(parse_signals_txt(spec.signals_txt), key=lambda p: (p.y, p.x, p.name))
        if args.name_regex:
            rx = re.compile(args.name_regex)
            points = [p for p in points if rx.search(p.name)]
        if args.limit and args.limit > 0:
            points = points[: args.limit]

        chip_dir = out_dir / chip.lower()
        crops_dir = chip_dir / "crops"
        chip_dir.mkdir(parents=True, exist_ok=True)
        crops_dir.mkdir(parents=True, exist_ok=True)

        rows: list[dict[str, object]] = []
        for idx, p in enumerate(points):
            base_left = max(0, p.x - args.region_w // 2)
            base_top = max(0, p.y - args.region_h // 2)
            base_right = min(img.width, base_left + args.region_w)
            base_bottom = min(img.height, base_top + args.region_h)
            base_region_bbox = {"x": int(base_left), "y": int(base_top), "w": int(base_right - base_left), "h": int(base_bottom - base_top)}

            expected_kind = classify_expected(p.name)
            if args.labels_only and expected_kind != "label":
                rows.append(
                    {
                        "idx": idx,
                        "x": p.x,
                        "y": p.y,
                        "region_bbox": base_region_bbox,
                        "expected": p.name,
                        "expected_kind": expected_kind,
                        "expected_norm": normalize_name(p.name),
                        "ocr_norm": "",
                        "ocr_raw": "",
                        "score": 0.0,
                        "ok": False,
                        "reason": "skipped_expression",
                    }
                )
                continue

            expected_norm = normalize_name(p.name)
            alias_ok = sorted(set(aliases.get(chip, {}).get(expected_norm, [])))

            # Select best token by (similarity, distance) among confident tokens.
            best = None
            best_score = 0.0
            best_dist = None
            best_conf = None
            best_bbox = None
            best_bbox_frame = None
            best_region_bbox = None
            best_region_offset = None
            best_region_rotation_deg = None
            best_region_psm = None

            # Diagnostics for interpreting mismatches ("OCR failed" vs "name not printed here").
            region_tokens_total = 0
            region_tokens_confident = 0
            region_max_similarity = None
            region_ocr_calls = 0
            region_components_total = 0

            # Fast alias resolution (pin labels, D0/D1, etc) without doing full OCR.
            if alias_ok and is_short_alias(alias_ok):
                region_for_alias = crop_around(img, x=p.x, y=p.y, w=args.region_w, h=args.region_h)
                # point location in region coords
                region_left = max(0, p.x - args.region_w // 2)
                region_top = max(0, p.y - args.region_h // 2)
                px0 = p.x - region_left
                py0 = p.y - region_top
                hit = ocr_alias_near_point(region_for_alias, px=px0, py=py0, aliases=alias_ok)
                if hit is not None:
                    best = {"text": hit, "norm": hit, "conf": 0.0, "bbox": None}
                    best_score = 1.0
                    best_bbox = None
                    best_bbox_frame = "alias"
                    best_dist = 0.0
                    best_region_bbox = base_region_bbox
                    best_region_offset = (0, 0)
                    best_region_rotation_deg = 0
                    best_region_psm = None

            # For longer textual aliases (e.g. TEST), do a bounded offset+token scan.
            if best_score < args.min_score and alias_ok and is_text_alias(alias_ok):
                hit = ocr_alias_by_offset_region(
                    img=img,
                    x=p.x,
                    y=p.y,
                    region_w=args.region_w,
                    region_h=args.region_h,
                    scale=args.scale,
                    invert=bool(args.invert),
                    aliases=alias_ok,
                )
                if hit is not None:
                    best = {"text": hit, "norm": hit, "conf": 0.0, "bbox": None}
                    best_score = 1.0
                    best_bbox = None
                    best_bbox_frame = "alias"
                    best_dist = 0.0
                    best_region_bbox = base_region_bbox
                    best_region_offset = (0, 0)
                    best_region_rotation_deg = 0
                    best_region_psm = None

            if args.mode == "crop":
                # Component-driven OCR: coordinates are often on wires/pins, not centered on text.
                if best_score >= args.min_score:
                    # alias already resolved
                    pass
                region = crop_around(img, x=p.x, y=p.y, w=args.region_w, h=args.region_h).convert("L")
                region_np = np.asarray(region)

                region_left = max(0, p.x - args.region_w // 2)
                region_top = max(0, p.y - args.region_h // 2)
                px = p.x - region_left
                py = p.y - region_top

                comps: list[dict[str, object]] = []
                for orient in ("h", "v"):
                    for c in find_text_components_oriented(region_np, orientation=orient):
                        cx = float(c["center"]["x"])
                        cy = float(c["center"]["y"])
                        c["dist"] = float(((cx - px) ** 2 + (cy - py) ** 2) ** 0.5)
                        c["orientation"] = orient
                        comps.append(c)

                region_components_total = len(comps)
                comps.sort(key=lambda c: (float(c.get("dist", 0.0)), -int(c.get("area", 0))))
                comps = comps[: max(1, int(args.candidates))]
                union_comps = build_union_candidates(comps)

                psm_variants = [int(args.psm)]
                if args.fallback:
                    psm_variants = [int(args.psm), 6, 7, 8, 11, 13]

                max_calls = int(args.max_calls)
                calls = 0

                for c in comps + union_comps:
                    if best_score >= args.min_score:
                        break
                    bbox = c["bbox"]
                    x0 = max(0, int(bbox["x"]) - int(args.pad))
                    y0 = max(0, int(bbox["y"]) - int(args.pad))
                    x1 = min(region_np.shape[1], int(bbox["x"]) + int(bbox["w"]) + int(args.pad))
                    y1 = min(region_np.shape[0], int(bbox["y"]) + int(bbox["h"]) + int(args.pad))
                    comp_crop = Image.fromarray(region_np[y0:y1, x0:x1], mode="L")
                    pre = preprocess(comp_crop, scale=args.scale, invert=bool(args.invert))

                    rotations = [0]
                    if c.get("orientation") == "v" or looks_vertical_label(pre) or pre.width > pre.height * 1.2:
                        rotations = [0, 90, 270]

                    for deg in rotations:
                        pre_rot = pre if deg == 0 else pre.rotate(deg, expand=True)
                        for psm in psm_variants:
                            calls += 1
                            if calls > max_calls:
                                break
                            raw = ocr_single_line(pre_rot, psm=psm, whitelist=args.whitelist)
                            norm = normalize_name(raw)
                            if not norm:
                                continue
                            sim = score_match(expected_norm, norm)
                            if alias_ok and norm in set(alias_ok):
                                sim = 1.0
                            if best is None or sim > best_score:
                                best = {"text": raw, "norm": norm, "conf": 0.0, "bbox": bbox}
                                best_score = sim
                                best_bbox = bbox
                                best_bbox_frame = "region"
                                best_dist = float(c.get("dist", 0.0)) if "dist" in c else None
                                best_region_bbox = base_region_bbox
                                best_region_offset = (0, 0)
                                best_region_rotation_deg = deg
                                best_region_psm = psm
                            if best_score >= args.min_score:
                                break
                        if calls > max_calls or best_score >= args.min_score:
                            break
                    if calls > max_calls:
                        break
            else:
                # Region mode: token selection (slower, more robust to coordinates on wires/pins).
                if best_score >= args.min_score:
                    # alias already resolved
                    pass
                # Pre-compute "is there any text-like blob in the base region?" to classify
                # "OCR empty" vs "likely no label printed near this coordinate".
                base_region_gray = crop_around(img, x=p.x, y=p.y, w=args.region_w, h=args.region_h).convert("L")
                base_np = np.asarray(base_region_gray)
                region_components_total = len(find_text_components_oriented(base_np, orientation="h")) + len(
                    find_text_components_oriented(base_np, orientation="v")
                )
                # Region mode often contains multiple nearby labels; always try a small PSM set.
                psm_variants = [int(args.psm), 6]
                offset_variants = [(0, 0)]
                rotation_degs = [0]
                if args.fallback:
                    psm_variants = [int(args.psm), 7, 11]
                    offset_variants = candidate_offsets(args.region_w, args.region_h)
                    rotation_degs = [0, 90, 180, 270]
                else:
                    # Cheap vertical-label assist: if the expected token contains letters, also try 90/270.
                    if any(ch.isalpha() for ch in expected_norm):
                        rotation_degs = [0, 90, 270]

                for dx, dy in offset_variants:
                    if best_score >= args.min_score and not args.fallback:
                        break

                    cx0 = p.x + dx
                    cy0 = p.y + dy
                    region = crop_around(img, x=cx0, y=cy0, w=args.region_w, h=args.region_h)
                    pre0 = preprocess(region, scale=args.scale, invert=bool(args.invert))

                    region_left = max(0, cx0 - args.region_w // 2)
                    region_top = max(0, cy0 - args.region_h // 2)
                    px = p.x - region_left
                    py = p.y - region_top

                    for deg in rotation_degs:
                        pre = pre0 if deg == 0 else pre0.rotate(deg, expand=True)
                        for psm in psm_variants:
                            region_ocr_calls += 1
                            tokens = ocr_tokens(pre, psm=psm, whitelist=args.whitelist)
                            region_tokens_total += len(tokens)
                            for t in tokens:
                                conf = float(t["conf"])
                                if conf >= args.min_conf:
                                    region_tokens_confident += 1

                                sim_any = score_match(expected_norm, str(t["norm"]))
                                if region_max_similarity is None or sim_any > float(region_max_similarity):
                                    region_max_similarity = sim_any

                                if conf < args.min_conf:
                                    continue
                                sim = sim_any
                                if sim <= 0.0:
                                    continue

                                dist = None
                                if args.use_distance and deg == 0:
                                    cx = float(t["center"]["x"]) / float(args.scale)
                                    cy = float(t["center"]["y"]) / float(args.scale)
                                    d = float(((cx - px) ** 2 + (cy - py) ** 2) ** 0.5)
                                    if d > args.max_dist:
                                        continue
                                    dist = d

                                if best is None or (sim > best_score) or (
                                    sim == best_score and dist is not None and (best_dist is None or dist < best_dist)
                                ):
                                    best = t
                                    best_score = sim
                                    best_dist = dist
                                    best_conf = conf
                                    best_bbox = t.get("bbox")
                                    best_bbox_frame = "pre_scaled"
                                    left = max(0, cx0 - args.region_w // 2)
                                    top = max(0, cy0 - args.region_h // 2)
                                    right = min(img.width, left + args.region_w)
                                    bottom = min(img.height, top + args.region_h)
                                    best_region_bbox = {"x": int(left), "y": int(top), "w": int(right - left), "h": int(bottom - top)}
                                    best_region_offset = (int(dx), int(dy))
                                    best_region_rotation_deg = int(deg)
                                    best_region_psm = int(psm)

            if best is None:
                rows.append(
                    {
                        "idx": idx,
                        "x": p.x,
                        "y": p.y,
                        "region_bbox": base_region_bbox,
                        "expected": p.name,
                        "expected_kind": expected_kind,
                        "expected_norm": expected_norm,
                        "ocr_raw": "",
                        "ocr_norm": "",
                        "score": 0.0,
                        "ok": False,
                        "reason": classify_no_match_reason(
                            tokens_total=region_tokens_total,
                            tokens_confident=region_tokens_confident,
                            max_similarity=region_max_similarity,
                            components_total=region_components_total,
                        ),
                        "best_bbox_frame": best_bbox_frame,
                        "best_region_bbox": best_region_bbox,
                        "best_region_offset": list(best_region_offset) if best_region_offset is not None else None,
                        "best_region_rotation_deg": best_region_rotation_deg,
                        "best_region_psm": best_region_psm,
                        "region_components_total": region_components_total,
                        "region_tokens_total": region_tokens_total,
                        "region_tokens_confident": region_tokens_confident,
                        "region_max_similarity": region_max_similarity,
                        "region_ocr_calls": region_ocr_calls,
                    }
                )
                continue

            ok = expected_kind == "label" and (best_score >= args.min_score or (alias_ok and str(best["norm"]) in set(alias_ok)))
            rows.append(
                {
                    "idx": idx,
                    "x": p.x,
                    "y": p.y,
                    "region_bbox": base_region_bbox,
                    "expected": p.name,
                    "expected_kind": expected_kind,
                    "expected_norm": expected_norm,
                    "aliases_ok": alias_ok,
                    "ocr_raw": str(best["text"]),
                    "ocr_norm": str(best["norm"]),
                    "score": best_score,
                    "ok": ok,
                    "reason": "matched" if ok else ("not_printed_near_point" if region_tokens_confident > 0 and best_score < 0.35 else "mismatch"),
                    "best_conf": float(best_conf if best_conf is not None else best["conf"]),
                    "best_dist_px": float(best_dist or 0.0) if best_dist is not None else None,
                    "best_bbox": best_bbox if best_bbox is not None else best.get("bbox"),
                    "best_bbox_frame": best_bbox_frame,
                    "best_region_bbox": best_region_bbox,
                    "best_region_offset": list(best_region_offset) if best_region_offset is not None else None,
                    "best_region_rotation_deg": best_region_rotation_deg,
                    "best_region_psm": best_region_psm,
                    "region_tokens_total": region_tokens_total,
                    "region_tokens_confident": region_tokens_confident,
                    "region_max_similarity": region_max_similarity,
                    "region_ocr_calls": region_ocr_calls,
                }
            )

        # Save worst mismatches for inspection (deterministic ordering).
        worst = [r for r in rows if r.get("reason") in ("no_text_components", "ocr_no_tokens", "ocr_low_conf", "mismatch")]
        worst.sort(key=lambda r: (float(r.get("score", 0.0)), str(r.get("expected_norm", "")), int(r.get("idx", 0))))
        for r in worst[: max(0, int(args.save_mismatches))]:
            p = points[int(r["idx"])]
            region = crop_around(img, x=p.x, y=p.y, w=args.region_w, h=args.region_h)
            pre = preprocess(region, scale=args.scale, invert=bool(args.invert)).convert("RGB")
            draw = ImageDraw.Draw(pre)
            # Mark the reference point (scaled coords).
            region_left = max(0, p.x - args.region_w // 2)
            region_top = max(0, p.y - args.region_h // 2)
            px = int((p.x - region_left) * args.scale)
            py = int((p.y - region_top) * args.scale)
            draw.line((px - 10, py, px + 10, py), fill=(255, 0, 0), width=2)
            draw.line((px, py - 10, px, py + 10), fill=(255, 0, 0), width=2)
            # If we have a bbox, draw it in green.
            bbox = r.get("best_bbox")
            if isinstance(bbox, dict) and {"x", "y", "w", "h"} <= set(bbox.keys()):
                x = int(bbox["x"]) * args.scale
                y = int(bbox["y"]) * args.scale
                w = int(bbox["w"]) * args.scale
                h = int(bbox["h"]) * args.scale
                draw.rectangle((x, y, x + w, y + h), outline=(0, 255, 0), width=2)

            out = crops_dir / f"{int(r['idx']):04d}_{safe_slug(p.name)}.png"
            pre.save(out)

        report = {
            "chip": chip,
            "inputs": {
                "schematic_bmp": str(spec.schematic_bmp.relative_to(ROOT)),
                "signals_txt": str(spec.signals_txt.relative_to(ROOT)),
                "schematic_sha256": sha256(spec.schematic_bmp),
                "signals_sha256": sha256(spec.signals_txt),
            },
            "params": {
                "region_w": args.region_w,
                "region_h": args.region_h,
                "scale": args.scale,
                "psm": args.psm,
                "invert": bool(args.invert),
                "min_score": args.min_score,
                "min_conf": args.min_conf,
                "max_dist": args.max_dist,
                "labels_only": bool(args.labels_only),
                "whitelist": args.whitelist,
            },
            "counts": {
                "total": len(rows),
                "ok": sum(1 for r in rows if r.get("ok")),
                "mismatch": sum(1 for r in rows if r.get("reason") not in ("matched", "skipped_expression")),
                "skipped": sum(1 for r in rows if r.get("reason") == "skipped_expression"),
                "labels_total": sum(1 for r in rows if r.get("expected_kind") == "label"),
            },
            "rows": rows,
        }

        out_json = chip_dir / f"{chip.lower()}_signal_ocr_report.json"
        out_tsv = chip_dir / f"{chip.lower()}_signal_ocr_report.tsv"
        out_json.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

        with out_tsv.open("w", encoding="utf-8", newline="") as f:
            w = csv.writer(f, delimiter="\t", lineterminator="\n")
            w.writerow(
                [
                    "idx",
                    "x",
                    "y",
                    "expected",
                    "expected_kind",
                    "expected_norm",
                    "ocr_norm",
                    "score",
                    "ok",
                    "reason",
                    "best_conf",
                    "best_dist_px",
                    "region_tokens_total",
                    "region_tokens_confident",
                    "region_max_similarity",
                    "region_ocr_calls",
                ]
            )
            for r in rows:
                w.writerow(
                    [
                        r["idx"],
                        r["x"],
                        r["y"],
                        r["expected"],
                        r.get("expected_kind", ""),
                        r["expected_norm"],
                        r["ocr_norm"],
                        f"{r['score']:.3f}",
                        "1" if r["ok"] else "0",
                        r.get("reason", ""),
                        f"{float(r.get('best_conf', 0.0)):.1f}" if "best_conf" in r else "",
                        f"{float(r.get('best_dist_px', 0.0)):.1f}" if r.get("best_dist_px") is not None else "",
                        str(int(r.get("region_tokens_total", 0))),
                        str(int(r.get("region_tokens_confident", 0))),
                        f"{float(r.get('region_max_similarity', 0.0)):.3f}" if r.get("region_max_similarity") is not None else "",
                        str(int(r.get("region_ocr_calls", 0))),
                    ]
                )

        manifest["outputs"].append(
            {
                "chip": chip,
                "report_json": maybe_rel(out_json),
                "report_tsv": maybe_rel(out_tsv),
                "crops_dir": maybe_rel(crops_dir),
                "counts": report["counts"],
            }
        )

    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
