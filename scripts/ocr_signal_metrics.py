#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class ChipMetrics:
    chip: str
    labels_total: int
    ok: int
    mismatch: int
    skipped: int
    ok_rate: float
    reasons: dict[str, int]
    top_mismatched_expected: list[dict[str, object]]


def load_json(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def compute_chip_metrics(report: dict[str, object]) -> ChipMetrics:
    chip = str(report.get("chip", ""))
    rows = report.get("rows", [])
    if not isinstance(rows, list):
        raise ValueError("report.rows must be a list")

    labels_total = 0
    ok = 0
    skipped = 0
    mismatch = 0

    reason_counts: Counter[str] = Counter()
    mismatch_expected: Counter[str] = Counter()

    for r in rows:
        if not isinstance(r, dict):
            continue
        expected_kind = str(r.get("expected_kind", ""))
        if expected_kind == "label":
            labels_total += 1

        reason = str(r.get("reason", ""))
        if reason:
            reason_counts[reason] += 1

        if reason == "skipped_expression":
            skipped += 1
            continue

        if bool(r.get("ok")):
            ok += 1
            continue

        mismatch += 1
        expected_norm = str(r.get("expected_norm", ""))
        if expected_norm:
            mismatch_expected[expected_norm] += 1

    ok_rate = (ok / labels_total) if labels_total else 0.0

    top_mismatched_expected = [
        {"expected_norm": name, "count": count}
        for name, count in mismatch_expected.most_common(25)
    ]

    return ChipMetrics(
        chip=chip,
        labels_total=labels_total,
        ok=ok,
        mismatch=mismatch,
        skipped=skipped,
        ok_rate=ok_rate,
        reasons=dict(reason_counts.most_common()),
        top_mismatched_expected=top_mismatched_expected,
    )


def metrics_to_markdown(metrics: list[ChipMetrics]) -> str:
    lines: list[str] = []
    lines.append("# OCR signal label verification metrics")
    lines.append("")
    lines.append("Generated from `docs/evidence/ocr_signal_labels/*/*_signal_ocr_report.json`.")
    lines.append("")
    lines.append("| Chip | Label points | OK | Mismatch | OK rate | Top mismatch reasons |")
    lines.append("|------|-------------:|---:|---------:|--------:|----------------------|")
    for m in metrics:
        top_reasons = []
        for reason, count in list(m.reasons.items())[:4]:
            if reason in ("matched", "skipped_expression"):
                continue
            top_reasons.append(f"{reason}:{count}")
        lines.append(
            f"| {m.chip} | {m.labels_total} | {m.ok} | {m.mismatch} | {m.ok_rate:.3f} | {'; '.join(top_reasons) or '-'} |"
        )
    lines.append("")
    lines.append("## Notes")
    lines.append("")
    lines.append("- High mismatch rates are expected because many `signals.txt` entries are net identifiers, while the schematic prints pin numbers or local labels near the sampled coordinate.")
    lines.append("- The `not_printed_near_point` reason is the working hypothesis when OCR finds confident nearby tokens but none resemble the expected net name.")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument(
        "--in-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "ocr_signal_labels",
        help="Directory containing per-chip OCR reports",
    )
    p.add_argument("--out-json", type=Path, default=None)
    p.add_argument("--out-md", type=Path, default=None)
    args = p.parse_args()

    in_dir = Path(args.in_dir)
    reports = list(in_dir.glob("*/**/*_signal_ocr_report.json"))

    reports = [p for p in reports if p.exists()]
    if not reports:
        raise SystemExit(f"no reports found under {in_dir}")

    chip_metrics: list[ChipMetrics] = []
    for report_path in sorted(reports):
        report = load_json(report_path)
        if not isinstance(report, dict):
            continue
        chip_metrics.append(compute_chip_metrics(report))

    out_json = Path(args.out_json) if args.out_json else (in_dir / "metrics.json")
    out_md = Path(args.out_md) if args.out_md else (in_dir / "metrics.md")

    def maybe_rel(path: Path) -> str:
        p = path.resolve()
        try:
            return str(p.relative_to(ROOT))
        except ValueError:
            return str(p)

    payload = {
        "tool": "scripts/ocr_signal_metrics.py",
        "inputs": {"reports": [maybe_rel(p) for p in reports]},
        "chips": [
            {
                "chip": m.chip,
                "labels_total": m.labels_total,
                "ok": m.ok,
                "mismatch": m.mismatch,
                "skipped": m.skipped,
                "ok_rate": m.ok_rate,
                "reasons": m.reasons,
                "top_mismatched_expected": m.top_mismatched_expected,
            }
            for m in chip_metrics
        ],
    }

    out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    out_md.write_text(metrics_to_markdown(chip_metrics), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
