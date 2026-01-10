#!/usr/bin/env python3
from __future__ import annotations

import json
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def run(cmd: list[str]) -> None:
    proc = subprocess.run(cmd, cwd=ROOT, check=False, text=True, capture_output=True)
    if proc.returncode != 0:
        sys.stderr.write(proc.stdout)
        sys.stderr.write(proc.stderr)
        raise SystemExit(proc.returncode)


def load_json(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    out_dir = ROOT / "target" / "ocr_smoke"
    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    # 1) Basic format/coordinate sanity on all signal maps.
    run([sys.executable, "scripts/verify_signals_txt.py", "--all", "--out", str(out_dir / "signals_txt_audit.json")])

    # 2) Fast OCR smoke test: 4004 CLK1/CLK2 are alias-mapped to printed 01/02.
    run(
        [
            sys.executable,
            "scripts/ocr_signal_labels.py",
            "--chip",
            "4004",
            "--labels-only",
            "--mode",
            "region",
            "--save-mismatches",
            "0",
            "--out-dir",
            str(out_dir / "labels"),
            "--name-regex",
            "^(CLK1|CLK2)$",
            "--limit",
            "0",
        ]
    )

    report = load_json(out_dir / "labels" / "4004" / "4004_signal_ocr_report.json")
    rows = report.get("rows", [])
    ok_by_name = {r.get("expected"): bool(r.get("ok")) for r in rows}
    if ok_by_name.get("CLK1") is not True or ok_by_name.get("CLK2") is not True:
        sys.stderr.write("expected CLK1/CLK2 to match in smoke OCR run\n")
        sys.stderr.write(json.dumps(rows, indent=2, sort_keys=True) + "\n")
        return 1

    print(f"ok: wrote smoke outputs under {out_dir.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

