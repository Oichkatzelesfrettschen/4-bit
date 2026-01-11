#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path

from PIL import Image


@dataclass(frozen=True)
class ChipSpec:
    chip: str
    schematic_bmp: Path
    signals_txt: Path


ROOT = Path(__file__).resolve().parents[1]


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


def parse_signals_txt(path: Path) -> tuple[list[SignalPoint], list[dict[str, object]]]:
    points: list[SignalPoint] = []
    errors: list[dict[str, object]] = []

    raw = path.read_text(encoding="utf-8", errors="replace")
    for i, line in enumerate(raw.splitlines(), start=1):
        s = line.strip().strip("\ufeff")
        if not s or s.startswith(";"):
            continue
        parts = [p.strip() for p in s.split(",")]
        if len(parts) != 3:
            errors.append({"line": i, "kind": "format", "raw": line})
            continue
        try:
            x = int(parts[0])
            y = int(parts[1])
        except ValueError:
            errors.append({"line": i, "kind": "coord_parse", "raw": line})
            continue
        name = parts[2]
        if not name:
            errors.append({"line": i, "kind": "empty_name", "raw": line})
            continue
        points.append(SignalPoint(x=x, y=y, name=name, line_no=i))

    return points, errors


def main() -> int:
    parser = argparse.ArgumentParser(description="Sanity-check docs/emulators/i400{1,2,3,4}-signals.txt coordinate maps.")
    parser.add_argument("--chip", action="append", choices=sorted(specs().keys()), help="Chip to check (repeatable)")
    parser.add_argument("--all", action="store_true", help="Check all supported chips")
    parser.add_argument(
        "--out",
        type=Path,
        default=ROOT / "docs" / "evidence" / "ocr_signal_labels" / "signals_txt_audit.json",
        help="Where to write JSON audit output",
    )
    args = parser.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = set(specs().keys())
    if not selected:
        parser.error("select --all or at least one --chip")

    out: dict[str, object] = {
        "tool": "scripts/verify_signals_txt.py",
        "schema": {
            "format": "x,y,name",
            "coordinates": "pixel",
            "origin": "top-left",
            "indexing": "0-based",
        },
        "outputs": [],
    }
    had_errors = False

    for chip in sorted(selected):
        spec = specs()[chip]
        img = Image.open(spec.schematic_bmp)
        w, h = img.size

        points, errors = parse_signals_txt(spec.signals_txt)

        out_of_bounds: list[dict[str, object]] = []
        for p in points:
            if p.x < 0 or p.y < 0 or p.x >= w or p.y >= h:
                out_of_bounds.append({"line": p.line_no, "x": p.x, "y": p.y, "name": p.name})

        if errors or out_of_bounds:
            had_errors = True

        out["outputs"].append(
            {
                "chip": chip,
                "inputs": {
                    "schematic_bmp": str(spec.schematic_bmp.relative_to(ROOT)),
                    "signals_txt": str(spec.signals_txt.relative_to(ROOT)),
                    "schematic_size": {"w": w, "h": h},
                },
                "counts": {
                    "points": len(points),
                    "parse_errors": len(errors),
                    "out_of_bounds": len(out_of_bounds),
                },
                "parse_errors": errors,
                "out_of_bounds": out_of_bounds,
            }
        )

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(out, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {args.out.relative_to(ROOT) if args.out.is_relative_to(ROOT) else args.out}")

    return 1 if had_errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
