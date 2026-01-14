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


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Attach ranked power-rail node candidates into schematic_layout_anchors_v1.json for human review (v1)."
    )
    ap.add_argument("--chip", action="append", choices=["4001", "4002", "4003", "4004"], help="Chip (repeatable).")
    ap.add_argument("--all", action="store_true", help="Process all chips.")
    ap.add_argument(
        "--anchors",
        type=Path,
        default=ROOT / "docs" / "evidence" / "schematic_layout_anchors_v1.json",
        help="Anchors JSON (v1).",
    )
    ap.add_argument(
        "--candidates-root",
        type=Path,
        default=ROOT / "docs" / "evidence" / "power_rail_candidates_v0",
        help="Root folder containing per-chip candidate JSONs.",
    )
    ap.add_argument("--top", type=int, default=10, help="How many candidates to attach to each rail anchor.")
    ap.add_argument("--out", type=Path, default=None, help="Write updated anchors JSON here (defaults overwrite).")
    args = ap.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = {"4001", "4002", "4003", "4004"}
    if not selected:
        ap.error("select --all or at least one --chip")

    anchors_path = (ROOT / args.anchors).resolve() if not args.anchors.is_absolute() else args.anchors
    out_path = (ROOT / args.out).resolve() if args.out and not args.out.is_absolute() else (args.out or anchors_path)
    candidates_root = (ROOT / args.candidates_root).resolve() if not args.candidates_root.is_absolute() else args.candidates_root

    anchors = _load(anchors_path)
    aroot = anchors.get("anchors")
    if not isinstance(aroot, dict):
        raise SystemExit("anchors file missing top-level 'anchors'")

    updated = 0
    for chip in sorted(selected):
        block = aroot.get(chip)
        if not isinstance(block, dict):
            continue

        rails = ["VSS"]
        if chip == "4004":
            rails.append("VCC")
        else:
            rails.append("VDD")

        cand_path = candidates_root / chip / f"{chip}_power_rail_candidates_v0.json"
        if not cand_path.exists():
            continue
        cand = _load(cand_path)
        cands = cand.get("candidates", [])
        if not isinstance(cands, list) or not cands:
            continue
        top = []
        for c in cands[: int(args.top)]:
            if not isinstance(c, dict) or not isinstance(c.get("node"), int):
                continue
            top.append(
                {
                    "node": int(c["node"]),
                    "bbox": c.get("bbox"),
                    "edge_distance": c.get("edge_distance"),
                    "incidence": c.get("incidence"),
                    "bbox_area": c.get("bbox_area"),
                }
            )

        for rail in rails:
            row = block.get(rail)
            if not isinstance(row, dict):
                continue
            if row.get("layout_node") is not None:
                continue
            row["layout_candidates_v0"] = {
                "source": str(cand_path.relative_to(ROOT)) if cand_path.is_relative_to(ROOT) else str(cand_path),
                "top": int(args.top),
                "candidates": top,
            }
            updated += 1

    anchors.setdefault("notes", [])
    if isinstance(anchors["notes"], list):
        anchors["notes"].append(
            {
                "kind": "attach_power_rail_candidates_v1",
                "chips": sorted(selected),
                "candidates_root": str(candidates_root.relative_to(ROOT)) if candidates_root.is_relative_to(ROOT) else str(candidates_root),
                "top": int(args.top),
                "updated": int(updated),
            }
        )

    _write(out_path, anchors)
    print(json.dumps({"out": str(out_path), "updated": int(updated)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

