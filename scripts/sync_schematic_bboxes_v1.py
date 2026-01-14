#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class ChipSpec:
    chip: str
    schematic_bmp: Path
    signals_txt: Path


def specs() -> dict[str, ChipSpec]:
    def emu(p: str) -> Path:
        return ROOT / "docs" / "emulators" / p

    return {
        "4001": ChipSpec("4001", emu("i4001-schematic.bmp"), emu("i4001-signals.txt")),
        "4002": ChipSpec("4002", emu("i4002-schematic.bmp"), emu("i4002-signals.txt")),
        "4003": ChipSpec("4003", emu("i4003-schematic.bmp"), emu("i4003-signals.txt")),
        "4004": ChipSpec("4004", emu("i4004-schematic.bmp"), emu("i4004-signals.txt")),
    }


@dataclass(frozen=True)
class SignalPoint:
    x: int
    y: int
    name: str
    line_no: int


def parse_signals_txt(path: Path) -> dict[str, SignalPoint]:
    points: dict[str, SignalPoint] = {}
    raw = path.read_text(encoding="utf-8", errors="replace")
    for i, line in enumerate(raw.splitlines(), start=1):
        s = line.strip().strip("\ufeff").strip("\r")
        if not s or s.startswith(";"):
            continue
        parts = [p.strip() for p in s.split(",")]
        if len(parts) != 3:
            continue
        try:
            x = int(parts[0])
            y = int(parts[1])
        except ValueError:
            continue
        name = parts[2]
        if not name:
            continue
        # First definition wins; duplicates are typically aliases in later sections.
        points.setdefault(name, SignalPoint(x=x, y=y, name=name, line_no=i))
    return points


def clamp_bbox(*, x: int, y: int, w: int, h: int, img_w: int, img_h: int) -> dict[str, int]:
    x0 = int(round(x - w / 2))
    y0 = int(round(y - h / 2))
    x0 = max(0, min(x0, img_w - 1))
    y0 = max(0, min(y0, img_h - 1))
    x1 = min(img_w, x0 + int(w))
    y1 = min(img_h, y0 + int(h))
    # Ensure non-empty bbox even near edges.
    if x1 <= x0:
        x1 = min(img_w, x0 + 1)
    if y1 <= y0:
        y1 = min(img_h, y0 + 1)
    return {"x0": int(x0), "y0": int(y0), "x1": int(x1), "y1": int(y1)}


def main() -> int:
    ap = argparse.ArgumentParser(description="Populate schematic_bbox/schematic_point in schematic_layout_anchors_v1.json from i400x-signals.txt.")
    ap.add_argument("--chip", action="append", choices=sorted(specs().keys()), help="Chip to sync (repeatable).")
    ap.add_argument("--all", action="store_true", help="Sync all supported chips.")
    ap.add_argument(
        "--anchors",
        type=Path,
        default=ROOT / "docs" / "evidence" / "schematic_layout_anchors_v1.json",
        help="Input anchors JSON (v1).",
    )
    ap.add_argument("--out", type=Path, default=None, help="Write updated JSON here (defaults to overwrite --anchors).")
    ap.add_argument("--bbox-w", type=int, default=160, help="Default schematic bbox width (px).")
    ap.add_argument("--bbox-h", type=int, default=160, help="Default schematic bbox height (px).")
    ap.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite existing schematic_bbox/schematic_point entries (default: only fill missing).",
    )
    args = ap.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = set(specs().keys())
    if not selected:
        ap.error("select --all or at least one --chip")

    anchors_path = (ROOT / args.anchors).resolve() if not args.anchors.is_absolute() else args.anchors
    out_path = (ROOT / args.out).resolve() if args.out and not args.out.is_absolute() else (args.out or anchors_path)

    payload = json.loads(anchors_path.read_text(encoding="utf-8"))
    aroot = payload.get("anchors")
    if not isinstance(aroot, dict):
        raise SystemExit("anchors file missing top-level 'anchors' object")

    notes = payload.get("notes")
    if notes is None:
        payload["notes"] = []
        notes = payload["notes"]
    if not isinstance(notes, list):
        raise SystemExit("anchors file has non-list 'notes'")

    updated_total = 0
    for chip in sorted(selected):
        spec = specs()[chip]
        if not spec.schematic_bmp.exists():
            raise SystemExit(f"missing schematic bmp: {spec.schematic_bmp}")
        if not spec.signals_txt.exists():
            raise SystemExit(f"missing signals txt: {spec.signals_txt}")

        with Image.open(spec.schematic_bmp) as img:
            img_w, img_h = img.size
        points = parse_signals_txt(spec.signals_txt)

        block = aroot.get(chip)
        if block is None:
            raise SystemExit(f"anchors missing chip block: {chip}")
        if not isinstance(block, dict):
            raise SystemExit(f"anchors['{chip}'] is not an object")

        updated_chip = 0
        for name, rec in block.items():
            if not isinstance(rec, dict):
                continue
            pt = points.get(name)
            if pt is None:
                continue
            if not args.overwrite and rec.get("schematic_bbox") is not None:
                continue
            rec["schematic_point"] = {
                "x": int(pt.x),
                "y": int(pt.y),
                "src": f"{spec.signals_txt.relative_to(ROOT)}:{pt.line_no}",
            }
            rec["schematic_bbox"] = clamp_bbox(
                x=int(pt.x),
                y=int(pt.y),
                w=int(args.bbox_w),
                h=int(args.bbox_h),
                img_w=int(img_w),
                img_h=int(img_h),
            )
            updated_chip += 1
        updated_total += updated_chip

        notes.append(
            {
                "kind": "sync_schematic_bboxes_v1",
                "chip": chip,
                "signals_txt": str(spec.signals_txt.relative_to(ROOT)),
                "schematic_bmp": str(spec.schematic_bmp.relative_to(ROOT)),
                "bbox": {"w": int(args.bbox_w), "h": int(args.bbox_h)},
                "updated": int(updated_chip),
            }
        )

    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"out": str(out_path), "updated_total": int(updated_total)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
