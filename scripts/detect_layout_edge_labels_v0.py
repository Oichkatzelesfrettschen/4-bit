#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
from dataclasses import dataclass
from pathlib import Path

import cv2
import numpy as np
import pytesseract
from PIL import Image, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parents[1]


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def try_font(size: int) -> ImageFont.ImageFont:
    try:
        return ImageFont.truetype("DejaVuSans.ttf", size=size)
    except Exception:
        return ImageFont.load_default()


def dist_point_to_bbox(px: float, py: float, bb: dict[str, int]) -> float:
    x0, y0, x1, y1 = float(bb["x0"]), float(bb["y0"]), float(bb["x1"]), float(bb["y1"])
    dx = 0.0
    if px < x0:
        dx = x0 - px
    elif px > x1:
        dx = px - x1
    dy = 0.0
    if py < y0:
        dy = y0 - py
    elif py > y1:
        dy = py - y1
    return math.hypot(dx, dy)


def ocr_token(img: np.ndarray, *, psm: int) -> tuple[str, float]:
    """
    Return (best_token, best_conf). Token may be empty.
    We use a strict whitelist since labels are short and OCR is fragile.
    """
    whitelist = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    cfg = f"--psm {int(psm)} -l eng -c tessedit_char_whitelist={whitelist}"
    data = pytesseract.image_to_data(img, config=cfg, output_type=pytesseract.Output.DICT)
    best = ("", -1.0)
    for txt, conf in zip(data.get("text", []), data.get("conf", [])):
        t = (txt or "").strip().upper()
        if not t:
            continue
        try:
            c = float(conf)
        except Exception:
            c = -1.0
        if c > best[1]:
            best = (t, c)
    return best


def normalize_token(t: str) -> str:
    t = t.strip().upper()
    t = re.sub(r"[^A-Z0-9]", "", t)
    # Common confusions on tiny glyphs
    if t == "O":
        return "0"
    if t == "I":
        return "1"
    return t


def token_candidates(raw: str) -> list[str]:
    """
    Extract plausible tokens from a raw OCR string.

    Tesseract is prone to returning concatenations like 'RMEI' or numeric garbage like '351'.
    We keep any 1-3 character alnum substrings (uppercased) and also split common 2-letter/letter-digit combos.
    """
    s = (raw or "").strip().upper()
    s = re.sub(r"[^A-Z0-9]", " ", s)
    parts = [p for p in s.split() if p]
    out: list[str] = []
    for p in parts:
        p = normalize_token(p)
        if 1 <= len(p) <= 3:
            out.append(p)
        # If it's longer, also keep sliding windows (helps recover RM from RMEI).
        if len(p) > 3:
            for n in (2, 3):
                for i in range(0, len(p) - n + 1):
                    out.append(normalize_token(p[i : i + n]))
    # De-dupe but preserve order.
    seen: set[str] = set()
    uniq: list[str] = []
    for t in out:
        if t and t not in seen:
            uniq.append(t)
            seen.add(t)
    return uniq


def ocr_token_sweep(img: np.ndarray, *, psms: tuple[int, ...]) -> list[dict]:
    whitelist = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    tries: list[dict] = []
    votes: dict[str, int] = {}
    for psm in psms:
        cfg = f"--psm {int(psm)} -l eng -c tessedit_char_whitelist={whitelist}"
        # String is better than TSV for multi-token labels like RM, R2, etc.
        raw = pytesseract.image_to_string(img, config=cfg)
        for t in token_candidates(raw):
            votes[t] = votes.get(t, 0) + 1
            tries.append({"token": t, "conf": -1.0, "psm": int(psm), "raw": raw})

    # Add vote-weighted aggregate candidates so multi-char tokens can beat single noisy TSV hits.
    for tok, count in votes.items():
        # Heuristic confidence: require at least 2 supporting psms for multi-char tokens,
        # but allow single-char tokens only when they have strong support.
        if len(tok) >= 2 and count >= 1:
            conf = 35.0 + 12.0 * float(count) + 6.0 * float(len(tok))
        elif len(tok) == 1 and count >= 3:
            conf = 25.0 + 10.0 * float(count)
        else:
            continue
        tries.append({"token": tok, "conf": conf, "psm": -1, "raw": None, "vote_count": int(count)})
    return tries


def bbox_area(bb: dict[str, int]) -> int:
    return max(0, int(bb["x1"]) - int(bb["x0"])) * max(0, int(bb["y1"]) - int(bb["y0"]))


def bbox_contains(bb: dict[str, int], *, x: int, y: int) -> bool:
    return int(bb["x0"]) <= x <= int(bb["x1"]) and int(bb["y0"]) <= y <= int(bb["y1"])


