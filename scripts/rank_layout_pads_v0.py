#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class ChipPaths:
    chip: str
    netlist_v0: Path


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def chip_paths(chip: str) -> ChipPaths:
    return ChipPaths(chip=chip, netlist_v0=ROOT / "docs" / "evidence" / "netlists_v0" / f"{chip.lower()}_netlist_v0.json")


def edge_distance(bb: dict[str, int], w: int, h: int) -> int:
    x0, y0, x1, y1 = int(bb["x0"]), int(bb["y0"]), int(bb["x1"]), int(bb["y1"])
    return int(min(x0, y0, w - x1, h - y1))


def main() -> int:
    p = argparse.ArgumentParser(description="Rank pad-like layout nodes from netlist_v0 node_stats.")
    p.add_argument("--chip", action="append", choices=["4001", "4002", "4003", "4004"], help="Chip (repeatable)")
    p.add_argument("--all", action="store_true", help="All supported chips")
    p.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "layout_pad_candidates_v0")
    p.add_argument("--top", type=int, default=40, help="How many candidates to emit per chip")
    p.add_argument("--edge-max", type=int, default=220, help="Max pixel distance from edge to consider 'periphery'")
    args = p.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = {"4001", "4002", "4003", "4004"}
    if not selected:
        p.error("select --all or at least one --chip")

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    manifest: dict[str, object] = {
        "tool": "scripts/rank_layout_pads_v0.py",
        "params": {"top": int(args.top), "edge_max": int(args.edge_max)},
        "outputs": [],
    }

    for chip in sorted(selected):
        paths = chip_paths(chip)
        obj = json.loads(paths.netlist_v0.read_text(encoding="utf-8"))
        shape = obj["inputs"]["layout_shape"]
        h = int(shape["h"])
        w = int(shape["w"])

        candidates = []
        for n in obj.get("node_stats", []):
            if not isinstance(n, dict):
                continue
            bb = n.get("metal_bbox")
            if not isinstance(bb, dict):
                continue
            dist = edge_distance(bb, w=w, h=h)
            if dist > int(args.edge_max):
                continue
            candidates.append(
                {
                    "node": int(n["node"]),
                    "metal_area": int(n.get("metal_area", 0)),
                    "terminal_degree": int(n.get("terminal_degree", 0)),
                    "gate_degree": int(n.get("gate_degree", 0)),
                    "edge_distance": int(dist),
                    "metal_bbox": bb,
                }
            )

        # Rank: big metal bbox at border, plus connectivity degree.
        ranked = sorted(
            candidates,
            key=lambda r: (
                -r["edge_distance"],
                r["metal_area"],
                r["terminal_degree"],
                r["gate_degree"],
                -r["node"],
            ),
            reverse=True,
        )[: int(args.top)]

        out_json = out_dir / f"{chip.lower()}_layout_pad_candidates_v0.json"
        payload = {
            "chip": chip,
            "schema": {"version": 0, "description": "Ranked pad-like layout nodes (border-touching metal) from netlist_v0."},
            "inputs": {"netlist_v0": rel_or_abs(paths.netlist_v0)},
            "params": manifest["params"],
            "counts": {"candidates_total": int(len(candidates)), "candidates_emitted": int(len(ranked))},
            "candidates": ranked,
        }
        out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        manifest["outputs"].append({"chip": chip, "output": rel_or_abs(out_json)})

        out_md = out_dir / f"{chip.lower()}_layout_pad_candidates_v0.md"
        lines = []
        lines.append(f"# Layout pad candidates v0 ({chip})\n")
        lines.append(f"- Source: `{payload['inputs']['netlist_v0']}`\n")
        lines.append("| Rank | Node | edge_distance | metal_area | terminal_degree | gate_degree | metal_bbox |")
        lines.append("|---:|---:|---:|---:|---:|---:|---|")
        for i, r in enumerate(ranked, start=1):
            bb = r["metal_bbox"]
            lines.append(
                f"| {i} | {r['node']} | {r['edge_distance']} | {r['metal_area']} | {r['terminal_degree']} | {r['gate_degree']} | ({bb['x0']},{bb['y0']})-({bb['x1']},{bb['y1']}) |"
            )
        lines.append("")
        out_md.write_text("\n".join(lines) + "\n", encoding="utf-8")

    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
