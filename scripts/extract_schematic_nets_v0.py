#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class ChipSpec:
    chip: str
    schematic_bmp: Path
    signals_txt: Path
    ocr_report_json: Path


def specs() -> dict[str, ChipSpec]:
    emu = ROOT / "docs" / "emulators"
    ocr = ROOT / "docs" / "evidence" / "ocr_signal_labels"
    return {
        "4001": ChipSpec("4001", emu / "i4001-schematic.bmp", emu / "i4001-signals.txt", ocr / "4001" / "4001_signal_ocr_report.json"),
        "4002": ChipSpec("4002", emu / "i4002-schematic.bmp", emu / "i4002-signals.txt", ocr / "4002" / "4002_signal_ocr_report.json"),
        "4003": ChipSpec("4003", emu / "i4003-schematic.bmp", emu / "i4003-signals.txt", ocr / "4003" / "4003_signal_ocr_report.json"),
        "4004": ChipSpec("4004", emu / "i4004-schematic.bmp", emu / "i4004-signals.txt", ocr / "4004" / "4004_signal_ocr_report.json"),
    }


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def rel_or_abs(path: Path) -> str:
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def parse_signals_txt(path: Path) -> list[dict[str, object]]:
    out: list[dict[str, object]] = []
    for raw in path.read_text(errors="replace").splitlines():
        line = raw.strip().strip("\r")
        if not line or line.startswith(";"):
            continue
        parts = [p.strip() for p in line.split(",")]
        if len(parts) != 3:
            continue
        try:
            x = int(parts[0])
            y = int(parts[1])
        except ValueError:
            continue
        out.append({"x": x, "y": y, "name": parts[2]})
    return out


def load_ocr_rows(path: Path) -> dict[int, dict[str, object]]:
    """
    OCR reports are optional inputs. When present, join by `idx`.
    """
    if not path.exists():
        return {}
    obj = json.loads(path.read_text(encoding="utf-8"))
    rows = obj.get("rows", [])
    if not isinstance(rows, list):
        return {}
    out: dict[int, dict[str, object]] = {}
    for r in rows:
        if not isinstance(r, dict):
            continue
        try:
            idx = int(r.get("idx"))
        except Exception:
            continue
        out[idx] = {
            "ok": bool(r.get("ok")),
            "reason": r.get("reason"),
            "expected": r.get("expected"),
            "ocr_raw": r.get("ocr_raw"),
            "ocr_norm": r.get("ocr_norm"),
            "score": r.get("score"),
        }
    return out


def main() -> int:
    p = argparse.ArgumentParser(
        description="Extract schematic net names from i400x signals.txt and join OCR verification (v0)."
    )
    p.add_argument("--chip", action="append", choices=sorted(specs().keys()), help="Chip to extract (repeatable)")
    p.add_argument("--all", action="store_true", help="Extract for all supported chips")
    p.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "schematic_net_names_v0")
    args = p.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = set(specs().keys())
    if not selected:
        p.error("select --all or at least one --chip")

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    manifest: dict[str, object] = {
        "tool": "scripts/extract_schematic_nets_v0.py",
        "params": {},
        "outputs": [],
    }

    for chip in sorted(selected):
        spec = specs()[chip]
        points = parse_signals_txt(spec.signals_txt)
        ocr_rows = load_ocr_rows(spec.ocr_report_json)

        nets: dict[str, list[int]] = defaultdict(list)
        point_rows: list[dict[str, object]] = []
        for idx, pt in enumerate(points):
            name = str(pt["name"])
            nets[name].append(idx)
            row = {"idx": idx, "x": int(pt["x"]), "y": int(pt["y"]), "name": name}
            if idx in ocr_rows:
                row["ocr"] = ocr_rows[idx]
            point_rows.append(row)

        out_json = out_dir / f"{chip.lower()}_schematic_net_names_v0.json"
        payload = {
            "chip": chip,
            "schema": {"version": 0, "description": "Schematic net names from signals.txt (schematic-space), with optional OCR join."},
            "inputs": {
                "schematic_bmp": rel_or_abs(spec.schematic_bmp),
                "signals_txt": rel_or_abs(spec.signals_txt),
                "ocr_report_json": rel_or_abs(spec.ocr_report_json) if spec.ocr_report_json.exists() else None,
                "sha256": {
                    "schematic_bmp": sha256(spec.schematic_bmp),
                    "signals_txt": sha256(spec.signals_txt),
                    "ocr_report_json": sha256(spec.ocr_report_json) if spec.ocr_report_json.exists() else None,
                },
            },
            "counts": {
                "signals_points": int(len(points)),
                "net_names": int(len(nets)),
                "points_with_ocr": int(sum(1 for r in point_rows if "ocr" in r)),
            },
            "nets": [{"name": k, "point_indices": v} for k, v in sorted(nets.items(), key=lambda kv: kv[0])],
            "points": point_rows,
        }
        out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        manifest["outputs"].append({"chip": chip, "output": rel_or_abs(out_json)})

    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

