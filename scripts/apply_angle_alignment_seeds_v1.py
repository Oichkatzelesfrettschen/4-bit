#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
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
        description="Apply angle-alignment seed suggestions to schematic_layout_anchors_v1.json (v1)."
    )
    ap.add_argument("--chip", required=True, choices=["4001", "4002", "4003", "4004"])
    ap.add_argument(
        "--suggestions",
        type=Path,
        default=None,
        help="Angle-alignment suggestions JSON (defaults under docs/evidence/anchor_seed_suggestions_v0/).",
    )
    ap.add_argument(
        "--anchors",
        type=Path,
        default=ROOT / "docs" / "evidence" / "schematic_layout_anchors_v1.json",
        help="Input anchors JSON (v1).",
    )
    ap.add_argument("--out", type=Path, default=None, help="Write updated anchors JSON here (defaults to overwrite).")
    ap.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite existing layout_node seeds even if set (default: only fill missing or clearly bad).",
    )
    ap.add_argument(
        "--skip-regex",
        default="",
        help="Regex of anchor names to ignore (e.g. '^D[0-3]_PAD$' to protect manual pad mappings).",
    )
    args = ap.parse_args()

    chip = str(args.chip).strip()
    sug_path = (
        (ROOT / args.suggestions).resolve()
        if args.suggestions and not args.suggestions.is_absolute()
        else (
            args.suggestions
            if args.suggestions
            else ROOT / "docs" / "evidence" / "anchor_seed_suggestions_v0" / f"{chip}_angle_alignment.json"
        )
    )
    anchors_path = (ROOT / args.anchors).resolve() if not args.anchors.is_absolute() else args.anchors
    out_path = (ROOT / args.out).resolve() if args.out and not args.out.is_absolute() else (args.out or anchors_path)

    anchors = _load(anchors_path)
    if not isinstance(anchors.get("anchors"), dict) or not isinstance(anchors["anchors"].get(chip), dict):
        raise SystemExit(f"anchors missing chip={chip}")
    block: dict[str, Any] = anchors["anchors"][chip]

    sug = _load(sug_path)
    matches = sug.get("alignment", {}).get("matches", [])
    if not isinstance(matches, list) or not matches:
        raise SystemExit(f"no matches in suggestions: {sug_path}")

    skip_re = re.compile(args.skip_regex) if args.skip_regex else None
    applied = 0
    skipped = 0
    missing = 0

    for m in matches:
        if not isinstance(m, dict):
            continue
        name = m.get("signal")
        node = m.get("node")
        if not isinstance(name, str) or not isinstance(node, int):
            continue
        if skip_re and skip_re.search(name):
            skipped += 1
            continue
        if name not in block or not isinstance(block.get(name), dict):
            missing += 1
            continue
        row: dict[str, Any] = block[name]

        cur = row.get("layout_node")
        if isinstance(cur, int) and not args.overwrite:
            # Heuristic: leave stable manual/derived anchors in place unless explicitly overwritten.
            continue

        if isinstance(cur, int) and cur != int(node):
            row.setdefault("layout_seed_history", [])
            hist = row["layout_seed_history"]
            if isinstance(hist, list):
                hist.append(
                    {
                        "kind": "prev_layout_node",
                        "layout_node": int(cur),
                        "source": "pre_apply_angle_alignment",
                    }
                )

        row["layout_node"] = int(node)
        row["layout_seed_v0"] = {
            "kind": "angle_alignment",
            "suggestions": str(sug_path.relative_to(ROOT)) if sug_path.is_relative_to(ROOT) else str(sug_path),
            "node": int(node),
            "node_bbox": m.get("node_bbox"),
            "cost_l1": sug.get("alignment", {}).get("cost_l1"),
            "offset": sug.get("alignment", {}).get("offset"),
        }
        applied += 1

    anchors.setdefault("notes", [])
    if isinstance(anchors["notes"], list):
        anchors["notes"].append(
            {
                "kind": "apply_angle_alignment_seeds_v1",
                "chip": chip,
                "suggestions": str(sug_path.relative_to(ROOT)) if sug_path.is_relative_to(ROOT) else str(sug_path),
                "applied": int(applied),
                "skipped": int(skipped),
                "missing": int(missing),
                "overwrite": bool(args.overwrite),
                "skip_regex": str(args.skip_regex),
            }
        )

    _write(out_path, anchors)
    print(json.dumps({"out": str(out_path), "chip": chip, "applied": applied, "skipped": skipped, "missing": missing}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

