#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class SummaryRow:
    path: str
    backend: str
    accuracy: float
    correct: int
    total: int


def main() -> int:
    ap = argparse.ArgumentParser(description="Summarize backend_bench*.json files (v0).")
    ap.add_argument(
        "--glob",
        default="docs/evidence/ocr_benchmarks_v0/backend_bench*.json",
        help="Glob of benchmark JSON files (default: docs/evidence/ocr_benchmarks_v0/backend_bench*.json).",
    )
    ap.add_argument(
        "--out-json",
        default="docs/evidence/ocr_benchmarks_v0/backend_bench_summary_v0.json",
        help="Write summary JSON here.",
    )
    ap.add_argument(
        "--out-tsv",
        default="docs/evidence/ocr_benchmarks_v0/backend_bench_summary_v0.tsv",
        help="Write summary TSV here.",
    )
    args = ap.parse_args()

    rows: list[SummaryRow] = []
    for p in sorted(ROOT.glob(str(args.glob))):
        try:
            data = json.loads(p.read_text(encoding="utf-8"))
        except Exception:
            continue
        results = data.get("results") or {}
        params = data.get("params") or {}
        backend = str(params.get("backend") or data.get("tool") or "unknown")
        rows.append(
            SummaryRow(
                path=str(p.relative_to(ROOT)),
                backend=backend,
                accuracy=float(results.get("accuracy") or 0.0),
                correct=int(results.get("correct") or 0),
                total=int(results.get("total") or 0),
            )
        )

    out_json = ROOT / args.out_json
    out_tsv = ROOT / args.out_tsv
    out_json.parent.mkdir(parents=True, exist_ok=True)

    out_json.write_text(
        json.dumps(
            {
                "schema": {"version": 0, "description": "Summary of OCR backend benchmarks (v0)."},
                "tool": "scripts/summarize_ocr_backend_benchmarks_v0.py",
                "rows": [r.__dict__ for r in rows],
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )

    out_tsv.write_text(
        "path\tbackend\taccuracy\tcorrect\ttotal\n"
        + "".join(f"{r.path}\t{r.backend}\t{r.accuracy:.6f}\t{r.correct}\t{r.total}\n" for r in rows),
        encoding="utf-8",
    )

    print(str(out_json.relative_to(ROOT)))
    print(str(out_tsv.relative_to(ROOT)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

