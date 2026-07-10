#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def main() -> int:
    p = argparse.ArgumentParser(description="Summarize schematic_net_names_v0 outputs into JSON and Markdown tables.")
    p.add_argument("--in-dir", type=Path, default=ROOT / "docs" / "evidence" / "schematic_net_names_v0")
    p.add_argument("--out-json", type=Path, default=None)
    p.add_argument("--out-md", type=Path, default=None)
    args = p.parse_args()

    in_dir = Path(args.in_dir)
    paths = sorted(in_dir.glob("*_schematic_net_names_v0.json"))
    if not paths:
        raise SystemExit(f"no schematic_net_names_v0 JSON found under {in_dir}")

    rows = []
    for pth in paths:
        obj = json.loads(pth.read_text(encoding="utf-8"))
        counts = obj["counts"]
        ocr = Counter()
        for pt in obj.get("points", []):
            r = pt.get("ocr")
            if isinstance(r, dict):
                ocr[str(r.get("reason"))] += 1
        rows.append(
            {
                "chip": str(obj["chip"]),
                "path": rel_or_abs(pth),
                "bytes": int(pth.stat().st_size),
                "signals_points": int(counts["signals_points"]),
                "net_names": int(counts["net_names"]),
                "points_with_ocr": int(counts["points_with_ocr"]),
                "ocr_reason_top": ocr.most_common(5),
            }
        )

    out = {"tool": "scripts/schematic_net_names_v0_metrics.py", "in_dir": rel_or_abs(in_dir), "rows": rows}

    if args.out_json:
        Path(args.out_json).write_text(json.dumps(out, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if args.out_md:
        lines = []
        lines.append("# schematic_net_names_v0 metrics\n")
        lines.append(f"- Input dir: `{out['in_dir']}`\n")
        lines.append("")
        lines.append("| Chip | JSON (KiB) | Signals.txt points | Net names | OCR rows | Top OCR reasons |")
        lines.append("|---:|---:|---:|---:|---:|---|")
        for r in sorted(rows, key=lambda x: x["chip"]):
            kib = r["bytes"] / 1024.0
            top = ", ".join([f"{k}:{v}" for k, v in r["ocr_reason_top"]]) if r["ocr_reason_top"] else ""
            lines.append(
                f"| {r['chip']} | {kib:.1f} | {r['signals_points']} | {r['net_names']} | {r['points_with_ocr']} | {top} |"
            )
        lines.append("")
        Path(args.out_md).write_text("\n".join(lines) + "\n", encoding="utf-8")

    print(json.dumps(out, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
