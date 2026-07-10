#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


def _load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _xywh_to_bbox(xywh: dict[str, Any]) -> dict[str, int]:
    x = int(xywh["x"])
    y = int(xywh["y"])
    w = int(xywh["w"])
    h = int(xywh["h"])
    return {"x0": x, "y0": y, "x1": x + w, "y1": y + h}


def _ensure_bbox(b: dict[str, Any]) -> dict[str, int]:
    if {"x0", "y0", "x1", "y1"} <= set(b.keys()):
        return {"x0": int(b["x0"]), "y0": int(b["y0"]), "x1": int(b["x1"]), "y1": int(b["y1"])}
    if {"x", "y", "w", "h"} <= set(b.keys()):
        return _xywh_to_bbox(b)
    raise ValueError(f"unsupported bbox keys: {sorted(b.keys())}")


def _build_4004() -> dict[str, dict[str, int]]:
    pad_labels = _load(ROOT / "docs/evidence/layout_pad_labels_v0/4004/4004_layout_pad_labels_v0.json")
    edge_labels = _load(ROOT / "docs/evidence/layout_edge_labels_v0/4004/4004_layout_edge_labels_v0.json")

    pad_boxes = pad_labels.get("boxes")
    if not isinstance(pad_boxes, list) or len(pad_boxes) < 104:
        raise SystemExit("pad label boxes missing/unexpected shape")

    # Map anchors to known pad-box indices (see docs/evidence/layout_pad_labels_v0/4004/manual_readings_v0.md).
    out: dict[str, dict[str, int]] = {}
    box_index_map = {
        "CLK1": 16,  # printed `01`
        "D3_PAD": 67,  # printed `D3`
        "D2_PAD": 102,  # printed `D2`
        "D1_PAD": 103,  # printed `D1`
        "CMRAM1": 100,  # printed `R1` (used as CMRAM1)
        "CMRAM0": 101,  # printed `R0` (used as CMRAM0)
    }
    for anchor, idx in box_index_map.items():
        b = pad_boxes[int(idx)]
        if not isinstance(b, dict) or not isinstance(b.get("bbox"), dict):
            continue
        out[str(anchor)] = _ensure_bbox(b["bbox"])

    dets = edge_labels.get("detections")
    if not isinstance(dets, list):
        raise SystemExit("edge label detections missing")
    # Prefer edge-label detection for top-edge callouts and long-arrow labels.
    token_to_anchor = {
        "R2": "CMRAM2",
        "R3": "CMRAM3",
        "RM": "CMROM",
        "C": "POC_PAD",
        "S": "SYNC",
        "T": "TEST_PAD",
        "02": "CLK2",
    }
    for d in dets:
        if not isinstance(d, dict) or not isinstance(d.get("token"), str):
            continue
        tok = str(d["token"]).strip()
        anchor = token_to_anchor.get(tok)
        if not anchor:
            continue
        bbox = d.get("bbox")
        if not isinstance(bbox, dict):
            continue
        out[str(anchor)] = _ensure_bbox(bbox)
    return out


def _parse_manual_anchor_rows(path: Path) -> dict[int, str]:
    lines = path.read_text(encoding="utf-8").splitlines()
    start = -1
    for i, line in enumerate(lines):
        if line.strip().startswith("| idx |") and "anchor_name" in line:
            start = i + 2  # skip header and separator
            break
    if start < 0:
        return {}

    out: dict[int, str] = {}
    for line in lines[start:]:
        if not line.strip().startswith("|"):
            break
        cols = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cols) != 7:
            continue
        try:
            idx = int(cols[0])
        except ValueError:
            # A non-numeric index cell marks a header or separator row; skip it.
            continue
        anchor_name = cols[5].strip()
        if anchor_name and anchor_name != "*":
            out[int(idx)] = str(anchor_name)
    return out


def _build_from_pad_labels(*, chip: str) -> dict[str, dict[str, int]]:
    pad_labels = _load(ROOT / f"docs/evidence/layout_pad_labels_v0/{chip}/{chip}_layout_pad_labels_v0.json")
    boxes = pad_labels.get("boxes")
    if not isinstance(boxes, list) or not boxes:
        raise SystemExit(f"{chip}: pad label boxes missing")

    # We currently trust the geometry-based node suggestion pipeline; human_crops include the box index.
    # So we can treat `idx` in the manual readings table as the canonical index into pad_labels.boxes.
    manual_path = ROOT / f"docs/evidence/layout_pad_labels_v0/{chip}/manual_readings_v0.md"
    idx_to_anchor = _parse_manual_anchor_rows(manual_path)
    if not idx_to_anchor:
        return {}

    out: dict[str, dict[str, int]] = {}
    for idx, anchor in idx_to_anchor.items():
        if idx < 0 or idx >= len(boxes):
            continue
        b = boxes[int(idx)]
        if not isinstance(b, dict) or not isinstance(b.get("bbox"), dict):
            continue
        out[str(anchor)] = _ensure_bbox(b["bbox"])
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description="Build per-anchor label bbox map from existing label detection artifacts (v0).")
    ap.add_argument("--chip", default="4004", help="Chip number (4001/4002/4003/4004)")
    ap.add_argument("--out", type=Path, required=True, help="Write bbox map JSON here")
    args = ap.parse_args()

    chip = str(args.chip).strip()
    if chip == "4004":
        anchor_map = _build_4004()
    elif chip in {"4001", "4002", "4003"}:
        anchor_map = _build_from_pad_labels(chip=chip)
    else:
        raise SystemExit("unsupported chip")

    out_map = {"schema": "anchor_label_bboxes_v0", "anchors": {chip: anchor_map}}

    out = args.out
    if not out.is_absolute():
        out = (ROOT / out).resolve()
    out.parent.mkdir(parents=True, exist_ok=True)
    if out.exists():
        try:
            existing = _load(out)
        except Exception:
            existing = {}
        if isinstance(existing, dict) and isinstance(existing.get("anchors"), dict):
            existing["schema"] = out_map["schema"]
            existing["anchors"][chip] = anchor_map
            out_map = existing

    out.write_text(json.dumps(out_map, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(str(out))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
