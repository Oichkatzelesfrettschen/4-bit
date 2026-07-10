#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections.abc import Iterable
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def _iter_signal_names(signal_list: str) -> Iterable[str]:
    for raw in signal_list.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith(";"):
            # Section separators in the signal-list format.
            continue
        parts = [p.strip() for p in line.split(",")]
        if len(parts) < 3:
            continue
        name = parts[2]
        if not name or name.startswith(";"):
            continue
        # Keep only label-like anchors; skip complex expressions (contain operators or long length).
        if any(ch in name for ch in "()+=*[]"):
            continue
        if len(name) > 16:
            continue
        yield name


def main() -> int:
    ap = argparse.ArgumentParser(description="Populate schematic_layout_anchors_v0.json from i400x-signals.txt lists.")
    ap.add_argument(
        "--chips",
        nargs="+",
        default=["4001", "4002", "4003"],
        help="Chip ids to sync (e.g., 4001 4002 4003 4004).",
    )
    ap.add_argument(
        "--anchors",
        type=Path,
        default=ROOT / "docs" / "evidence" / "schematic_layout_anchors_v0.json",
        help="Input anchors JSON (v0).",
    )
    ap.add_argument(
        "--out",
        type=Path,
        default=None,
        help="Write updated anchors JSON here (defaults to in-place overwrite of --anchors).",
    )
    args = ap.parse_args()

    anchors_path = (ROOT / args.anchors).resolve() if not args.anchors.is_absolute() else args.anchors
    out_path = (ROOT / args.out).resolve() if args.out and not args.out.is_absolute() else (args.out or anchors_path)

    anchors = _load_json(anchors_path)
    aroot = anchors.get("anchors")
    if not isinstance(aroot, dict):
        raise SystemExit("anchors file missing top-level 'anchors' object")

    notes = anchors.get("notes")
    if notes is None:
        anchors["notes"] = []
        notes = anchors["notes"]
    if not isinstance(notes, list):
        raise SystemExit("anchors file has non-list 'notes'")

    added_total = 0
    for chip in [str(c).strip() for c in args.chips]:
        sig_path = ROOT / "docs" / "emulators" / f"i{chip}-signals.txt"
        if not sig_path.exists():
            raise SystemExit(f"missing signal list: {sig_path}")
        names = sorted(set(_iter_signal_names(sig_path.read_text(encoding="utf-8", errors="replace"))))
        block = aroot.get(chip)
        if block is None:
            aroot[chip] = {}
            block = aroot[chip]
        if not isinstance(block, dict):
            raise SystemExit(f"anchors['{chip}'] is not an object")
        before = set(block.keys())
        for name in names:
            if name in block:
                continue
            block[name] = {
                "layout_node": None,
                "note": f"From `{sig_path.relative_to(ROOT)}`; layout node not yet mapped.",
            }
            added_total += 1
        added_chip = len(set(block.keys()) - before)

        notes.append(
            {
                "kind": "sync_signal_anchors_v0",
                "chip": chip,
                "signal_list": str(sig_path.relative_to(ROOT)),
                "added": int(added_chip),
                "total": int(len(block)),
            }
        )

    _write_json(out_path, anchors)
    print(json.dumps({"out": str(out_path), "added_total": int(added_total)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
