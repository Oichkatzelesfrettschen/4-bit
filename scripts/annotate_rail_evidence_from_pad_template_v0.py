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
        description="Annotate VSS/VDD/VCC anchors with rail_evidence_v0 derived from pad_pin_template_v0 seeding (v0)."
    )
    ap.add_argument(
        "--anchors",
        type=Path,
        default=ROOT / "docs" / "evidence" / "schematic_layout_anchors_v1.json",
        help="Anchors JSON (v1).",
    )
    ap.add_argument("--chips", nargs="*", default=["4001", "4002", "4003"], help="Chip blocks to update.")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    anchors_path = (ROOT / args.anchors).resolve() if not args.anchors.is_absolute() else args.anchors
    payload = _load(anchors_path)

    aroot = payload.get("anchors")
    if not isinstance(aroot, dict):
        raise SystemExit("anchors file missing top-level 'anchors' object")

    updates: list[dict[str, Any]] = []
    for chip in [str(c).strip() for c in args.chips]:
        block = aroot.get(chip)
        if not isinstance(block, dict):
            continue
        for rail in ("VSS", "VDD", "VCC"):
            row = block.get(rail)
            if not isinstance(row, dict):
                continue
            if row.get("rail_evidence_v0") is not None:
                continue
            seed = row.get("layout_seed_v0")
            if not isinstance(seed, dict) or seed.get("kind") != "pad_pin_template_v0":
                continue
            # This is intentionally conservative: record what we did and why it's weak.
            ev = {
                "kind": "pad_pin_template_v0",
                "template": seed.get("template"),
                "pad_idx": seed.get("pad_idx"),
                "pin_dip": seed.get("pin_dip"),
                "seed_node": seed.get("seed_node"),
                "assumptions": [
                    "Periphery pad ordering matches DIP pin ordering counter-clockwise (CCW).",
                    "Primary-source pinouts correctly identify VSS/VDD/VCC pins.",
                    "Remapped layout_node may land on an internal trunk net; treat layout_node_src as the pad metal seed.",
                ],
                "confidence": float(seed.get("confidence", 0.0) or 0.0),
            }
            row["rail_evidence_v0"] = ev
            updates.append({"chip": chip, "rail": rail, "layout_node": row.get("layout_node"), "layout_node_src": row.get("layout_node_src")})

    if updates:
        payload["notes"] = list(payload.get("notes") or [])
        if isinstance(payload["notes"], list):
            payload["notes"].append({"kind": "annotate_rail_evidence_from_pad_template_v0", "updates": updates})

    if args.dry_run:
        print(json.dumps({"updates": updates}, indent=2))
        return 0

    _write(anchors_path, payload)
    print(json.dumps({"out": str(anchors_path), "updates": updates}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

