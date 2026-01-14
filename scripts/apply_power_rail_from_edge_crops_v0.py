#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any, Iterable

ROOT = Path(__file__).resolve().parents[1]


def _load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _write(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


_CROP_RE = re.compile(r"^\d+_(?P<tok>[A-Za-z0-9]+)_node(?P<node>\d+)_conf(?P<conf>[0-9.]+)\.png$")


def _iter_edge_crops(edge_crops_dir: Path) -> Iterable[tuple[str, int, float, str]]:
    if not edge_crops_dir.exists():
        return []
    out: list[tuple[str, int, float, str]] = []
    for p in sorted(edge_crops_dir.glob("*.png")):
        m = _CROP_RE.match(p.name)
        if not m:
            continue
        tok = str(m.group("tok"))
        node = int(m.group("node"))
        conf = float(m.group("conf"))
        out.append((tok, node, conf, p.name))
    return out


def _best_by_token(rows: Iterable[tuple[str, int, float, str]]) -> dict[str, dict[str, Any]]:
    best: dict[str, dict[str, Any]] = {}
    for tok, node, conf, fname in rows:
        cur = best.get(tok)
        if cur is None or float(conf) > float(cur["conf"]):
            best[tok] = {"token": tok, "node": int(node), "conf": float(conf), "source_crop": str(fname)}
    return best


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Apply power-rail layout_node selections based on edge-label crop filenames (v0)."
    )
    ap.add_argument(
        "--anchors",
        type=Path,
        default=ROOT / "docs" / "evidence" / "schematic_layout_anchors_v1.json",
        help="Anchors JSON (v1).",
    )
    ap.add_argument(
        "--edge-labels-root",
        type=Path,
        default=ROOT / "docs" / "evidence" / "layout_edge_labels_v0",
        help="Root directory containing <chip>/crops/*.png edge-label crops.",
    )
    ap.add_argument("--chip", default="4004", help="Chip number (4001/4002/4003/4004)")
    ap.add_argument(
        "--min-conf",
        type=float,
        default=70.0,
        help="Minimum crop confidence to accept as evidence for rails.",
    )
    ap.add_argument("--dry-run", action="store_true", help="Print proposed updates without writing.")
    args = ap.parse_args()

    anchors_path = (ROOT / args.anchors).resolve() if not args.anchors.is_absolute() else args.anchors
    edge_root = (ROOT / args.edge_labels_root).resolve() if not args.edge_labels_root.is_absolute() else args.edge_labels_root
    chip = str(args.chip).strip()

    payload = _load(anchors_path)
    aroot = payload.get("anchors")
    if not isinstance(aroot, dict) or not isinstance(aroot.get(chip), dict):
        raise SystemExit(f"anchors missing chip={chip}")
    block: dict[str, Any] = aroot[chip]

    crops_dir = edge_root / chip / "crops"
    best = _best_by_token(_iter_edge_crops(crops_dir))

    updates: list[dict[str, Any]] = []

    # Token conventions observed in crops:
    # - 'G' indicates ground (VSS)
    # - 'V' indicates VDD/VCC depending on chip naming
    # These are the only ones we apply automatically.
    g = best.get("G")
    v = best.get("V")
    if g and float(g["conf"]) >= float(args.min_conf) and "VSS" in block:
        row = block.get("VSS")
        if isinstance(row, dict):
            prev = row.get("layout_node")
            row["layout_node"] = int(g["node"])
            row["layout_node_src"] = int(g["node"])
            row["rail_evidence_v0"] = {
                "kind": "edge_crop_token",
                "token": "G",
                "min_conf": float(args.min_conf),
                **g,
            }
            updates.append({"signal": "VSS", "prev": prev, "next": int(g["node"]), "evidence": g})

    if v and float(v["conf"]) >= float(args.min_conf):
        want = "VCC" if "VCC" in block else ("VDD" if "VDD" in block else None)
        if want:
            row = block.get(want)
            if isinstance(row, dict):
                prev = row.get("layout_node")
                row["layout_node"] = int(v["node"])
                row["layout_node_src"] = int(v["node"])
                row["rail_evidence_v0"] = {
                    "kind": "edge_crop_token",
                    "token": "V",
                    "min_conf": float(args.min_conf),
                    **v,
                }
                updates.append({"signal": want, "prev": prev, "next": int(v["node"]), "evidence": v})

    if updates:
        payload["notes"] = list(payload.get("notes") or [])
        if isinstance(payload["notes"], list):
            payload["notes"].append(
                {
                    "kind": "apply_power_rail_from_edge_crops_v0",
                    "chip": chip,
                    "edge_crops_dir": str(crops_dir.relative_to(ROOT)) if crops_dir.is_relative_to(ROOT) else str(crops_dir),
                    "min_conf": float(args.min_conf),
                    "updates": updates,
                }
            )

    if args.dry_run:
        print(json.dumps({"chip": chip, "updates": updates}, indent=2))
        return 0

    _write(anchors_path, payload)
    print(json.dumps({"out": str(anchors_path), "chip": chip, "updates": updates}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

