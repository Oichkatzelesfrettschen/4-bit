#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
from collections import defaultdict, deque
from dataclasses import dataclass
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


@dataclass(frozen=True)
class Subgraph:
    seed_nodes: list[int]
    nodes: list[int]
    transistors: list[dict[str, object]]


def _load_netlist(path: Path) -> dict:
    obj = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(obj, dict):
        raise SystemExit(f"invalid json: {path}")
    return obj


def _signal_node_map(net: dict) -> dict[str, int]:
    out: dict[str, int] = {}
    for s in net.get("signals", []) if isinstance(net.get("signals"), list) else []:
        if not isinstance(s, dict):
            continue
        name = s.get("name")
        node = s.get("layout_node")
        if isinstance(name, str) and isinstance(node, int):
            out[name] = int(node)
    return out


def _incident_nodes(t: dict) -> list[int]:
    out: list[int] = []
    for k in ("gate_node", "a_node", "b_node"):
        v = t.get(k)
        if isinstance(v, int) and v >= 0:
            out.append(int(v))
    return out


def extract_subcircuit(
    *,
    net: dict,
    seed_nodes: list[int],
    radius: int,
    max_transistors: int,
    max_nodes: int,
) -> Subgraph:
    dev = net.get("devices", {})
    trans = dev.get("transistors", []) if isinstance(dev, dict) else []
    if not isinstance(trans, list):
        raise SystemExit("netlist missing devices.transistors[]")

    # Index: node -> transistor indices
    node_to_tr: dict[int, list[int]] = defaultdict(list)
    trs_norm: list[dict[str, object]] = []
    for idx, t in enumerate(trans):
        if not isinstance(t, dict):
            continue
        tt = {
            "id": int(idx),
            "kind": t.get("kind"),
            "gate_node": int(t.get("gate_node", -1)),
            "a_node": int(t.get("a_node", -1)),
            "b_node": int(t.get("b_node", -1)),
            "bbox": t.get("bbox"),
        }
        trs_norm.append(tt)
        for n in _incident_nodes(tt):
            node_to_tr[n].append(int(idx))

    seen_nodes: set[int] = set()
    seen_trs: set[int] = set()
    frontier: deque[int] = deque()
    for n in seed_nodes:
        if n >= 0:
            seen_nodes.add(int(n))
            frontier.append(int(n))

    # Layered expansion by hop count (node -> transistors -> nodes).
    for _hop in range(max(0, int(radius))):
        if not frontier:
            break
        next_frontier: deque[int] = deque()
        layer_nodes = list(frontier)
        frontier.clear()

        for n in layer_nodes:
            for tid in node_to_tr.get(n, []):
                if tid in seen_trs:
                    continue
                seen_trs.add(tid)
                if len(seen_trs) >= int(max_transistors):
                    break
                t = trs_norm[tid]
                for m in _incident_nodes(t):
                    if m not in seen_nodes:
                        seen_nodes.add(m)
                        if len(seen_nodes) >= int(max_nodes):
                            break
                        next_frontier.append(m)
            if len(seen_trs) >= int(max_transistors) or len(seen_nodes) >= int(max_nodes):
                break

        frontier = next_frontier
        if len(seen_trs) >= int(max_transistors) or len(seen_nodes) >= int(max_nodes):
            break

    nodes_sorted = sorted(seen_nodes)
    trs_sorted = [trs_norm[i] for i in sorted(seen_trs)]
    return Subgraph(seed_nodes=seed_nodes, nodes=nodes_sorted, transistors=trs_sorted)


