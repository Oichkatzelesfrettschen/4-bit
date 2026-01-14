#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


def _load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _write(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def _ensure(block: dict[str, Any], name: str, note: str) -> bool:
    if name in block:
        return False
    block[name] = {"layout_node": None, "note": note}
    return True


def main() -> int:
    ap = argparse.ArgumentParser(description="Ensure power/reset anchors exist in schematic_layout_anchors_v1.json (placeholders if needed).")
    ap.add_argument(
        "--anchors",
        type=Path,
        default=ROOT / "docs" / "evidence" / "schematic_layout_anchors_v1.json",
        help="Anchors JSON (v1).",
    )
    ap.add_argument("--out", type=Path, default=None, help="Write updated JSON here (defaults to overwrite --anchors).")
    args = ap.parse_args()

    anchors_path = (ROOT / args.anchors).resolve() if not args.anchors.is_absolute() else args.anchors
    out_path = (ROOT / args.out).resolve() if args.out and not args.out.is_absolute() else (args.out or anchors_path)

    payload = _load(anchors_path)
    aroot = payload.get("anchors")
    if not isinstance(aroot, dict):
        raise SystemExit("anchors file missing top-level 'anchors' object")

    notes = payload.get("notes")
    if notes is None:
        payload["notes"] = []
        notes = payload["notes"]
    if not isinstance(notes, list):
        raise SystemExit("anchors file has non-list 'notes'")

    # Canonical names used in this repo:
    # - 4001/4002/4003: VSS/VDD (PMOS supply naming)
    # - 4004: VSS/VCC plus RESET as an external pin even if not present in i4004-signals.txt
    want: dict[str, list[str]] = {
        "4001": ["VSS", "VDD"],
        "4002": ["VSS", "VDD"],
        "4003": ["VSS", "VDD"],
        "4004": ["VSS", "VCC", "RESET"],
    }

    added: list[dict[str, Any]] = []
    for chip, names in want.items():
        block = aroot.get(chip)
        if block is None:
            raise SystemExit(f"anchors missing chip block: {chip}")
        if not isinstance(block, dict):
            raise SystemExit(f"anchors['{chip}'] is not an object")
        for name in names:
            did = _ensure(
                block,
                name,
                note=(
                    f"Placeholder anchor (power/reset). Not currently present in docs/emulators/i{chip}-signals.txt; "
                    "to be resolved via primary-source pinouts + power-rail identification in netlist_v1."
                ),
            )
            if did:
                added.append({"chip": chip, "signal": name})

    if added:
        notes.append({"kind": "ensure_power_reset_anchors_v1", "added": added})

    _write(out_path, payload)
    print(json.dumps({"out": str(out_path), "added": added}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

