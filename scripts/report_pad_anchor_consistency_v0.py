#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any, NamedTuple

ROOT = Path(__file__).resolve().parents[1]


def _load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _rel(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


class _PadMatch(NamedTuple):
    pad_idx: int
    method: str
    score: float


def _center(bb: dict[str, Any]) -> tuple[float, float]:
    return (0.5 * (float(bb["x0"]) + float(bb["x1"])), 0.5 * (float(bb["y0"]) + float(bb["y1"])))


def _contains(bb: dict[str, Any], x: float, y: float) -> bool:
    return float(bb["x0"]) <= x <= float(bb["x1"]) and float(bb["y0"]) <= y <= float(bb["y1"])


def _dist_to_rect(bb: dict[str, Any], x: float, y: float) -> float:
    x0, y0, x1, y1 = float(bb["x0"]), float(bb["y0"]), float(bb["x1"]), float(bb["y1"])
    dx = 0.0
    if x < x0:
        dx = x0 - x
    elif x > x1:
        dx = x - x1
    dy = 0.0
    if y < y0:
        dy = y0 - y
    elif y > y1:
        dy = y - y1
    return math.hypot(dx, dy)


def _index_node_stats(net_v0: dict[str, Any]) -> dict[int, dict[str, Any]]:
    out: dict[int, dict[str, Any]] = {}
    for ns in net_v0.get("node_stats", []):
        if isinstance(ns, dict) and isinstance(ns.get("node"), int):
            out[int(ns["node"])] = ns
    return out


def _best_bbox(ns: dict[str, Any]) -> dict[str, Any] | None:
    for k in ("metal_bbox", "poly_bbox", "diffusion_bbox"):
        bb = ns.get(k)
        if isinstance(bb, dict) and {"x0", "y0", "x1", "y1"} <= set(bb.keys()):
            return bb
    return None


def _match_pad_for_node(*, pads: list[dict[str, Any]], node: int, node_bb: dict[str, Any] | None) -> _PadMatch | None:
    # 1) Strong match: pad suggested_nodes includes the node. Use the highest score.
    best: _PadMatch | None = None
    for p in pads:
        if not isinstance(p, dict) or not isinstance(p.get("suggested_nodes"), list):
            continue
        for sn in p["suggested_nodes"]:
            if not isinstance(sn, dict) or sn.get("node") != node:
                continue
            pad_idx = int(p.get("idx_perimeter_ccw", p.get("idx_ccw", -1)))
            if pad_idx < 0:
                continue
            score = float(sn.get("score", 0.0) or 0.0)
            cand = _PadMatch(pad_idx=pad_idx, method="suggested_nodes", score=score)
            if best is None or cand.score > best.score:
                best = cand
    if best is not None:
        return best

    # 2) Fallback: bbox containment / distance using node bbox center.
    if not isinstance(node_bb, dict):
        return None
    cx, cy = _center(node_bb)
    contains: list[_PadMatch] = []
    dists: list[_PadMatch] = []
    for p in pads:
        bb = p.get("bbox")
        if not isinstance(bb, dict) or not {"x0", "y0", "x1", "y1"} <= set(bb.keys()):
            continue
        pad_idx = int(p.get("idx_perimeter_ccw", p.get("idx_ccw", -1)))
        if pad_idx < 0:
            continue
        if _contains(bb, cx, cy):
            contains.append(_PadMatch(pad_idx=pad_idx, method="bbox_contains", score=1.0))
        else:
            d = _dist_to_rect(bb, cx, cy)
            dists.append(_PadMatch(pad_idx=pad_idx, method="bbox_distance", score=-d))
    if contains:
        # Multiple can contain when boxes overlap; pick the smallest by area for determinism.
        def area_for_idx(pi: int) -> float:
            for p in pads:
                if int(p.get("idx_perimeter_ccw", -1)) == pi:
                    bb = p.get("bbox", {})
                    return float(bb.get("area", 0.0) or 0.0)
            return 0.0

        return sorted(contains, key=lambda m: (area_for_idx(m.pad_idx), m.pad_idx))[0]
    if dists:
        return sorted(dists, key=lambda m: (-m.score, m.pad_idx))[0]
    return None


def _write_md(
    *,
    out: Path,
    chip: str,
    inputs: dict[str, str],
    anchor_rows: list[dict[str, Any]],
    pad_rows: list[dict[str, Any]],
) -> None:
    lines: list[str] = []
    lines.append(f"# {chip} pad↔anchor consistency (v0)\n")
    lines.append("Inputs:")
    for k, v in inputs.items():
        lines.append(f"- {k}: `{v}`")
    lines.append("")

    lines.append("## Anchor→pad mapping\n")
    lines.append("| Anchor | layout_node_src | pad_idx_perimeter_ccw | method | score |")
    lines.append("|---|---:|---:|---|---:|")
    for r in anchor_rows:
        a = r["anchor"]
        src = r.get("layout_node_src")
        pad = r.get("pad_idx")
        method = r.get("method", "")
        score = r.get("score", "")
        lines.append(f"| `{a}` | {'' if src is None else int(src)} | {'' if pad is None else int(pad)} | {method} | {score} |")
    lines.append("")

    lines.append("## Pad→anchors (collisions)\n")
    lines.append("| pad_idx_perimeter_ccw | anchors |")
    lines.append("|---:|---|")
    for pr in pad_rows:
        lines.append(f"| {pr['pad_idx']} | {', '.join(f'`{a}`' for a in pr['anchors'])} |")
    lines.append("")

    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    ap = argparse.ArgumentParser(description="Report consistency between pad detections and anchor layout_node_src values (v0).")
    ap.add_argument("--chip", action="append", choices=["4001", "4002", "4003", "4004"], help="Chip (repeatable)")
    ap.add_argument("--all", action="store_true", help="All supported chips")
    ap.add_argument("--anchors", type=Path, default=ROOT / "docs/evidence/schematic_layout_anchors_v1.json")
    ap.add_argument("--netlists-v0", type=Path, default=ROOT / "docs/evidence/netlists_v0")
    ap.add_argument("--pads-v0", type=Path, default=ROOT / "docs/evidence/layout_pads_v0")
    ap.add_argument("--out-dir", type=Path, default=ROOT / "docs/evidence/pad_anchor_consistency_v0")
    args = ap.parse_args()

    chips = ["4001", "4002", "4003", "4004"] if args.all else (args.chip or [])
    if not chips:
        ap.error("select --all or at least one --chip")

    anchors = _load_json(args.anchors).get("anchors", {})
    out_dir = args.out_dir if args.out_dir.is_absolute() else (ROOT / args.out_dir).resolve()

    manifest: dict[str, Any] = {
        "tool": "scripts/report_pad_anchor_consistency_v0.py",
        "schema": {"version": 0},
        "inputs": {"anchors": _rel(args.anchors)},
        "outputs": [],
    }

    for chip in chips:
        net_path = args.netlists_v0 / f"{chip}_netlist_v0.json"
        pads_path = args.pads_v0 / chip / f"{chip}_layout_pads_v0.json"
        if not net_path.is_absolute():
            net_path = (ROOT / net_path).resolve()
        if not pads_path.is_absolute():
            pads_path = (ROOT / pads_path).resolve()
        net = _load_json(net_path)
        pads_obj = _load_json(pads_path)
        pads = pads_obj.get("pads", [])
        node_stats = _index_node_stats(net)

        anchor_map = anchors.get(chip, {})
        anchor_rows: list[dict[str, Any]] = []
        pad_to_anchors: dict[int, list[str]] = {}
        for name, rec in sorted(anchor_map.items()):
            if not isinstance(rec, dict):
                continue
            src = rec.get("layout_node_src")
            if not isinstance(src, int):
                continue
            ns = node_stats.get(int(src), {})
            bb = _best_bbox(ns) if isinstance(ns, dict) else None
            m = _match_pad_for_node(pads=pads, node=int(src), node_bb=bb)
            anchor_rows.append(
                {
                    "anchor": name,
                    "layout_node_src": int(src),
                    "pad_idx": None if m is None else int(m.pad_idx),
                    "method": "" if m is None else m.method,
                    "score": "" if m is None else round(float(m.score), 6),
                }
            )
            if m is not None:
                pad_to_anchors.setdefault(int(m.pad_idx), []).append(name)

        pad_rows = [{"pad_idx": i, "anchors": sorted(v)} for i, v in sorted(pad_to_anchors.items()) if len(v) > 1]

        md_out = out_dir / chip / f"{chip}_pad_anchor_consistency_v0.md"
        _write_md(
            out=md_out,
            chip=chip,
            inputs={
                "anchors": _rel(args.anchors),
                "netlist_v0": _rel(net_path),
                "layout_pads_v0": _rel(pads_path),
            },
            anchor_rows=anchor_rows,
            pad_rows=pad_rows,
        )
        manifest["outputs"].append(
            {
                "chip": chip,
                "counts": {
                    "anchors_with_layout_node_src": sum(1 for r in anchor_rows if r.get("layout_node_src") is not None),
                    "anchors_mapped_to_pad": sum(1 for r in anchor_rows if r.get("pad_idx") is not None),
                    "pad_collisions": len(pad_rows),
                },
                "outputs": {"report_md": _rel(md_out)},
            }
        )

    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(_rel(out_dir / "manifest.json"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

