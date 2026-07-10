#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class Row:
    chip: str
    path: Path
    bytes: int
    nodes: int
    metal: int
    poly: int
    diffusion: int
    vias: int
    contacts: int
    transistors_kept: int
    transistors_ambiguous: int
    signals_points: int


def load_row(path: Path) -> Row:
    obj = json.loads(path.read_text(encoding="utf-8"))
    counts = obj["counts"]
    comps = counts["components"]
    stitches = counts["stitches"]
    return Row(
        chip=str(obj["chip"]),
        path=path,
        bytes=path.stat().st_size,
        nodes=int(counts["nodes"]),
        metal=int(comps["metal"]),
        poly=int(comps["poly"]),
        diffusion=int(comps["diffusion"]),
        vias=int(stitches["vias"]),
        contacts=int(stitches["contacts"]),
        transistors_kept=int(counts["transistors_kept"]),
        transistors_ambiguous=int(counts["transistors_ambiguous"]),
        signals_points=int(counts["signals_points"]),
    )


def main() -> int:
    p = argparse.ArgumentParser(description="Summarize netlist_v0 outputs into JSON and Markdown tables.")
    p.add_argument("--in-dir", type=Path, default=ROOT / "docs" / "evidence" / "netlists_v0")
    p.add_argument("--out-json", type=Path, default=None)
    p.add_argument("--out-md", type=Path, default=None)
    args = p.parse_args()

    in_dir = Path(args.in_dir)
    paths = sorted(in_dir.glob("*_netlist_v0.json"))
    if not paths:
        raise SystemExit(f"no netlist_v0 JSON found under {in_dir}")

    rows = [load_row(pth) for pth in paths]
    rows.sort(key=lambda r: r.chip)

    out = {
        "tool": "scripts/netlist_v0_metrics.py",
        "in_dir": str(in_dir.relative_to(ROOT) if in_dir.is_relative_to(ROOT) else in_dir),
        "rows": [
            {
                "chip": r.chip,
                "path": str(r.path.relative_to(ROOT) if r.path.is_relative_to(ROOT) else r.path),
                "bytes": r.bytes,
                "nodes": r.nodes,
                "components": {"metal": r.metal, "poly": r.poly, "diffusion": r.diffusion},
                "stitches": {"vias": r.vias, "contacts": r.contacts},
                "transistors": {"kept": r.transistors_kept, "ambiguous": r.transistors_ambiguous},
                "signals_points": r.signals_points,
            }
            for r in rows
        ],
    }

    if args.out_json:
        Path(args.out_json).write_text(json.dumps(out, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if args.out_md:
        lines = []
        lines.append("# netlist_v0 metrics\n")
        lines.append(f"- Input dir: `{out['in_dir']}`\n")
        lines.append("")
        lines.append("| Chip | JSON (KiB) | Nodes | Metal CC | Poly CC | Diff CC | Via stitches | Contact stitches | Tx kept | Tx ambiguous | Signal ref pts |")
        lines.append("|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
        for r in rows:
            kib = r.bytes / 1024.0
            lines.append(
                f"| {r.chip} | {kib:.1f} | {r.nodes} | {r.metal} | {r.poly} | {r.diffusion} | {r.vias} | {r.contacts} | {r.transistors_kept} | {r.transistors_ambiguous} | {r.signals_points} |"
            )
        lines.append("")
        Path(args.out_md).write_text("\n".join(lines) + "\n", encoding="utf-8")

    print(json.dumps(out, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

