#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def load_json(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def parse_analyzer_4004_counts(readme: str) -> dict[str, int] | None:
    """
    Best-effort parse of i400x_analyzer readme component table for 4004.

    Example (spacing varies):
      transistor:          1807  1807-66= 1741
      capacitor:            66  66-66=    0
    """
    def find_expr(label: str) -> tuple[int, int, int, int] | None:
        m = re.search(
            rf"{re.escape(label)}:\s+(\d+)\s+(\d+)\s*-\s*(\d+)\s*=\s*(\d+)",
            readme,
            flags=re.IGNORECASE,
        )
        if not m:
            return None
        return (int(m.group(1)), int(m.group(2)), int(m.group(3)), int(m.group(4)))

    trans = find_expr("transistor")
    if not trans:
        return None

    res = re.search(r"resistor:\s+(\d+)\s+(\d+)", readme, flags=re.IGNORECASE)
    cap = find_expr("capacitor")
    prot = re.search(r"input\s*\(gate\)\s*protector:\s+(\d+)\s+(\d+)", readme, flags=re.IGNORECASE)

    layout_t, schematic_total_t, schematic_minus_t, schematic_effective_t = trans
    out = {
        "layout_transistors": layout_t,
        "schematic_transistors_total": schematic_total_t,
        "schematic_transistors_minus_caps": schematic_minus_t,
        "schematic_transistors_effective": schematic_effective_t,
    }
    if res:
        out["layout_resistors"] = int(res.group(1))
        out["schematic_resistors"] = int(res.group(2))
    if cap:
        layout_c, schematic_total_c, schematic_minus_c, schematic_effective_c = cap
        out["layout_capacitors"] = layout_c
        out["schematic_capacitors_total"] = schematic_total_c
        out["schematic_capacitors_minus_caps"] = schematic_minus_c
        out["schematic_capacitors_effective"] = schematic_effective_c
    if prot:
        out["layout_gate_protectors"] = int(prot.group(1))
        out["schematic_gate_protectors"] = int(prot.group(2))
    return out


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument(
        "--in-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "transistors",
        help="Directory containing *_poly_diffusion_transistors.json files",
    )
    p.add_argument("--out-json", type=Path, default=None)
    p.add_argument("--out-md", type=Path, default=None)
    args = p.parse_args()

    in_dir = Path(args.in_dir)
    manifest_path = in_dir / "manifest.json"

    analyzer_ref = None
    analyzer_readme = ROOT / "docs" / "emulators" / "readme.txt"
    if analyzer_readme.exists():
        analyzer_ref = parse_analyzer_4004_counts(analyzer_readme.read_text(errors="replace"))

    outputs: list[Path] = []
    if manifest_path.exists():
        manifest = load_json(manifest_path)
        items = manifest.get("outputs", []) if isinstance(manifest, dict) else []
        if isinstance(items, list):
            for item in items:
                if not isinstance(item, dict):
                    continue
                rel = item.get("output")
                if isinstance(rel, str) and rel:
                    outputs.append(ROOT / rel)
    else:
        outputs = list(in_dir.glob("*_poly_diffusion_transistors.json"))

    outputs = [p for p in outputs if p.exists()]
    if not outputs:
        raise SystemExit(f"no transistor outputs found under {in_dir}")

    chips: list[dict[str, object]] = []
    for path in sorted(outputs):
        obj = load_json(path)
        if not isinstance(obj, dict):
            continue
        chip = str(obj.get("chip", ""))
        counts = obj.get("counts", {})
        if not isinstance(counts, dict):
            counts = {}
        row: dict[str, object] = {
            "chip": chip,
            "components_total": int(counts.get("components_total", 0)),
            "components_kept": int(counts.get("components_kept", 0)),
            "source": str(path.relative_to(ROOT)),
        }
        if chip == "4004" and isinstance(analyzer_ref, dict):
            row["analyzer_reference"] = analyzer_ref
            row["delta_vs_analyzer_layout"] = int(row["components_total"]) - int(analyzer_ref["layout_transistors"])
        chips.append(row)

    out_json = Path(args.out_json) if args.out_json else (in_dir / "metrics.json")
    out_md = Path(args.out_md) if args.out_md else (in_dir / "metrics.md")

    payload = {
        "tool": "scripts/transistor_metrics.py",
        "inputs": {"outputs": [c["source"] for c in chips]},
        "analyzer_reference_4004": analyzer_ref,
        "chips": chips,
    }
    out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    lines: list[str] = []
    lines.append("# Transistor candidate extraction metrics")
    lines.append("")
    lines.append("Generated from `docs/evidence/transistors/*_poly_diffusion_transistors.json`.")
    lines.append("")
    lines.append("| Chip | Poly∩Diffusion components | Kept (area>=min) |")
    lines.append("|------|--------------------------:|----------------:|")
    for c in chips:
        extra = ""
        if c.get("chip") == "4004" and isinstance(c.get("delta_vs_analyzer_layout"), int):
            extra = f" (Δ vs analyzer: {c['delta_vs_analyzer_layout']:+d})"
        lines.append(f"| {c['chip']} | {c['components_total']}{extra} | {c['components_kept']} |")
    lines.append("")
    lines.append("## Notes")
    lines.append("")
    lines.append("- This is a best-effort *candidate* list from poly/diffusion intersections, not a transistor-accurate netlist.")
    lines.append("- Counts depend on mask-layer thresholding and connected-component settings; see `docs/evidence/transistors/manifest.json`.")
    if isinstance(analyzer_ref, dict):
        lines.append("- The i400x analyzer readme reports 4004 layout transistors as "
                     f"`{analyzer_ref['layout_transistors']}` and schematic-effective transistors as "
                     f"`{analyzer_ref['schematic_transistors_effective']}` (excluding bootstrap-capacitor artifacts).")
        if "layout_resistors" in analyzer_ref and "layout_capacitors" in analyzer_ref:
            lines.append(
                "- The same table also reports passive components (not extracted here): "
                f"resistors `{analyzer_ref.get('layout_resistors')}` and capacitors `{analyzer_ref.get('layout_capacitors')}`."
            )
    lines.append("")
    out_md.write_text("\n".join(lines), encoding="utf-8")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
