#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


def _load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    p = argparse.ArgumentParser(description="Write a manual pad-label reading index for a chip (v0).")
    p.add_argument("--chip", required=True, choices=["4001", "4002", "4003", "4004"])
    p.add_argument("--in-dir", type=Path, default=ROOT / "docs" / "evidence" / "layout_pad_labels_v0")
    p.add_argument("--limit", type=int, default=120)
    args = p.parse_args()

    chip = str(args.chip)
    in_chip = (ROOT / args.in_dir / chip).resolve() if not args.in_dir.is_absolute() else (args.in_dir / chip).resolve()
    pad_json = in_chip / f"{chip.lower()}_layout_pad_labels_v0.json"
    sugg_json = in_chip / f"{chip.lower()}_pad_boxes_node_suggestions_v0.json"
    if not pad_json.exists() or not sugg_json.exists():
        raise SystemExit(f"missing inputs: {pad_json} or {sugg_json}")

    pad_obj = _load(pad_json)
    sugg_obj = _load(sugg_json)
    boxes = pad_obj.get("boxes", [])
    sugg = sugg_obj.get("suggestions", [])
    if not isinstance(boxes, list) or not isinstance(sugg, list):
        raise SystemExit("unexpected json shape")

    out_md = in_chip / "manual_readings_v0.md"
    lines: list[str] = []
    lines.append(f"# {chip} metal-mask pad label readings (manual, v0)")
    lines.append("")
    lines.append("This file is a human-maintained index for converting metal-mask pad-label crops into anchors.")
    lines.append(f"- Canonical crops: `docs/evidence/layout_pad_labels_v0/{chip}/human_crops/`")
    lines.append("")
    lines.append("## Candidates")
    lines.append("")
    lines.append("| idx | suggested_node | ocr_best | crop | printed_label | anchor_name | notes |")
    lines.append("|---:|---:|---|---|---|---|---|")

    limit = min(int(args.limit), len(sugg))
    for i in range(limit):
        row = sugg[i]
        if not isinstance(row, dict) or not isinstance(row.get("suggested"), dict):
            continue
        node = row["suggested"].get("node")
        crop = f"docs/evidence/layout_pad_labels_v0/{chip}/human_crops/box_{i:03d}_node_{node}.png"
        ocr_best = ""
        if i < len(boxes) and isinstance(boxes[i], dict):
            ocr = boxes[i].get("ocr")
            if isinstance(ocr, dict) and isinstance(ocr.get("best"), str):
                ocr_best = str(ocr["best"])
        lines.append(f"| {i} | {int(node) if isinstance(node, int) else ''} | `{ocr_best}` | `{crop}` |  |  |  |")

    out_md.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(str(out_md.relative_to(ROOT)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

