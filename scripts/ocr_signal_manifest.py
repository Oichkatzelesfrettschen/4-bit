#!/usr/bin/env python3
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def load_json(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def tesseract_version() -> str | None:
    try:
        # PATH lookup of the tesseract binary is the probe itself; FileNotFoundError
        # below reports the tool as absent.
        proc = subprocess.run(  # noqa: S603
            ["tesseract", "--version"],  # noqa: S607
            check=False,
            cwd=ROOT,
            text=True,
            capture_output=True,
        )
    except FileNotFoundError:
        return None
    if proc.returncode != 0:
        return None
    first = (proc.stdout or "").splitlines()
    if not first:
        return None
    # Example: "tesseract 5.5.2"
    parts = first[0].strip().split()
    if len(parts) >= 2 and parts[0].lower() == "tesseract":
        return parts[1]
    return first[0].strip() or None


def main() -> int:
    in_dir = ROOT / "docs" / "evidence" / "ocr_signal_labels"
    manifest_path = in_dir / "manifest.json"

    manifest: dict[str, object] = {"tool": "scripts/ocr_signal_labels.py", "outputs": []}
    if manifest_path.exists():
        raw = load_json(manifest_path)
        if isinstance(raw, dict):
            # Preserve run params if present; outputs are rebuilt from current reports.
            for k in ("tool", "params", "tesseract_version"):
                if k in raw:
                    manifest[k] = raw[k]

    outputs: list[dict[str, object]] = []
    for chip_dir in sorted(p for p in in_dir.iterdir() if p.is_dir()):
        chip = chip_dir.name
        report = chip_dir / f"{chip}_signal_ocr_report.json"
        tsv = chip_dir / f"{chip}_signal_ocr_report.tsv"
        if not report.exists():
            continue

        data = load_json(report)
        counts = data.get("counts") if isinstance(data, dict) else None

        outputs.append(
            {
                "chip": chip,
                "counts": counts if isinstance(counts, dict) else {},
                "crops_dir": str((chip_dir / "crops").relative_to(ROOT)),
                "report_json": str(report.relative_to(ROOT)),
                "report_tsv": str(tsv.relative_to(ROOT)) if tsv.exists() else "",
            }
        )

    if not outputs:
        sys.stderr.write(f"no per-chip reports found under {in_dir}\n")
        return 1

    manifest["outputs"] = outputs
    tess = tesseract_version()
    if tess:
        manifest["tesseract_version"] = tess

    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

