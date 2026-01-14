#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    ap = argparse.ArgumentParser(description="Merge a single chip anchor block from one anchors JSON into another.")
    ap.add_argument("--chip", required=True, help="Chip id to merge (e.g., 4003).")
    ap.add_argument("--src", type=Path, required=True, help="Source anchors JSON (e.g., tmp v1 output).")
    ap.add_argument("--dst", type=Path, required=True, help="Destination anchors JSON (will be updated).")
    ap.add_argument("--out", type=Path, default=None, help="Write merged JSON here (defaults to --dst in-place).")
    args = ap.parse_args()

    chip = str(args.chip).strip()
    src_path = (ROOT / args.src).resolve() if not args.src.is_absolute() else args.src
    dst_path = (ROOT / args.dst).resolve() if not args.dst.is_absolute() else args.dst
    out_path = (ROOT / args.out).resolve() if args.out and not args.out.is_absolute() else (args.out or dst_path)

    src = _load_json(src_path)
    dst = _load_json(dst_path)

    if not isinstance(src.get("anchors"), dict) or not isinstance(dst.get("anchors"), dict):
        raise SystemExit("both src and dst must have top-level 'anchors' objects")
    if chip not in src["anchors"]:
        raise SystemExit(f"src missing anchors[{chip!r}]")
    if not isinstance(src["anchors"][chip], dict):
        raise SystemExit(f"src anchors[{chip!r}] is not an object")

    dst["anchors"][chip] = src["anchors"][chip]

    # Preserve destination notes but append source notes relevant to this chip (avoid duplicates).
    src_notes = src.get("notes") if isinstance(src.get("notes"), list) else []
    dst_notes = dst.get("notes")
    if dst_notes is None:
        dst["notes"] = []
        dst_notes = dst["notes"]
    if not isinstance(dst_notes, list):
        raise SystemExit("dst has non-list 'notes'")

    def _canon_note(n: dict[str, Any]) -> str:
        # Stable-ish key: drop timestamps/paths if present; keep chip+kind+method+params.
        keep = {k: n.get(k) for k in ("kind", "chip", "method", "max_dist", "min_incident", "area_ratio_weight")}
        return json.dumps(keep, sort_keys=True, separators=(",", ":"))

    seen = {_canon_note(n) for n in dst_notes if isinstance(n, dict)}
    appended = 0
    for n in src_notes:
        if not isinstance(n, dict) or str(n.get("chip")) != chip:
            continue
        key = _canon_note(n)
        if key in seen:
            continue
        dst_notes.append(n)
        seen.add(key)
        appended += 1

    _write_json(out_path, dst)
    print(json.dumps({"out": str(out_path), "chip": chip, "appended_notes": appended}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

