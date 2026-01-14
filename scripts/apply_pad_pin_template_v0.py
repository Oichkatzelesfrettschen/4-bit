#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


def _load(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as f:
        data = json.load(f)
    if not isinstance(data, dict):
        raise SystemExit(f"expected JSON object: {path}")
    return data


def _write(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as f:
        json.dump(payload, f, indent=2, sort_keys=True)
        f.write("\n")


def _parse_pad_pin_template(md_text: str) -> list[dict[str, Any]]:
    """
    Parse the markdown table in pad_pin_template_v0.md.

    Expected columns:
      pad_idx | edge | bbox (...) | suggested_node | edge_label | pin_dip | signal | confidence | notes
    """
    rows: list[dict[str, Any]] = []
    in_table = False
    for raw in md_text.splitlines():
        line = raw.strip()
        if not line:
            if in_table and rows:
                # End of first table.
                break
            continue
        if line.startswith("| pad_idx |"):
            in_table = True
            continue
        if not in_table:
            continue
        if line.startswith("|---"):
            continue
        if not (line.startswith("|") and line.endswith("|")):
            continue
        parts = [p.strip() for p in line.strip("|").split("|")]
        if len(parts) < 9:
            continue
        pad_idx_s, _edge, _bbox_s, suggested_node_s, _edge_label, pin_dip_s, signal, confidence_s, notes = parts[:9]
        if not pad_idx_s or not suggested_node_s:
            continue
        try:
            pad_idx = int(pad_idx_s)
            suggested_node = int(suggested_node_s)
        except ValueError:
            continue
        pin_dip = None
        if pin_dip_s:
            try:
                pin_dip = int(pin_dip_s)
            except ValueError:
                pin_dip = None
        confidence = None
        if confidence_s:
            try:
                confidence = float(confidence_s)
            except ValueError:
                confidence = None
        rows.append(
            {
                "pad_idx": pad_idx,
                "suggested_node": suggested_node,
                "pin_dip": pin_dip,
                "signal": signal.strip(),
                "confidence": confidence,
                "notes": notes.strip(),
            }
        )
    return rows


def main() -> int:
    ap = argparse.ArgumentParser(description="Apply pad→pin template seeds to anchors (v0).")
    ap.add_argument("--chip", required=True, choices=["4001", "4002", "4003", "4004"])
    ap.add_argument(
        "--template",
        type=Path,
        default=None,
        help="pad_pin_template_v0.md path (defaults under docs/evidence/layout_pad_labels_v0/<chip>/)",
    )
    ap.add_argument(
        "--anchors",
        type=Path,
        default=ROOT / "docs" / "evidence" / "schematic_layout_anchors_v1.json",
        help="Input anchors JSON (usually schematic_layout_anchors_v1.json).",
    )
    ap.add_argument("--out", type=Path, default=None, help="Write updated anchors JSON here (default overwrite).")
    ap.add_argument(
        "--only",
        default="",
        help="Optional regex to restrict which signals are applied (matches anchor names).",
    )
    ap.add_argument(
        "--strict",
        action="store_true",
        help="Fail if the template references an anchor missing from the anchors JSON.",
    )
    args = ap.parse_args()

    chip = str(args.chip)
    template_path = (
        (ROOT / args.template).resolve()
        if args.template and not args.template.is_absolute()
        else (
            args.template
            if args.template
            else ROOT / "docs" / "evidence" / "layout_pad_labels_v0" / chip / "pad_pin_template_v0.md"
        )
    )
    anchors_path = (ROOT / args.anchors).resolve() if not args.anchors.is_absolute() else args.anchors
    out_path = (ROOT / args.out).resolve() if args.out and not args.out.is_absolute() else (args.out or anchors_path)

    anchors = _load(anchors_path)
    aroot = anchors.get("anchors")
    if not isinstance(aroot, dict) or not isinstance(aroot.get(chip), dict):
        raise SystemExit(f"anchors missing chip={chip}")
    block: dict[str, Any] = aroot[chip]

    only_re = re.compile(str(args.only)) if str(args.only).strip() else None

    tpl_text = template_path.read_text(encoding="utf-8")
    tpl_rows = _parse_pad_pin_template(tpl_text)
    if not tpl_rows:
        raise SystemExit(f"no template rows parsed: {template_path}")

    applied = 0
    skipped = 0
    missing = 0

    for r in tpl_rows:
        sig = str(r.get("signal") or "").strip()
        if not sig:
            continue
        if only_re and not only_re.search(sig):
            skipped += 1
            continue
        if sig not in block:
            missing += 1
            if args.strict:
                raise SystemExit(f"template references missing anchor {chip}.{sig}")
            continue
        row = block[sig]
        if not isinstance(row, dict):
            continue
        seed_node = int(r["suggested_node"])

        prev = row.get("layout_node")
        if isinstance(prev, int) and prev != seed_node:
            row.setdefault("layout_seed_history", [])
            hist = row["layout_seed_history"]
            if isinstance(hist, list):
                hist.append({"kind": "prev_layout_node", "layout_node": int(prev), "source": "pre_apply_pad_pin_template"})

        row["layout_node"] = seed_node
        row.pop("layout_node_src", None)
        row.pop("layout_node_uid", None)
        row.pop("remap_v1", None)
        row["layout_seed_v0"] = {
            "kind": "pad_pin_template_v0",
            "template": str(template_path.relative_to(ROOT)) if template_path.is_relative_to(ROOT) else str(template_path),
            "pad_idx": int(r["pad_idx"]),
            "pin_dip": r.get("pin_dip"),
            "confidence": r.get("confidence"),
            "notes": r.get("notes"),
            "seed_node": seed_node,
        }
        applied += 1

    anchors.setdefault("notes", [])
    if isinstance(anchors["notes"], list):
        anchors["notes"].append(
            {
                "kind": "apply_pad_pin_template_v0",
                "chip": chip,
                "template": str(template_path.relative_to(ROOT)) if template_path.is_relative_to(ROOT) else str(template_path),
                "applied": int(applied),
                "skipped": int(skipped),
                "missing": int(missing),
                "only": str(args.only),
                "strict": bool(args.strict),
            }
        )

    _write(out_path, anchors)
    print(json.dumps({"out": str(out_path), "chip": chip, "applied": applied, "skipped": skipped, "missing": missing}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

