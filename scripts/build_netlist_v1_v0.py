#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def load_json(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def _get(obj: object, *keys: str) -> object:
    cur: object = obj
    for k in keys:
        if not isinstance(cur, dict):
            return None
        cur = cur.get(k)
    return cur


def _bbox_area(bb: object) -> int:
    if not isinstance(bb, dict):
        return 0
    w = int(bb.get("w", 0) or 0)
    h = int(bb.get("h", 0) or 0)
    return max(0, w) * max(0, h)


def main() -> int:
    p = argparse.ArgumentParser(description="Build netlist_v1 (anchors + schematic wiring + device candidates) (v0).")
    p.add_argument("--chip", required=True, choices=["4001", "4002", "4003", "4004"])
    p.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "netlists_v1")
    p.add_argument(
        "--max-transistor-bbox-area",
        type=int,
        default=500_000,
        help="Filter obviously broken transistor bboxes larger than this area (px^2).",
    )
    p.add_argument(
        "--max-transistor-bbox-dim",
        type=int,
        default=1200,
        help="Filter transistor bboxes with width/height larger than this many pixels.",
    )
    args = p.parse_args()

    chip = str(args.chip)
    anchors_path = ROOT / "docs" / "evidence" / "schematic_layout_anchors_v0.json"
    schem_names_path = ROOT / "docs" / "evidence" / "schematic_net_names_v0" / f"{chip.lower()}_schematic_net_names_v0.json"
    schem_wirenets_path = (
        ROOT / "docs" / "evidence" / "schematic_wirenets_v0" / chip / f"{chip.lower()}_schematic_wirenets_v0.json"
    )
    schem_conn_path = ROOT / "docs" / "evidence" / "schematic_connectivity_v0" / chip / f"{chip.lower()}_schematic_connectivity_v0.json"
    layout_path = ROOT / "docs" / "evidence" / "netlists_v0" / f"{chip.lower()}_netlist_v0.json"

    anchors = load_json(anchors_path)
    a = _get(anchors, "anchors", chip)
    if not isinstance(a, dict):
        raise SystemExit(f"anchors missing for chip={chip}")

    layout = load_json(layout_path)
    devices = _get(layout, "devices")
    trans = _get(layout, "devices", "transistors")
    if not isinstance(devices, dict) or not isinstance(trans, list):
        raise SystemExit(f"{layout_path}: missing devices.transistors")

    conn_by_name: dict[str, dict] = {}
    if schem_conn_path.exists():
        conn = load_json(schem_conn_path)
        if isinstance(conn, dict):
            for t in conn.get("targets", []) if isinstance(conn.get("targets"), list) else []:
                if isinstance(t, dict) and isinstance(t.get("name"), str):
                    conn_by_name[str(t["name"])] = t

    comp_by_name: dict[str, int] = {}
    if schem_wirenets_path.exists():
        wobj = load_json(schem_wirenets_path)
        if isinstance(wobj, dict):
            for r in wobj.get("points", []) if isinstance(wobj.get("points"), list) else []:
                if not isinstance(r, dict):
                    continue
                name = r.get("name")
                comp = r.get("schematic_component")
                if isinstance(name, str) and isinstance(comp, int):
                    comp_by_name[name] = int(comp)

    signals: list[dict[str, object]] = []
    anchor_nodes: set[int] = set()
    for name in sorted(a.keys()):
        row = a.get(name)
        if not isinstance(row, dict):
            continue
        layout_node = row.get("layout_node")
        note = row.get("note")
        match_conf = 1.0 if layout_node is not None else 0.0
        out: dict[str, object] = {
            "name": name,
            "layout_node": layout_node,
            "schematic_component": comp_by_name.get(name),
            "match": {"kind": "manual_anchor_v0", "confidence": match_conf},
            "evidence": {"note": note, "anchor": True},
        }
        if isinstance(layout_node, int):
            anchor_nodes.add(int(layout_node))
        if name in conn_by_name:
            t = conn_by_name[name]
            out["schematic_connectivity_v0"] = {
                "seed": t.get("seed"),
                "counts": t.get("counts"),
                "bbox": t.get("bbox"),
                "hits": t.get("hits"),
            }
        signals.append(out)

    # Normalize transistor candidates; filter obvious bbox catastrophes.
    max_area = int(args.max_transistor_bbox_area)
    max_dim = int(args.max_transistor_bbox_dim)
    kept: list[dict[str, object]] = []
    filtered: list[dict[str, object]] = []
    node_ids: set[int] = set(anchor_nodes)
    for t in trans:
        if not isinstance(t, dict):
            continue
        bb = t.get("bbox")
        area = _bbox_area(bb)
        w = int(bb.get("w", 0) or 0) if isinstance(bb, dict) else 0
        h = int(bb.get("h", 0) or 0) if isinstance(bb, dict) else 0
        if area > max_area or w > max_dim or h > max_dim:
            filtered.append({"kind": t.get("kind"), "bbox": bb, "reason": "bbox_too_large", "area": int(area)})
            continue
        tr = {
            "kind": t.get("kind"),
            "gate_node": int(t.get("gate_node", -1)),
            "a_node": int(t.get("a_node", -1)),
            "b_node": int(t.get("b_node", -1)),
            "bbox": bb,
        }
        kept.append(tr)
        for k in ("gate_node", "a_node", "b_node"):
            v = tr.get(k)
            if isinstance(v, int) and v >= 0:
                node_ids.add(int(v))

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    out_json = out_dir / f"{chip.lower()}_netlist_v1.json"

    payload = {
        "chip": chip,
        "schema": {"version": 1, "description": "Netlist v1 (v0 build): anchors + schematic wiring + device candidates."},
        "inputs": {
            "anchors_v0": rel_or_abs(anchors_path),
            "schematic_net_names_v0": rel_or_abs(schem_names_path),
            "schematic_wirenets_v0": rel_or_abs(schem_wirenets_path) if schem_wirenets_path.exists() else None,
            "schematic_connectivity_v0": rel_or_abs(schem_conn_path) if schem_conn_path.exists() else None,
            "layout_netlist_v0": rel_or_abs(layout_path),
            "sha256": {
                "anchors_v0": sha256(anchors_path),
                "schematic_net_names_v0": sha256(schem_names_path) if schem_names_path.exists() else None,
                "schematic_wirenets_v0": sha256(schem_wirenets_path) if schem_wirenets_path.exists() else None,
                "schematic_connectivity_v0": sha256(schem_conn_path) if schem_conn_path.exists() else None,
                "layout_netlist_v0": sha256(layout_path),
            },
        },
        "params": {"max_transistor_bbox_area": max_area, "max_transistor_bbox_dim": max_dim},
        "counts": {
            "signals": int(len(signals)),
            "transistors_total": int(len(trans)),
            "transistors_kept": int(len(kept)),
            "transistors_filtered": int(len(filtered)),
            "nodes_referenced": int(len(node_ids)),
        },
        "signals": signals,
        "nodes": [{"node": n} for n in sorted(node_ids)],
        "devices": {
            "transistors": kept,
            "filtered_transistors": filtered,
        },
    }
    out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    manifest = {
        "tool": "scripts/build_netlist_v1_v0.py",
        "params": payload["params"],
        "outputs": [{"chip": chip, "output": rel_or_abs(out_json)}],
    }
    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    print(json.dumps({"out_json": str(out_json)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