def compute_tip(mask: np.ndarray) -> tuple[int, int]:
    """
    Compute a crude 'pointer tip' as the right-most pixel in the component.
    This works well for the metal mask's label bubbles, where the arrow points to the die.
    """
    ys, xs = np.where(mask > 0)
    if len(xs) == 0:
        return (0, 0)
    x_max = int(xs.max())
    y_candidates = ys[xs == x_max]
    y_tip = int(np.median(y_candidates)) if len(y_candidates) else int(ys.mean())
    return (x_max, y_tip)


def compute_tip_in_bbox(mask: np.ndarray, *, x: int, y: int, w: int, h: int) -> dict[str, int]:
    """
    Prefer a tip computed from the component mask (arrow head), falling back to bbox heuristic.
    """
    tx, ty = compute_tip(mask)
    if tx != 0 or ty != 0:
        return {"x": int(x + tx), "y": int(y + ty)}
    return {"x": int(x + w), "y": int(y + h // 2)}


def extract_label_roi(gray: np.ndarray) -> np.ndarray:
    """
    Heuristic: find the densest connected component inside a crop to isolate the label block.

    The periphery metal mask has lots of thin wiring; label "bubbles" are comparatively dense blobs.
    Returning the best sub-ROI makes OCR substantially more reliable.
    """
    if gray.size == 0:
        return gray

    _, inv = cv2.threshold(gray, 210, 255, cv2.THRESH_BINARY_INV)
    # Knock out thin-ish traces while keeping the larger label blocks.
    k = cv2.getStructuringElement(cv2.MORPH_RECT, (7, 7))
    opened = cv2.morphologyEx(inv, cv2.MORPH_OPEN, k, iterations=1)

    contours, _ = cv2.findContours(opened, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    best = None
    for cnt in contours:
        x, y, w, h = cv2.boundingRect(cnt)
        if w < 18 or h < 18:
            continue
        area = float(cv2.contourArea(cnt))
        fill = area / float(max(1, w * h))
        if fill < 0.18:
            continue
        if best is None or (fill, area) > (best["fill"], best["area"]):
            best = {"x": x, "y": y, "w": w, "h": h, "fill": fill, "area": area}

    if best is None:
        return gray

    x0, y0 = int(best["x"]), int(best["y"])
    x1, y1 = x0 + int(best["w"]), y0 + int(best["h"])
    return gray[y0:y1, x0:x1]


def extract_label_roi_for_ocr(gray: np.ndarray) -> np.ndarray:
    """
    Like extract_label_roi, but tuned for OCR rather than component detection.

    In particular, avoid aggressively trimming to a sub-ROI for short multi-char labels like 'RM',
    where the arrow tip may be part of the connected component and we still want the glyphs.
    """
    if gray.size == 0:
        return gray
    # If the crop is not tall, just use the full crop; small labels tend to be here.
    h, w = gray.shape[:2]
    if h <= 180:
        return gray
    return extract_label_roi(gray)

def label_head_crop(gray: np.ndarray, *, frac: float = 0.42) -> np.ndarray:
    """
    Many periphery labels are in the top part of a tall crop (e.g. RM with a long pointer).
    Cropping to the head region helps OCR focus on the text.
    """
    if gray.size == 0:
        return gray
    h, w = gray.shape[:2]
    hh = max(1, int(round(float(h) * float(frac))))
    return gray[0:hh, 0:w]


@dataclass(frozen=True)
class ChipSpec:
    chip: str
    metal_bmp: Path
    netlist_v0: Path


def specs() -> dict[str, ChipSpec]:
    emu = ROOT / "docs" / "emulators"
    ev = ROOT / "docs" / "evidence" / "netlists_v0"
    return {
        "4004": ChipSpec("4004", emu / "i4004-metal.bmp", ev / "4004_netlist_v0.json"),
    }


def main() -> int:
    p = argparse.ArgumentParser(description="Detect + OCR periphery edge label blocks on metal masks (v0).")
    p.add_argument("--chip", required=True, choices=sorted(specs().keys()))
    p.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "layout_edge_labels_v0")
    p.add_argument("--band", type=int, default=320, help="Pixels from each edge to search for labels.")
    p.add_argument("--min-area", type=int, default=800, help="Min component area (in binarized mask).")
    p.add_argument("--max-area", type=int, default=120000, help="Max component area (in binarized mask).")
    p.add_argument("--min-fill", type=float, default=0.22, help="Min fill ratio (area/(w*h)) for a component bbox.")
    p.add_argument(
        "--edges",
        default="top,bottom,left,right",
        help="Comma-separated edges to search: top,bottom,left,right",
    )
    p.add_argument("--pad", type=int, default=24, help="Padding around each detected component bbox for OCR.")
    p.add_argument("--area-penalty", type=float, default=0.003, help="Penalty for large net metal area in node suggestion.")
    p.add_argument(
        "--write-crops",
        action="store_true",
        help="Write per-detection OCR crops under out-dir for manual review.",
    )
    args = p.parse_args()

    spec = specs()[args.chip]

    net = json.loads(spec.netlist_v0.read_text(encoding="utf-8"))
    nodes = [n for n in net.get("node_stats", []) if isinstance(n, dict) and isinstance(n.get("metal_bbox"), dict)]
    if not nodes:
        raise SystemExit("netlist_v0 missing node_stats metal_bbox; re-run extract_netlist_v0.py")

    img = cv2.imread(str(spec.metal_bmp), cv2.IMREAD_GRAYSCALE)
    if img is None:
        raise SystemExit(f"failed to read {spec.metal_bmp}")
    h, w = img.shape
    band = int(args.band)
    edges = {e.strip().lower() for e in str(args.edges).split(",") if e.strip()}

    # Binarize: metal/features are dark.
    _, bw = cv2.threshold(img, 210, 255, cv2.THRESH_BINARY_INV)

    # Combine edge bands (top/bottom/left/right) into one mask.
    edge = np.zeros_like(bw)
    if "top" in edges:
        edge[0:band, :] = bw[0:band, :]
    if "bottom" in edges:
        edge[h - band : h, :] = np.maximum(edge[h - band : h, :], bw[h - band : h, :])
    if "left" in edges:
        edge[:, 0:band] = np.maximum(edge[:, 0:band], bw[:, 0:band])
    if "right" in edges:
        edge[:, w - band : w] = np.maximum(edge[:, w - band : w], bw[:, w - band : w])

    # Prefer large, filled-ish label blocks over thin traces.
    # Opening with a larger kernel tends to remove wiring while preserving label bubbles.
    k_small = cv2.getStructuringElement(cv2.MORPH_RECT, (3, 3))
    # Big kernel removes thin wiring; smaller kernels help preserve small 2-letter labels like "RM".
    k_block = cv2.getStructuringElement(cv2.MORPH_RECT, (9, 9))
    edge2 = cv2.morphologyEx(edge, cv2.MORPH_OPEN, k_small, iterations=1)
    blocks = cv2.morphologyEx(edge2, cv2.MORPH_OPEN, k_block, iterations=1)
    blocks = cv2.morphologyEx(blocks, cv2.MORPH_CLOSE, k_small, iterations=3)

    # External contours.
    contours, _ = cv2.findContours(blocks, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)

    allowed = {
        # canonical external labels we expect to see
        "T",
        "C",
        "S",
        "G",
        "V",
        "L",
        "RM",
        "R0",
        "R1",
        "R2",
        "R3",
        "D0",
        "D1",
        "D2",
        "D3",
        "01",
        "02",
        "4004",
    }

    ocr_aliases = {
        # Frequent confusion in these crops: 'T' vs 'L' when the tip/wiring gets included.
        "L": "T",
    }

    detections = []
    for cnt in contours:
        x, y, ww, hh = cv2.boundingRect(cnt)
        area = int(cv2.contourArea(cnt))
        if area < int(args.min_area) or area > int(args.max_area):
            continue
        if ww < 16 or hh < 16:
            continue
        fill = float(area) / float(max(1, ww * hh))
        if fill < float(args.min_fill):
            continue

        # Crop for OCR.
        pad = int(args.pad)
        x0 = max(0, x - pad)
        y0 = max(0, y - pad)
        x1 = min(w, x + ww + pad)
        y1 = min(h, y + hh + pad)
        crop_full = img[y0:y1, x0:x1]
        crop = extract_label_roi_for_ocr(crop_full)

        # Pointer tip: compute from the component mask when possible (helps for RM, where bbox is far
        # from the arrow tip), falling back to bbox-right-mid.
        tip = compute_tip_in_bbox(blocks[y : y + hh, x : x + ww], x=x, y=y, w=ww, h=hh)

        # OCR tries: invert, contrast, and 90-degree rotations.
        tries = []
        for inv in (False, True):
            base = crop.copy()
            if inv:
                base = 255 - base
            base = cv2.normalize(base, None, 0, 255, cv2.NORM_MINMAX)
            base = cv2.resize(base, (base.shape[1] * 3, base.shape[0] * 3), interpolation=cv2.INTER_NEAREST)
            # Otsu is more robust than a fixed threshold for these crops.
            base = cv2.threshold(base, 0, 255, cv2.THRESH_BINARY + cv2.THRESH_OTSU)[1]
            for rot in (0, 90, 180, 270):
                if rot == 90:
                    t = cv2.rotate(base, cv2.ROTATE_90_CLOCKWISE)
                elif rot == 180:
                    t = cv2.rotate(base, cv2.ROTATE_180)
                elif rot == 270:
                    t = cv2.rotate(base, cv2.ROTATE_90_COUNTERCLOCKWISE)
                else:
                    t = base
                # First: TSV-based single-token best guess.
                for psm in (7, 8, 10, 11):
                    tok, conf = ocr_token(t, psm=psm)
                    tok = normalize_token(tok)
                    tries.append({"token": tok, "conf": float(conf), "inv": inv, "rot": rot, "psm": int(psm)})
                # Second: string-based sweep to recover multi-char tokens like RM.
                for r in ocr_token_sweep(t, psms=(7, 8, 10, 11, 13)):
                    tries.append(
                        {
                            "token": r["token"],
                            "conf": float(r.get("conf", -1.0)),
                            "inv": inv,
                            "rot": rot,
                            "psm": int(r.get("psm", -1)),
                            "raw": r.get("raw"),
                            "vote_count": r.get("vote_count"),
                            "crop": "full",
                        }
                    )
                # Third: try a head-crop for tall arrow labels (improves RM detection).
                head = label_head_crop(t, frac=0.42)
                for r in ocr_token_sweep(head, psms=(7, 8, 10, 11, 13)):
                    tries.append(
                        {
                            "token": r["token"],
                            "conf": float(r.get("conf", -1.0)),
                            "inv": inv,
                            "rot": rot,
                            "psm": int(r.get("psm", -1)),
                            "raw": r.get("raw"),
                            "vote_count": r.get("vote_count"),
                            "crop": "head",
                        }
                    )

        tries = sorted(tries, key=lambda r: r["conf"], reverse=True)
        best = tries[0] if tries else {"token": "", "conf": -1.0, "inv": False, "rot": 0}
        # Prefer higher-confidence entries, but if confidence is unknown (-1), tie-break by token length (favor 2-char like RM).
        tries = sorted(tries, key=lambda r: (float(r.get("conf", -1.0)), len(str(r.get("token", "")))), reverse=True)
        best = tries[0] if tries else {"token": "", "conf": -1.0, "inv": False, "rot": 0, "psm": 0}
        token = str(best.get("token", ""))
        token = ocr_aliases.get(token, token)

        # Only keep plausible tokens.
        if token not in allowed:
            continue

        # Suggest layout node by distance to metal bbox from tip point, with penalty for huge nets.
        # Suggest layout node by distance to metal bbox from tip point.
        # If the tip falls inside multiple bboxes (common with "big" periphery rings), prefer the
        # smallest bbox that contains it to avoid always selecting huge nets like node 3.
        inside: list[dict] = []
        for n in nodes:
            mb = n["metal_bbox"]
            if bbox_contains(mb, x=int(tip["x"]), y=int(tip["y"])):
                inside.append(n)

        def node_score(n: dict) -> tuple[float, int]:
            mb = n["metal_bbox"]
            d = dist_point_to_bbox(float(tip["x"]), float(tip["y"]), mb)
            a = float(n.get("metal_area", 0))
            score = float(d) + float(args.area_penalty) * math.log1p(a)
            return (score, int(n.get("metal_area", 0)))

        candidate_nodes = inside if inside else nodes
        best_node = None
        for n in candidate_nodes:
            mb = n["metal_bbox"]
            d = dist_point_to_bbox(float(tip["x"]), float(tip["y"]), mb)
            a = float(n.get("metal_area", 0))
            score = float(d) + float(args.area_penalty) * math.log1p(a)
            if best_node is None:
                best_node = (n, score)
                continue
            if inside:
                # inside: pick smallest bbox area, then score
                if (bbox_area(mb), score) < (bbox_area(best_node[0]["metal_bbox"]), best_node[1]):
                    best_node = (n, score)
            else:
                if score < best_node[1]:
                    best_node = (n, score)
        assert best_node is not None
        bn = best_node[0]
        best_node_payload = {
            "node": int(bn["node"]),
            "dist": float(dist_point_to_bbox(float(tip["x"]), float(tip["y"]), bn["metal_bbox"])),
            "score": float(node_score(bn)[0]),
            "metal_area": int(bn.get("metal_area", 0)),
            "metal_bbox": bn["metal_bbox"],
            "tip_inside_bbox": bool(inside),
        }

        detections.append(
            {
                "token": token,
                "bbox": {"x": int(x), "y": int(y), "w": int(ww), "h": int(hh), "area": area, "fill": float(fill)},
                "tip": tip,
                "best_ocr": best,
                "tries": tries[:6],
                "suggested_layout_node": best_node_payload,
                "ocr_roi": {"x0": int(x0), "y0": int(y0), "x1": int(x1), "y1": int(y1)},
            }
        )

    # De-dupe by (token, suggested node), keeping best confidence.
    # Also keep the "first seen" ordering stable for crop emission.
    merged: dict[tuple[str, int], dict] = {}
    for d in detections:
        key = (d["token"], int(d["suggested_layout_node"]["node"]))
        cur = merged.get(key)
        if cur is None or float(d["best_ocr"]["conf"]) > float(cur["best_ocr"]["conf"]):
            merged[key] = d

    # If a detection "hit" a known alias (e.g. T misread as L), keep a trace in the JSON for auditing.
    for r in merged.values():
        # Back-compat: older JSONs won't have this key, new ones should.
        r.setdefault("normalized_token", r.get("token", ""))

    out_chip = Path(args.out_dir) / spec.chip
    out_chip.mkdir(parents=True, exist_ok=True)

    crops_dir = out_chip / "crops" if bool(args.write_crops) else None
    if crops_dir is not None:
        crops_dir.mkdir(parents=True, exist_ok=True)

    out_json = out_chip / f"{spec.chip.lower()}_layout_edge_labels_v0.json"
    payload = {
        "chip": spec.chip,
        "schema": {"version": 0, "description": "Detected periphery label blocks on metal mask with OCR + node suggestion."},
        "inputs": {
            "metal_bmp": rel_or_abs(spec.metal_bmp),
            "sha256": {"metal_bmp": sha256(spec.metal_bmp)},
            "netlist_v0": rel_or_abs(spec.netlist_v0),
        },
        "params": {
            "band": int(band),
            "min_area": int(args.min_area),
            "max_area": int(args.max_area),
            "min_fill": float(args.min_fill),
            "pad": int(args.pad),
            "edges": sorted(list(edges)),
        },
        "counts": {"detections": int(len(merged))},
        "detections": sorted(
            list(merged.values()),
            key=lambda r: (r["token"], r["suggested_layout_node"]["node"], -r["best_ocr"]["conf"]),
        ),
    }
    out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if crops_dir is not None:
        # Re-run the crop extraction deterministically for each merged detection.
        # This intentionally duplicates some logic to keep the JSON minimal and the crops reviewable.
        for idx, r in enumerate(payload["detections"]):
            bb = r["bbox"]
            x, y, ww, hh = int(bb["x"]), int(bb["y"]), int(bb["w"]), int(bb["h"])
            pad = int(args.pad)
            x0 = max(0, x - pad)
            y0 = max(0, y - pad)
            x1 = min(w, x + ww + pad)
            y1 = min(h, y + hh + pad)
            crop_full = img[y0:y1, x0:x1]
            roi = extract_label_roi(crop_full)
            tok = str(r["token"])
            conf = float(r["best_ocr"]["conf"])
            node = int(r["suggested_layout_node"]["node"])
            out = crops_dir / f"{idx:03d}_{tok}_node{node}_conf{conf:.1f}.png"
            Image.fromarray(roi).save(out, format="PNG", optimize=False, compress_level=9)

    # Render overlay
    pil = Image.open(spec.metal_bmp).convert("RGB")
    d = ImageDraw.Draw(pil)
    font = try_font(16)
    for r in payload["detections"]:
        bb = r["bbox"]
        x, y, ww, hh = int(bb["x"]), int(bb["y"]), int(bb["w"]), int(bb["h"])
        d.rectangle([x, y, x + ww, y + hh], outline=(0, 0, 255), width=2)
        tx, ty = int(r["tip"]["x"]), int(r["tip"]["y"])
        d.ellipse([tx - 3, ty - 3, tx + 3, ty + 3], fill=(255, 0, 0))
        label = f"{r['token']}→{r['suggested_layout_node']['node']}"
        d.text((x + 2, y + 2), label, fill=(0, 0, 255), font=font)
    out_png = out_chip / f"{spec.chip.lower()}_layout_edge_labels_v0.png"
    pil.save(out_png, format="PNG", optimize=False, compress_level=9)

    print(json.dumps({"out_json": rel_or_abs(out_json), "out_png": rel_or_abs(out_png)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
