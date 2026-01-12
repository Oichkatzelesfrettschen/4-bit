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


def specs() -> dict[str, Path]:
    ev = ROOT / "docs" / "evidence" / "netlists_v0"
    return {
        "4001": ev / "4001_netlist_v0.json",
        "4002": ev / "4002_netlist_v0.json",
        "4003": ev / "4003_netlist_v0.json",
        "4004": ev / "4004_netlist_v0.json",
    }


def main() -> int:
    p = argparse.ArgumentParser(description="Extract a normalized device graph from netlist_v0 (v0).")
    p.add_argument("--chip", action="append", choices=sorted(specs().keys()), help="Chip to extract (repeatable)")
    p.add_argument("--all", action="store_true", help="Extract for all supported chips")
    p.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "device_graph_v0")
    args = p.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = set(specs().keys())
    if not selected:
        p.error("select --all or at least one --chip")

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    manifest: dict[str, object] = {"tool": "scripts/extract_device_graph_v0.py", "params": {}, "outputs": []}

    for chip in sorted(selected):
        netlist_path = specs()[chip]
        obj = json.loads(netlist_path.read_text(encoding="utf-8"))
        if not isinstance(obj, dict):
            raise SystemExit(f"invalid json: {netlist_path}")

        devices = obj.get("devices") if isinstance(obj.get("devices"), dict) else {}
        trans = devices.get("transistors") if isinstance(devices, dict) and isinstance(devices.get("transistors"), list) else []
        node_stats = obj.get("node_stats") if isinstance(obj.get("node_stats"), list) else []

        nodes: list[dict[str, object]] = []
        for n in node_stats:
            if not isinstance(n, dict) or "node" not in n:
                continue
            nodes.append(
                {
                    "node": int(n["node"]),
                    "degree": int(n.get("degree", 0)),
                    "metal_area": int(n.get("metal_area", 0)),
                    "metal_bbox": n.get("metal_bbox"),
                }
            )

        transistors: list[dict[str, object]] = []
        for t in trans:
            if not isinstance(t, dict):
                continue
            transistors.append(
                {
                    "kind": t.get("kind"),
                    "gate_node": int(t.get("gate_node", -1)),
                    "a_node": int(t.get("a_node", -1)),
                    "b_node": int(t.get("b_node", -1)),
                    "bbox": t.get("bbox"),
                }
            )

        out_chip = out_dir / chip
        out_chip.mkdir(parents=True, exist_ok=True)
        out_json = out_chip / f"{chip.lower()}_device_graph_v0.json"
        payload = {
            "chip": chip,
            "schema": {"version": 0, "description": "Normalized device graph derived from netlist_v0 devices + node stats."},
            "inputs": {"netlist_v0": rel_or_abs(netlist_path), "sha256": {"netlist_v0": sha256(netlist_path)}},
            "counts": {"nodes": int(len(nodes)), "transistors": int(len(transistors))},
            "nodes": nodes,
            "devices": {"transistors": transistors},
        }
        out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        manifest["outputs"].append({"chip": chip, "output": rel_or_abs(out_json)})

    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

