#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def _load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


@dataclass(frozen=True)
class Anchor:
    name: str
    layout_node: int


def _anchors_from_netlist_v1(path: Path) -> list[Anchor]:
    data = _load_json(path)
    out: list[Anchor] = []
    for s in data.get("signals", []):
        if not isinstance(s, dict):
            continue
        if not s.get("evidence", {}).get("anchor"):
            continue
        name = str(s.get("name", "")).strip()
        node = s.get("layout_node")
        if not name or not isinstance(node, int):
            continue
        out.append(Anchor(name=name, layout_node=int(node)))
    return out


def _node_stats_index(netlist_v0: dict) -> dict[int, dict]:
    idx: dict[int, dict] = {}
    for ns in netlist_v0.get("node_stats", []):
        if isinstance(ns, dict) and isinstance(ns.get("node"), int):
            idx[int(ns["node"])] = ns
    return idx


def _transistor_incidence(netlist_v1: dict) -> dict[int, dict[str, int]]:
    """
    Count how many transistors touch each node (gate vs terminal endpoints).
    """
    counts: dict[int, dict[str, int]] = {}
    trans = netlist_v1.get("devices", {}).get("transistors", [])
    if not isinstance(trans, list):
        return counts
    for t in trans:
        if not isinstance(t, dict):
            continue
        for field, key in (("gate_node", "gate"), ("a_node", "terminal"), ("b_node", "terminal")):
            n = t.get(field)
            if not isinstance(n, int):
                continue
            row = counts.setdefault(int(n), {"gate": 0, "terminal": 0, "total": 0})
            row[key] += 1
            row["total"] += 1
    return counts


def main() -> int:
    p = argparse.ArgumentParser(description="Audit anchor node incidence vs extracted transistor devices (v0).")
    p.add_argument("--chip", default="4004", help="Chip number (4001/4002/4003/4004)")
    p.add_argument(
        "--netlist-v1",
        type=Path,
        default=ROOT / "docs" / "evidence" / "netlists_v1" / "4004_netlist_v1.json",
        help="netlist_v1 JSON to inspect",
    )
    p.add_argument(
        "--netlist-v0",
        type=Path,
        default=ROOT / "docs" / "evidence" / "netlists_v0" / "4004_netlist_v0.json",
        help="netlist_v0 JSON (for node_stats layer metrics)",
    )
    p.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "anchor_incidence_v0")
    args = p.parse_args()

    chip = str(args.chip).strip()
    netlist_v1_path = args.netlist_v1
    netlist_v0_path = args.netlist_v0
    if not netlist_v1_path.is_absolute():
        netlist_v1_path = (ROOT / netlist_v1_path).resolve()
    if not netlist_v0_path.is_absolute():
        netlist_v0_path = (ROOT / netlist_v0_path).resolve()

    v1 = _load_json(netlist_v1_path)
    v0 = _load_json(netlist_v0_path)

    anchors = _anchors_from_netlist_v1(netlist_v1_path)
    node_stats = _node_stats_index(v0)
    incidence = _transistor_incidence(v1)

    rows: list[dict[str, object]] = []
    for a in sorted(anchors, key=lambda x: x.name):
        ns = node_stats.get(a.layout_node, {})
        inc = incidence.get(a.layout_node, {"gate": 0, "terminal": 0, "total": 0})
        rows.append(
            {
                "name": a.name,
                "layout_node": int(a.layout_node),
                "incidence": inc,
                "node_stats": {
                    "metal_area": ns.get("metal_area"),
                    "poly_area": ns.get("poly_area"),
                    "diffusion_area": ns.get("diffusion_area"),
                    "gate_degree": ns.get("gate_degree"),
                    "terminal_degree": ns.get("terminal_degree"),
                    "metal_bbox": ns.get("metal_bbox"),
                    "poly_bbox": ns.get("poly_bbox"),
                    "diff_bbox": ns.get("diff_bbox"),
                },
            }
        )

    total = len(rows)
    zero_total = sum(1 for r in rows if int(r["incidence"]["total"]) == 0)  # type: ignore[index]
    payload = {
        "schema": {"version": 0, "description": "Anchor incidence audit vs netlist_v1 transistor devices."},
        "chip": chip,
        "inputs": {"netlist_v1": rel_or_abs(netlist_v1_path), "netlist_v0": rel_or_abs(netlist_v0_path)},
        "counts": {"anchors": int(total), "anchors_with_zero_transistors": int(zero_total)},
        "anchors": rows,
    }

    out_dir = Path(args.out_dir) / chip
    out_dir.mkdir(parents=True, exist_ok=True)
    out_json = out_dir / f"{chip}_anchor_incidence_v0.json"
    out_md = out_dir / f"{chip}_anchor_incidence_v0.md"
    out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    lines = [
        f"# Anchor Incidence v0 ({chip})",
        "",
        f"- Netlist v1: `{rel_or_abs(netlist_v1_path)}`",
        f"- Netlist v0: `{rel_or_abs(netlist_v0_path)}`",
        f"- Anchors: {total}",
        f"- Anchors with 0 incident transistors: {zero_total}",
        "",
        "| anchor | node | transistors | gate | terminal | metal_area | poly_area | diff_area |",
        "|---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for r in rows:
        inc = r["incidence"]  # type: ignore[assignment]
        ns = r["node_stats"]  # type: ignore[assignment]
        lines.append(
            "| {name} | {node} | {tot} | {g} | {t} | {ma} | {pa} | {da} |".format(
                name=str(r["name"]),
                node=int(r["layout_node"]),
                tot=int(inc["total"]),
                g=int(inc["gate"]),
                t=int(inc["terminal"]),
                ma=int(ns["metal_area"] or 0),
                pa=int(ns["poly_area"] or 0),
                da=int(ns["diffusion_area"] or 0),
            )
        )
    out_md.write_text("\n".join(lines) + "\n", encoding="utf-8")

    print(rel_or_abs(out_json))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

