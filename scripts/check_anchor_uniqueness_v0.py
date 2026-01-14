#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


def _load(path: Path) -> dict[str, Any]:
    obj = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(obj, dict):
        raise SystemExit(f"invalid json object: {path}")
    return obj


def _read_signals_txt(path: Path) -> list[str]:
    out: list[str] = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        out.append(line)
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description="Fail if required anchors share a layout_node (v0).")
    ap.add_argument("--anchors", type=Path, default=ROOT / "docs" / "evidence" / "schematic_layout_anchors_v1.json")
    ap.add_argument("--chip", choices=["4001", "4002", "4003", "4004"], action="append", default=[])
    ap.add_argument("--all", action="store_true", help="Check all chips.")
    ap.add_argument(
        "--allow-regex",
        default="",
        help="Optional regex of signal names allowed to share layout_node (applies per chip).",
    )
    args = ap.parse_args()

    chips = ["4001", "4002", "4003", "4004"] if args.all else [str(c) for c in args.chip]
    if not chips:
        raise SystemExit("select --all or at least one --chip")

    anchors_path = (ROOT / args.anchors).resolve() if not args.anchors.is_absolute() else args.anchors
    anchors = _load(anchors_path)
    aroot = anchors.get("anchors")
    if not isinstance(aroot, dict):
        raise SystemExit(f"anchors missing 'anchors' root: {anchors_path}")

    allow_re = re.compile(str(args.allow_regex)) if str(args.allow_regex).strip() else None

    failed = False
    for chip in chips:
        block = aroot.get(chip)
        if not isinstance(block, dict):
            raise SystemExit(f"anchors missing chip={chip}")

        signals_txt = ROOT / "docs" / "emulators" / f"i{chip}-signals.txt"
        required = _read_signals_txt(signals_txt) if signals_txt.exists() else sorted(block.keys())

        # Only consider required signals with a concrete layout_node.
        layout_by_node: dict[int, list[str]] = {}
        for sig in required:
            if sig not in block:
                continue
            row = block.get(sig)
            if not isinstance(row, dict):
                continue
            if allow_re and allow_re.search(sig):
                continue
            node = row.get("layout_node")
            if not isinstance(node, int):
                continue
            layout_by_node.setdefault(int(node), []).append(str(sig))

        dups = {n: sorted(v) for n, v in layout_by_node.items() if len(v) > 1}
        if dups:
            failed = True
            print(f"[dup] chip={chip} has duplicate layout_node assignments:")
            for node, sigs in sorted(dups.items(), key=lambda kv: (-(len(kv[1])), kv[0])):
                print(f"  node {node}: {', '.join(sigs)}")

    if failed:
        raise SystemExit(2)
    print("Anchor uniqueness check passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

