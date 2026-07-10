#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    ap = argparse.ArgumentParser(description="Backfill layout_node_uid into schematic_layout_anchors_v0.json (v0).")
    ap.add_argument(
        "--anchors",
        type=Path,
        default=ROOT / "docs" / "evidence" / "schematic_layout_anchors_v0.json",
        help="Anchors JSON to update",
    )
    ap.add_argument(
        "--netlists-v0-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "netlists_v0",
        help="Directory containing i400x netlist_v0 JSONs",
    )
    ap.add_argument("--chip", action="append", choices=["4001", "4002", "4003", "4004"], help="Chip to update (repeatable)")
    ap.add_argument("--all", action="store_true", help="Update all chips present in anchors file")
    args = ap.parse_args()

    anchors_path = args.anchors
    if not anchors_path.is_absolute():
        anchors_path = (ROOT / anchors_path).resolve()
    data = _load(anchors_path)

    anchors_root = data.get("anchors")
    if not isinstance(anchors_root, dict):
        raise SystemExit("anchors json missing anchors{}")

    selected = set(args.chip or [])
    if args.all or not selected:
        selected = set(k for k in anchors_root.keys() if isinstance(k, str))

    for chip in sorted(selected):
        block = anchors_root.get(chip)
        if not isinstance(block, dict):
            continue
        net_path = Path(args.netlists_v0_dir) / f"{chip.lower()}_netlist_v0.json"
        if not net_path.is_absolute():
            net_path = (ROOT / net_path).resolve()
        if not net_path.exists():
            raise SystemExit(f"missing netlist_v0: {net_path}")
        net = _load(net_path)
        idx: dict[int, str] = {}
        for ns in net.get("node_stats", []):
            if not isinstance(ns, dict) or not isinstance(ns.get("node"), int):
                continue
            uid = ns.get("node_uid")
            if isinstance(uid, str) and uid:
                idx[int(ns["node"])] = uid

        for _name, row in block.items():
            if not isinstance(row, dict):
                continue
            n = row.get("layout_node")
            if not isinstance(n, int):
                continue
            uid = idx.get(int(n))
            if uid:
                row["layout_node_uid"] = uid

    anchors_path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(str(anchors_path))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

