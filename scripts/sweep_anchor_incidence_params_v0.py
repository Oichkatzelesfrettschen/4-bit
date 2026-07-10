#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _bbox_iou(a: dict, b: dict) -> float:
    ax0, ay0, ax1, ay1 = int(a["x0"]), int(a["y0"]), int(a["x1"]), int(a["y1"])
    bx0, by0, bx1, by1 = int(b["x0"]), int(b["y0"]), int(b["x1"]), int(b["y1"])
    ix0, iy0 = max(ax0, bx0), max(ay0, by0)
    ix1, iy1 = min(ax1, bx1), min(ay1, by1)
    iw, ih = max(0, ix1 - ix0), max(0, iy1 - iy0)
    inter = iw * ih
    if inter <= 0:
        return 0.0
    a_area = max(0, ax1 - ax0) * max(0, ay1 - ay0)
    b_area = max(0, bx1 - bx0) * max(0, by1 - by0)
    denom = a_area + b_area - inter
    return float(inter) / float(denom) if denom > 0 else 0.0


@dataclass(frozen=True)
class Anchor:
    name: str
    src_node: int
    metal_bbox: dict


def _anchors_from_sources(anchors_path: Path, src_netlist_v0: dict, chip: str) -> list[Anchor]:
    anchors = _load(anchors_path)
    block = anchors.get("anchors", {}).get(chip)
    if not isinstance(block, dict):
        raise SystemExit(f"anchors missing chip={chip}")
    src_stats = {int(ns["node"]): ns for ns in src_netlist_v0.get("node_stats", []) if isinstance(ns, dict) and isinstance(ns.get("node"), int)}
    out: list[Anchor] = []
    for name, row in sorted(block.items(), key=lambda kv: kv[0]):
        if not isinstance(row, dict):
            continue
        n = row.get("layout_node")
        if not isinstance(n, int):
            continue
        ns = src_stats.get(int(n))
        mb = ns.get("metal_bbox") if isinstance(ns, dict) else None
        if not isinstance(mb, dict):
            continue
        out.append(Anchor(name=str(name), src_node=int(n), metal_bbox=mb))
    return out


def _incidence_counts(netlist_v0: dict) -> dict[int, int]:
    counts: dict[int, int] = {}
    trans = netlist_v0.get("devices", {}).get("transistors", [])
    if not isinstance(trans, list):
        return counts
    for t in trans:
        if not isinstance(t, dict):
            continue
        for k in ("gate_node", "a_node", "b_node"):
            n = t.get(k)
            if isinstance(n, int):
                counts[n] = counts.get(n, 0) + 1
    return counts


def _dst_metal_nodes(netlist_v0: dict) -> list[tuple[int, dict]]:
    out = []
    for ns in netlist_v0.get("node_stats", []):
        if not isinstance(ns, dict) or not isinstance(ns.get("node"), int):
            continue
        mb = ns.get("metal_bbox")
        if isinstance(mb, dict):
            out.append((int(ns["node"]), mb))
    return out


def _remap_anchor(anchor: Anchor, dst_nodes: list[tuple[int, dict]]) -> tuple[int | None, float]:
    best_node = None
    best_iou = 0.0
    for n, bb in dst_nodes:
        iou = _bbox_iou(anchor.metal_bbox, bb)
        if iou > best_iou:
            best_iou = iou
            best_node = n
    return best_node, best_iou


def _run_extract(
    out_dir: Path,
    *,
    chip: str,
    dilate: int,
    stitch_policy: str,
    close: int,
    diffusion_split: bool,
) -> Path:
    out_dir.mkdir(parents=True, exist_ok=True)
    cmd = [
        "python3",
        str(ROOT / "scripts" / "extract_netlist_v0.py"),
        "--chip",
        chip,
        "--out-dir",
        str(out_dir),
        "--dilate",
        str(int(dilate)),
        "--close",
        str(int(close)),
        "--stitch-policy",
        str(stitch_policy),
    ]
    cmd.append("--diffusion-split" if diffusion_split else "--no-diffusion-split")
    # Fixed argv list running the sibling extract script via sys.executable; no shell.
    p = subprocess.run(cmd, check=False, text=True, capture_output=True)  # noqa: S603
    if p.returncode != 0:
        raise SystemExit(f"extract failed ({p.returncode}): {' '.join(cmd)}\n{p.stderr}")
    return out_dir / f"{chip.lower()}_netlist_v0.json"


