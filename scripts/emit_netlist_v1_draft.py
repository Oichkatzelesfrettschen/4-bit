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


def main() -> int:
    p = argparse.ArgumentParser(description="Emit a minimal netlist_v1 draft by combining anchors + schematic connectivity (v0).")
    p.add_argument("--chip", required=True, choices=["4001", "4002", "4003", "4004"])
    p.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "netlists_v1")
    args = p.parse_args()

    chip = str(args.chip)
    anchors_path = ROOT / "docs" / "evidence" / "schematic_layout_anchors_v0.json"
    schem_names_path = ROOT / "docs" / "evidence" / "schematic_net_names_v0" / f"{chip.lower()}_schematic_net_names_v0.json"
    schem_conn_path = ROOT / "docs" / "evidence" / "schematic_connectivity_v0" / chip / f"{chip.lower()}_schematic_connectivity_v0.json"
    layout_path = ROOT / "docs" / "evidence" / "netlists_v0" / f"{chip.lower()}_netlist_v0.json"

    anchors = load_json(anchors_path)
    a = (
        anchors.get("anchors", {}).get(chip, {})  # type: ignore[call-arg]
        if isinstance(anchors, dict)
        else {}
    )
    if not isinstance(a, dict):
        raise SystemExit(f"anchors missing for chip={chip}")

    conn = load_json(schem_conn_path) if schem_conn_path.exists() else {}
    conn_by_name: dict[str, dict] = {}
    if isinstance(conn, dict):
        for t in conn.get("targets", []) if isinstance(conn.get("targets"), list) else []:
            if isinstance(t, dict) and isinstance(t.get("name"), str):
                conn_by_name[str(t["name"])] = t

    # Emit signals: anchored names only (v1 draft).
    signals: list[dict[str, object]] = []
    for name in sorted(a.keys()):
        row = a.get(name)
        if not isinstance(row, dict):
            continue
        layout_node = row.get("layout_node")
        note = row.get("note")
        out: dict[str, object] = {
            "name": name,
            "layout_node": layout_node,
            "match": {"kind": "manual_anchor_v0", "confidence": 1.0 if layout_node is not None else 0.0},
            "evidence": {"note": note, "anchor": True},
        }
        if name in conn_by_name:
            t = conn_by_name[name]
            out["schematic_connectivity_v0"] = {
                "seed": t.get("seed"),
                "counts": t.get("counts"),
                "bbox": t.get("bbox"),
                "hits": t.get("hits"),
            }
        signals.append(out)

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    out_json = out_dir / f"{chip.lower()}_netlist_v1.json"
    payload = {
        "chip": chip,
        "schema": {"version": 1, "description": "Draft schematic↔layout bridge (anchors + schematic connectivity traces)."},
        "inputs": {
            "anchors_v0": rel_or_abs(anchors_path),
            "schematic_net_names_v0": rel_or_abs(schem_names_path),
            "schematic_connectivity_v0": rel_or_abs(schem_conn_path) if schem_conn_path.exists() else None,
            "layout_netlist_v0": rel_or_abs(layout_path),
            "sha256": {
                "anchors_v0": sha256(anchors_path),
                "schematic_net_names_v0": sha256(schem_names_path) if schem_names_path.exists() else None,
                "schematic_connectivity_v0": sha256(schem_conn_path) if schem_conn_path.exists() else None,
                "layout_netlist_v0": sha256(layout_path),
            },
        },
        "counts": {"signals": int(len(signals))},
        "signals": signals,
        "devices": {"transistors": [], "loads": []},
    }
    out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"out_json": str(out_json)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
