#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


@dataclass(frozen=True)
class Row:
    name: str
    nodes: int
    transistors: int
    output: str


def main() -> int:
    p = argparse.ArgumentParser(description="Summarize extracted subcircuits (v0).")
    p.add_argument("--manifest", type=Path, required=True, help="Subcircuits manifest.json")
    p.add_argument("--out-json", type=Path, required=True)
    p.add_argument("--out-md", type=Path, required=True)
    args = p.parse_args()

    mpath = args.manifest
    m = json.loads(mpath.read_text(encoding="utf-8"))
    outs = m.get("outputs", []) if isinstance(m, dict) else []
    if not isinstance(outs, list):
        raise SystemExit("manifest missing outputs[]")

    rows: list[Row] = []
    for o in outs:
        if not isinstance(o, dict):
            continue
        name = str(o.get("name", ""))
        counts = o.get("counts", {})
        if not isinstance(counts, dict):
            continue
        nodes = int(counts.get("nodes", 0) or 0)
        trans = int(counts.get("transistors", 0) or 0)
        out = str(o.get("output", ""))
        rows.append(Row(name=name, nodes=nodes, transistors=trans, output=out))

    rows_sorted = sorted(rows, key=lambda r: (-r.transistors, -r.nodes, r.name))
    zero = [r for r in rows_sorted if r.transistors == 0]
    nonzero = [r for r in rows_sorted if r.transistors > 0]

    payload = {
        "schema": {"version": 0, "description": "Subcircuit extraction metrics."},
        "inputs": {"manifest": rel_or_abs(mpath)},
        "counts": {
            "subcircuits": int(len(rows_sorted)),
            "nonzero_transistors": int(len(nonzero)),
            "zero_transistors": int(len(zero)),
        },
        "rows": [r.__dict__ for r in rows_sorted],
    }

    args.out_json.parent.mkdir(parents=True, exist_ok=True)
    args.out_md.parent.mkdir(parents=True, exist_ok=True)
    args.out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    md = []
    md.append("# Subcircuit Metrics (v0)\n")
    md.append(f"- Manifest: `{rel_or_abs(mpath)}`\n")
    md.append(f"- Subcircuits: {len(rows_sorted)} (nonzero: {len(nonzero)}, zero: {len(zero)})\n")
    md.append("\n## Summary\n")
    md.append("| Name | Nodes | Transistors | Output |\n")
    md.append("| --- | ---:| ---:| --- |\n")
    for r in rows_sorted:
        md.append(f"| `{r.name}` | {r.nodes} | {r.transistors} | `{r.output}` |\n")
    if zero:
        md.append("\n## Notes\n")
        md.append(
            "- Many pad/edge anchor nodes are currently metal-only and do not touch extracted transistor candidates; this indicates a stitching/extraction lacuna to fix (likely contact/via interpretation or mask alignment).\n"
        )
    args.out_md.write_text("".join(md), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