def main() -> int:
    p = argparse.ArgumentParser(description="Extract a bounded transistor subcircuit around one or more seed nodes (v0).")
    p.add_argument("--netlist", type=Path, default=ROOT / "docs" / "evidence" / "netlists_v1" / "4004_netlist_v1.json")
    p.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "subcircuits_v0")
    p.add_argument("--chip", type=str, default=None, help="Chip id for output paths (defaults to netlist.chip)")
    p.add_argument("--signal", action="append", default=[], help="Signal name to use as a seed (repeatable)")
    p.add_argument("--node", action="append", default=[], help="Layout node id to use as a seed (repeatable)")
    p.add_argument("--all-signals", action="store_true", help="Extract subcircuits for every netlist_v1 signal with a layout_node")
    p.add_argument("--radius", type=int, default=3, help="Hop radius expansion in node space")
    p.add_argument("--max-transistors", type=int, default=4000, help="Safety bound on transistor count")
    p.add_argument("--max-nodes", type=int, default=12000, help="Safety bound on node count")
    args = p.parse_args()

    net_path = Path(args.netlist)
    net = _load_netlist(net_path)
    chip = str(args.chip or net.get("chip") or "unknown")

    sig_map = _signal_node_map(net)
    selected: list[tuple[str, list[int]]] = []
    if bool(args.all_signals):
        for name, node in sorted(sig_map.items(), key=lambda kv: kv[0]):
            selected.append((name, [int(node)]))
    else:
        combined: list[int] = []
        for s in args.signal:
            if s not in sig_map:
                raise SystemExit(f"unknown signal (no layout_node): {s}")
            node = int(sig_map[s])
            selected.append((str(s), [node]))
            combined.append(node)
        for n in args.node:
            node = int(n)
            selected.append((f"node_{node}", [node]))
            combined.append(node)
        if not combined:
            raise SystemExit("select --all-signals or provide --signal/--node seeds")
        if len(combined) > 1:
            selected.append(("custom", combined))

    out_dir = Path(args.out_dir) / chip
    out_dir.mkdir(parents=True, exist_ok=True)

    manifest: dict[str, object] = {
        "tool": "scripts/extract_subcircuit_v0.py",
        "inputs": {"netlist": rel_or_abs(net_path), "sha256": {"netlist": sha256(net_path)}},
        "params": {
            "radius": int(args.radius),
            "max_transistors": int(args.max_transistors),
            "max_nodes": int(args.max_nodes),
        },
        "outputs": [],
    }

    for name, seeds in selected:
        sg = extract_subcircuit(
            net=net,
            seed_nodes=seeds,
            radius=int(args.radius),
            max_transistors=int(args.max_transistors),
            max_nodes=int(args.max_nodes),
        )
        out = out_dir / f"{chip.lower()}_{name}_subcircuit_v0.json"
        payload = {
            "chip": chip,
            "schema": {"version": 0, "description": "Extracted transistor subcircuit around seed nodes."},
            "inputs": {"netlist_v1": rel_or_abs(net_path), "sha256": {"netlist_v1": sha256(net_path)}},
            "params": manifest["params"],
            "seed": {"kind": name, "nodes": sg.seed_nodes},
            "counts": {"nodes": int(len(sg.nodes)), "transistors": int(len(sg.transistors))},
            "nodes": [{"node": n} for n in sg.nodes],
            "devices": {"transistors": sg.transistors},
            # Parent netlist_v1 signals whose layout_node falls inside this
            # subgraph, carried through in the shared node-ID space so rail
            # and clock identification reads from evidence instead of being
            # inferred downstream.
            "signals": [
                {
                    "name": s["name"],
                    "layout_node": int(s["layout_node"]),
                    "evidence": {"anchor": bool((s.get("evidence") or {}).get("anchor", False))},
                }
                for s in sorted(
                    (
                        s
                        for s in (net.get("signals") or [])
                        if isinstance(s, dict)
                        and isinstance(s.get("name"), str)
                        and isinstance(s.get("layout_node"), int)
                        and int(s["layout_node"]) in set(sg.nodes)
                    ),
                    key=lambda s: (str(s["name"]), int(s["layout_node"])),
                )
            ],
        }
        out.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        manifest["outputs"].append({"name": name, "output": rel_or_abs(out), "counts": payload["counts"]})

    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
