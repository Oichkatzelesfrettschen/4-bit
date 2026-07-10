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
    layout_netlist: Path
    schematic_names: Path


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def chip_paths(chip: str) -> ChipPaths:
    return ChipPaths(
        chip=chip,
        layout_netlist=ROOT / "docs" / "evidence" / "netlists_v0" / f"{chip.lower()}_netlist_v0.json",
        schematic_names=ROOT / "docs" / "evidence" / "schematic_net_names_v0" / f"{chip.lower()}_schematic_net_names_v0.json",
    )

def load_pad_candidates(chip: str) -> list[dict[str, object]]:
    p = ROOT / "docs" / "evidence" / "layout_pad_candidates_v0" / f"{chip.lower()}_layout_pad_candidates_v0.json"
    if not p.exists():
        return []
    obj = json.loads(p.read_text(encoding="utf-8"))
    cand = obj.get("candidates", [])
    return cand if isinstance(cand, list) else []


def top_candidates(layout: dict, k: int) -> list[dict[str, int]]:
    stats = layout.get("node_stats", [])
    if not isinstance(stats, list):
        return []
    # Rank by terminal_degree then metal_area, then diffusion_area (deterministic).
    stats_sorted = sorted(
        stats,
        key=lambda n: (
            int(n.get("terminal_degree", 0)),
            int(n.get("metal_area", 0)),
            int(n.get("diffusion_area", 0)),
            -int(n.get("node", 0)),
        ),
        reverse=True,
    )
    out = []
    for n in stats_sorted[: int(k)]:
        out.append(
            {
                "node": int(n["node"]),
                "terminal_degree": int(n.get("terminal_degree", 0)),
                "gate_degree": int(n.get("gate_degree", 0)),
                "metal_area": int(n.get("metal_area", 0)),
                "diffusion_area": int(n.get("diffusion_area", 0)),
                "poly_area": int(n.get("poly_area", 0)),
            }
        )
    return out