def main() -> int:
    ap = argparse.ArgumentParser(description="Sweep stitch parameters and measure anchor incident transistor counts (v0).")
    ap.add_argument("--chip", default="4004")
    ap.add_argument("--anchors", type=Path, default=ROOT / "docs" / "evidence" / "schematic_layout_anchors_v0.json")
    ap.add_argument("--src-netlist-v0", type=Path, default=ROOT / "docs" / "evidence" / "netlists_v0" / "4004_netlist_v0.json")
    ap.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "anchor_sweeps_v0")
    ap.add_argument("--min-iou", type=float, default=0.20)
    ap.add_argument("--dilates", default="0,1,2,3,5")
    ap.add_argument("--close", type=int, default=0, help="Pass --close N to extract_netlist_v0.py during the sweep.")
    ap.add_argument(
        "--diffusion-split",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Pass --diffusion-split/--no-diffusion-split to extract_netlist_v0.py during the sweep.",
    )
    args = ap.parse_args()

    chip = str(args.chip).strip()
    anchors_path = (ROOT / args.anchors).resolve() if not args.anchors.is_absolute() else args.anchors
    src_path = (ROOT / args.src_netlist_v0).resolve() if not args.src_netlist_v0.is_absolute() else args.src_netlist_v0
    src = _load(src_path)
    anchors = _anchors_from_sources(anchors_path, src, chip)

    out_root = Path(args.out_dir) / chip
    out_root.mkdir(parents=True, exist_ok=True)

    dilates = [int(x.strip()) for x in str(args.dilates).split(",") if x.strip()]
    policies = ["strict", "relaxed"]

    rows: list[dict[str, object]] = []
    for pol in policies:
        for d in dilates:
            run_dir = out_root / f"{pol}_dilate{d}"
            net_path = _run_extract(
                run_dir,
                chip=chip,
                dilate=d,
                stitch_policy=pol,
                close=int(args.close),
                diffusion_split=bool(args.diffusion_split),
            )
            dst = _load(net_path)
            dst_nodes = _dst_metal_nodes(dst)
            inc = _incidence_counts(dst)

            mapped = 0
            incident = 0
            ious: list[float] = []
            per_anchor: list[dict[str, object]] = []
            for a in anchors:
                dst_node, iou = _remap_anchor(a, dst_nodes)
                if dst_node is None or iou < float(args.min_iou):
                    per_anchor.append({"name": a.name, "src_node": a.src_node, "dst_node": None, "iou": float(iou), "incidence": 0})
                    continue
                mapped += 1
                ious.append(float(iou))
                cnt = int(inc.get(int(dst_node), 0))
                if cnt > 0:
                    incident += 1
                per_anchor.append({"name": a.name, "src_node": a.src_node, "dst_node": int(dst_node), "iou": float(iou), "incidence": cnt})

            rows.append(
                {
                    "policy": pol,
                    "dilate": int(d),
                    "anchors_total": len(anchors),
                    "anchors_mapped": int(mapped),
                    "anchors_incident": int(incident),
                    "avg_iou": float(sum(ious) / len(ious)) if ious else 0.0,
                    "netlist_v0": str(net_path.relative_to(ROOT)) if net_path.is_relative_to(ROOT) else str(net_path),
                    "per_anchor": per_anchor,
                }
            )

    out_json = out_root / f"{chip}_anchor_sweep_v0.json"
    out_json.write_text(json.dumps({"chip": chip, "rows": rows}, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    # Emit a concise table for quick scanning.
    lines = [
        f"# Anchor Sweep v0 ({chip})",
        "",
        f"- Source netlist_v0: `{src_path}`",
        f"- Anchors: {len(anchors)} (mapped by metal_bbox IoU >= {float(args.min_iou):.2f})",
        "",
        "| policy | dilate | mapped | incident | avg_iou |",
        "|---|---:|---:|---:|---:|",
    ]
    for r in rows:
        lines.append(
            "| {policy} | {dilate} | {mapped} | {incident} | {avg_iou:.2f} |".format(
                policy=r["policy"],
                dilate=int(r["dilate"]),
                mapped=int(r["anchors_mapped"]),
                incident=int(r["anchors_incident"]),
                avg_iou=float(r["avg_iou"]),
            )
        )
    (out_root / f"{chip}_anchor_sweep_v0.md").write_text("\n".join(lines) + "\n", encoding="utf-8")

    print(str(out_json))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
