#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def _bbox_xywh_to_x0x1y0y1(b: dict[str, Any]) -> dict[str, int]:
    x = int(b["x"])
    y = int(b["y"])
    w = int(b["w"])
    h = int(b["h"])
    if w <= 0 or h <= 0:
        raise ValueError(f"invalid bbox size: w={w} h={h}")
    return {"x0": x, "x1": x + w, "y0": y, "y1": y + h}


def _load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as f:
        return json.load(f)


def _save_json(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    with tmp.open("w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, sort_keys=True)
        f.write("\n")
    tmp.replace(path)


def main() -> None:
    ap = argparse.ArgumentParser(
        description=(
            "Sync select layout_bboxes in schematic_layout_anchors_v1.json from "
            "layout_edge_labels_v0 detections."
        )
    )
    ap.add_argument("--chip", required=True, help="Chip ID (e.g. 4004)")
    ap.add_argument("--anchors", required=True, type=Path, help="Anchors JSON (v1)")
    ap.add_argument(
        "--edge-labels",
        required=True,
        type=Path,
        help="layout_edge_labels_v0 JSON for this chip",
    )
    ap.add_argument(
        "--out",
        type=Path,
        help="Output path (defaults to overwriting --anchors)",
    )
    ap.add_argument(
        "--token-map",
        action="append",
        default=[],
        help="Override token mapping as TOKEN=ANCHOR (repeatable).",
    )
    ap.add_argument(
        "--signal",
        action="append",
        default=[],
        help="Limit sync to specific anchor signals (repeatable). Defaults to all.",
    )
    args = ap.parse_args()

    anchors_doc = _load_json(args.anchors)
    anchors_by_chip = anchors_doc.get("anchors")
    if not isinstance(anchors_by_chip, dict):
        raise SystemExit("anchors file missing top-level 'anchors' object")

    chip = str(args.chip)
    chip_anchors = anchors_by_chip.get(chip)
    if not isinstance(chip_anchors, dict):
        raise SystemExit(f"anchors file missing chip '{chip}'")

    edge = _load_json(args.edge_labels)
    detections = edge.get("detections")
    if not isinstance(detections, list):
        raise SystemExit("edge labels file missing 'detections' list")

    token_bbox: dict[str, dict[str, int]] = {}
    token_area: dict[str, int] = {}
    for d in detections:
        if not isinstance(d, dict):
            continue
        tok = d.get("token")
        bb = d.get("bbox")
        if not isinstance(tok, str) or not isinstance(bb, dict):
            continue
        area = int(bb.get("area") or (int(bb.get("w", 0)) * int(bb.get("h", 0))))
        if area <= 0:
            area = int(bb.get("w", 0)) * int(bb.get("h", 0))
        converted = _bbox_xywh_to_x0x1y0y1(bb)
        prev = token_area.get(tok, 0)
        if area <= prev:
            continue
        token_bbox[tok] = converted
        token_area[tok] = area

    signal_to_token: dict[str, str] = {
        "SYNC": "S",
        "POC": "C",
        "POC_PAD": "C",
        "TEST": "T",
        "TEST_PAD": "T",
    }
    for token_map in args.token_map or []:
        if "=" not in token_map:
            raise SystemExit(f"--token-map value must be TOKEN=ANCHOR, got '{token_map}'")
        token, signal = token_map.split("=", 1)
        token = token.strip()
        signal = signal.strip()
        if not token or not signal:
            raise SystemExit(f"invalid token map '{token_map}'")
        signal_to_token[signal] = token

    if args.signal:
        allowed = {sig.strip() for sig in args.signal if sig.strip()}
        signal_to_token = {sig: tok for sig, tok in signal_to_token.items() if sig in allowed}
        for req in allowed:
            if req not in signal_to_token:
                raise SystemExit(f"requested signal '{req}' is not in the default mapping")

    missing: list[str] = []
    for sig, tok in signal_to_token.items():
        rec = chip_anchors.get(sig)
        if not isinstance(rec, dict):
            missing.append(f"anchors[{chip}][{sig}]")
            continue
        bb = token_bbox.get(tok)
        if bb is None:
            missing.append(f"edge_labels[{chip}].token[{tok}]")
            continue
        rec["layout_bbox"] = bb

    test = chip_anchors.get("TEST")
    test_pad = chip_anchors.get("TEST_PAD")
    if isinstance(test, dict) and isinstance(test_pad, dict):
        if test.get("schematic_bbox") is None and test_pad.get("schematic_bbox") is not None:
            test["schematic_bbox"] = test_pad["schematic_bbox"]

    if missing:
        raise SystemExit("missing inputs:\n- " + "\n- ".join(missing))

    out = args.out or args.anchors
    _save_json(out, anchors_doc)


if __name__ == "__main__":
    main()