def main() -> int:
    p = argparse.ArgumentParser(description="Create a v0 matching scaffold between schematic net names and layout nodes.")
    p.add_argument("--chip", action="append", choices=["4001", "4002", "4003", "4004"], help="Chip (repeatable)")
    p.add_argument("--all", action="store_true", help="All supported chips")
    p.add_argument("--anchors", type=Path, default=ROOT / "docs" / "evidence" / "schematic_layout_anchors_v0.json")
    p.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "schematic_layout_match_v0")
    p.add_argument("--candidates", type=int, default=25, help="How many top layout node candidates to list")
    args = p.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = {"4001", "4002", "4003", "4004"}
    if not selected:
        p.error("select --all or at least one --chip")

    anchors_obj = json.loads(Path(args.anchors).read_text(encoding="utf-8")) if Path(args.anchors).exists() else {}
    anchors = anchors_obj.get("anchors", {}) if isinstance(anchors_obj, dict) else {}

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    manifest: dict[str, object] = {
        "tool": "scripts/match_schematic_layout_v0.py",
        "inputs": {"anchors": rel_or_abs(Path(args.anchors))},
        "params": {"candidates": int(args.candidates)},
        "outputs": [],
    }

    for chip in sorted(selected):
        paths = chip_paths(chip)
        layout = json.loads(paths.layout_netlist.read_text(encoding="utf-8"))
        schem = json.loads(paths.schematic_names.read_text(encoding="utf-8"))

        chip_anchors = anchors.get(chip, {}) if isinstance(anchors, dict) else {}
        # Net names present in schematic.
        schem_names = {n["name"] for n in schem.get("nets", []) if isinstance(n, dict) and "name" in n}

        # Resolve anchors (if any).
        resolved = []
        unresolved = []
        for name, info in sorted(chip_anchors.items(), key=lambda kv: kv[0]):
            layout_node = None
            note = None
            if isinstance(info, dict):
                layout_node = info.get("layout_node")
                note = info.get("note")
            status = "resolved" if isinstance(layout_node, int) else "unresolved"
            row = {"name": name, "status": status, "layout_node": layout_node, "note": note, "present_in_schematic": name in schem_names}
            if status == "resolved":
                resolved.append(row)
            else:
                unresolved.append(row)

        top = top_candidates(layout, k=int(args.candidates))
        pad_like = load_pad_candidates(chip)[: int(args.candidates)]

        out_json = out_dir / f"{chip.lower()}_schematic_layout_match_v0.json"
        payload = {
            "chip": chip,
            "schema": {
                "version": 0,
                "description": "Scaffold report for matching schematic net names (signals.txt) to layout nodes (netlist_v0).",
            },
            "inputs": {
                "layout_netlist_v0": rel_or_abs(paths.layout_netlist),
                "schematic_net_names_v0": rel_or_abs(paths.schematic_names),
                "anchors": rel_or_abs(Path(args.anchors)),
            },
            "counts": {
                "anchors_total": int(len(chip_anchors)) if isinstance(chip_anchors, dict) else 0,
                "anchors_resolved": int(len(resolved)),
                "anchors_unresolved": int(len(unresolved)),
                "schematic_net_names": int(schem.get("counts", {}).get("net_names", 0)),
                "layout_nodes": int(layout.get("counts", {}).get("nodes", 0)),
            },
            "top_layout_node_candidates": top,
            "pad_like_layout_nodes": pad_like,
            "anchors_resolved": resolved,
            "anchors_unresolved": unresolved,
        }
        out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        manifest["outputs"].append({"chip": chip, "output": rel_or_abs(out_json)})

        # Markdown convenience
        out_md = out_dir / f"{chip.lower()}_schematic_layout_match_v0.md"
        lines = []
        lines.append(f"# Schematic↔Layout match v0 ({chip})\n")
        lines.append(f"- Layout: `{payload['inputs']['layout_netlist_v0']}`")
        lines.append(f"- Schematic names: `{payload['inputs']['schematic_net_names_v0']}`")
        lines.append(f"- Anchors: `{payload['inputs']['anchors']}`\n")
        lines.append("## Anchors\n")
        lines.append(f"- Resolved: `{payload['counts']['anchors_resolved']}` / `{payload['counts']['anchors_total']}`")
        lines.append(f"- Unresolved: `{payload['counts']['anchors_unresolved']}`\n")
        if unresolved:
            lines.append("### Unresolved anchors\n")
            for r in unresolved:
                lines.append(f"- `{r['name']}` (present_in_schematic={r['present_in_schematic']})")
            lines.append("")

        lines.append("## Candidate layout nodes (by terminal degree + metal area)\n")
        lines.append("| Node | terminal_degree | gate_degree | metal_area | diffusion_area | poly_area |")
        lines.append("|---:|---:|---:|---:|---:|---:|")
        for n in top:
            lines.append(
                f"| {n['node']} | {n['terminal_degree']} | {n['gate_degree']} | {n['metal_area']} | {n['diffusion_area']} | {n['poly_area']} |"
            )
        lines.append("")

        if pad_like:
            lines.append("## Pad-like nodes (periphery metal, ranked)\n")
            lines.append("Source: `docs/evidence/layout_pad_candidates_v0/`\n")
            lines.append("| Node | edge_distance | metal_area | terminal_degree | gate_degree | metal_bbox |")
            lines.append("|---:|---:|---:|---:|---:|---|")
            for r in pad_like:
                bb = r.get("metal_bbox") or {}
                lines.append(
                    f"| {int(r.get('node', 0))} | {int(r.get('edge_distance', 0))} | {int(r.get('metal_area', 0))} | {int(r.get('terminal_degree', 0))} | {int(r.get('gate_degree', 0))} | ({bb.get('x0')},{bb.get('y0')})-({bb.get('x1')},{bb.get('y1')}) |"
                )
            lines.append("")
        out_md.write_text("\n".join(lines) + "\n", encoding="utf-8")

    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
