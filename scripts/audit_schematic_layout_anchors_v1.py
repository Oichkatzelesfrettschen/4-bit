#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class ChipSpec:
    chip: str
    signals_txt: Path


def specs() -> dict[str, ChipSpec]:
    def emu(p: str) -> Path:
        return ROOT / "docs" / "emulators" / p

    return {
        "4001": ChipSpec("4001", emu("i4001-signals.txt")),
        "4002": ChipSpec("4002", emu("i4002-signals.txt")),
        "4003": ChipSpec("4003", emu("i4003-signals.txt")),
        "4004": ChipSpec("4004", emu("i4004-signals.txt")),
    }


def parse_required_anchors_from_signals_txt(path: Path) -> list[str]:
    """
    Treat the first block (above the first ';' separator) as the "pad label" anchor set.
    """
    out: list[str] = []
    raw = path.read_text(encoding="utf-8", errors="replace")
    for line in raw.splitlines():
        s = line.strip().strip("\ufeff").strip("\r")
        if not s:
            continue
        if s.startswith(";"):
            break
        parts = [p.strip() for p in s.split(",")]
        if len(parts) != 3:
            continue
        name = parts[2]
        if not name:
            continue
        out.append(name)
    # De-dupe preserving order.
    seen: set[str] = set()
    uniq: list[str] = []
    for name in out:
        if name in seen:
            continue
        uniq.append(name)
        seen.add(name)
    return uniq


def main() -> int:
    ap = argparse.ArgumentParser(description="Audit schematic_layout_anchors_v1.json coverage against i400x-signals.txt pad blocks.")
    ap.add_argument("--chip", action="append", choices=sorted(specs().keys()), help="Chip to audit (repeatable).")
    ap.add_argument("--all", action="store_true", help="Audit all supported chips.")
    ap.add_argument(
        "--anchors",
        type=Path,
        default=ROOT / "docs" / "evidence" / "schematic_layout_anchors_v1.json",
        help="Anchors JSON (v1).",
    )
    ap.add_argument(
        "--out",
        type=Path,
        default=ROOT / "docs" / "evidence" / "anchor_audit_v1.json",
        help="Where to write JSON audit output.",
    )
    args = ap.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = set(specs().keys())
    if not selected:
        ap.error("select --all or at least one --chip")

    anchors_path = (ROOT / args.anchors).resolve() if not args.anchors.is_absolute() else args.anchors
    payload = json.loads(anchors_path.read_text(encoding="utf-8"))
    aroot = payload.get("anchors")
    if not isinstance(aroot, dict):
        raise SystemExit("anchors file missing top-level 'anchors' object")

    out: dict[str, object] = {
        "tool": "scripts/audit_schematic_layout_anchors_v1.py",
        "inputs": {"anchors": str(anchors_path.relative_to(ROOT) if anchors_path.is_relative_to(ROOT) else anchors_path)},
        "outputs": [],
    }

    had_errors = False
    for chip in sorted(selected):
        spec = specs()[chip]
        required = parse_required_anchors_from_signals_txt(spec.signals_txt)
        block = aroot.get(chip, {})
        if not isinstance(block, dict):
            raise SystemExit(f"anchors['{chip}'] is not an object")

        missing_entries: list[str] = []
        missing_layout_node: list[str] = []
        missing_schematic_bbox: list[str] = []
        for name in required:
            rec = block.get(name)
            if rec is None:
                missing_entries.append(name)
                continue
            if rec.get("layout_node") is None:
                missing_layout_node.append(name)
            if rec.get("schematic_bbox") is None:
                missing_schematic_bbox.append(name)

        if missing_entries or missing_layout_node or missing_schematic_bbox:
            had_errors = True

        out["outputs"].append(
            {
                "chip": chip,
                "inputs": {"signals_txt": str(spec.signals_txt.relative_to(ROOT))},
                "required": required,
                "counts": {
                    "required": len(required),
                    "missing_entries": len(missing_entries),
                    "missing_layout_node": len(missing_layout_node),
                    "missing_schematic_bbox": len(missing_schematic_bbox),
                },
                "missing_entries": missing_entries,
                "missing_layout_node": missing_layout_node,
                "missing_schematic_bbox": missing_schematic_bbox,
            }
        )

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(out, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {args.out.relative_to(ROOT) if args.out.is_relative_to(ROOT) else args.out}")
    return 1 if had_errors else 0


if __name__ == "__main__":
    raise SystemExit(main())

