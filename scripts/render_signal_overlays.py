#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class ChipSpec:
    chip: str
    schematic_bmp: Path
    signals_txt: Path


def specs() -> dict[str, ChipSpec]:
    def emu(p: str) -> Path:
        return ROOT / "docs" / "emulators" / p

    return {
        "4001": ChipSpec("4001", emu("i4001-schematic.bmp"), emu("i4001-signals.txt")),
        "4002": ChipSpec("4002", emu("i4002-schematic.bmp"), emu("i4002-signals.txt")),
        "4003": ChipSpec("4003", emu("i4003-schematic.bmp"), emu("i4003-signals.txt")),
        "4004": ChipSpec("4004", emu("i4004-schematic.bmp"), emu("i4004-signals.txt")),
    }


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


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


def load_report(path: Path) -> dict[str, object]:
    return json.loads(path.read_text(encoding="utf-8"))


def best_effort_font(size: int) -> ImageFont.ImageFont:
    # Keep deterministic: use default bitmap font if truetype unavailable.
    try:
        return ImageFont.truetype("DejaVuSansMono.ttf", size=size)
    except Exception:
        return ImageFont.load_default()


def draw_cross(draw: ImageDraw.ImageDraw, *, x: int, y: int, r: int, color: tuple[int, int, int]) -> None:
    draw.line((x - r, y, x + r, y), fill=color, width=2)
    draw.line((x, y - r, x, y + r), fill=color, width=2)


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--chip", action="append", default=[], choices=sorted(specs().keys()))
    p.add_argument("--all", action="store_true")
    p.add_argument(
        "--report-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "ocr_signal_labels",
        help="Where per-chip *_signal_ocr_report.json lives",
    )
    p.add_argument(
        "--out-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "ocr_signal_labels",
        help="Base output directory (per-chip overlays go under <chip>/overlays/)",
    )
    p.add_argument(
        "--mode",
        choices=["mismatch", "ok", "all"],
        default="mismatch",
        help="Which points to render (requires report for mismatch/ok)",
    )
    p.add_argument("--limit", type=int, default=250, help="Max annotated points to render (0 = all)")
    p.add_argument("--font-size", type=int, default=18)
    p.add_argument("--label-offset-x", type=int, default=12)
    p.add_argument("--label-offset-y", type=int, default=-18)
    args = p.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = set(specs().keys())
    if not selected:
        p.error("select --all or at least one --chip")

    for chip in sorted(selected):
        spec = specs()[chip]
        schematic = Image.open(spec.schematic_bmp).convert("RGB")
        draw = ImageDraw.Draw(schematic)
        font = best_effort_font(args.font_size)

        report_path = args.report_dir / chip / f"{chip}_signal_ocr_report.json"
        report = load_report(report_path) if report_path.exists() else None

        points = parse_signals_txt(spec.signals_txt)
        to_render: list[dict[str, object]] = []

        if args.mode == "all":
            to_render = points
        else:
            if not isinstance(report, dict):
                raise SystemExit(f"{args.mode} mode requires report: {report_path}")
            rows = report.get("rows", [])
            if not isinstance(rows, list):
                raise SystemExit(f"invalid report format: {report_path}")
            for r in rows:
                if not isinstance(r, dict):
                    continue
                ok = bool(r.get("ok"))
                if args.mode == "ok" and not ok:
                    continue
                if args.mode == "mismatch" and ok:
                    continue
                to_render.append(r)

        if args.limit and args.limit > 0:
            to_render = to_render[: args.limit]

        for r in to_render:
            x = int(r.get("x", 0))
            y = int(r.get("y", 0))
            ok = bool(r.get("ok"))
            expected = str(r.get("expected", r.get("name", "")))
            ocr = str(r.get("ocr_norm", "")).strip()
            reason = str(r.get("reason", "")).strip()
            label = expected
            if report is not None and args.mode != "all":
                suffix = f"  [{reason}]"
                if ocr:
                    label = f"{expected} -> {ocr}{suffix}"
                else:
                    label = f"{expected}{suffix}"

            color = (0, 160, 0) if ok else (200, 0, 0)
            draw_cross(draw, x=x, y=y, r=10, color=color)
            draw.text((x + args.label_offset_x, y + args.label_offset_y), label, fill=color, font=font)

        overlays_dir = args.out_dir / chip / "overlays"
        overlays_dir.mkdir(parents=True, exist_ok=True)
        out_png = overlays_dir / f"{chip}_signal_points_{args.mode}.png"
        out_meta = overlays_dir / f"{chip}_signal_points_{args.mode}.meta.json"

        schematic.save(out_png)
        meta = {
            "tool": "scripts/render_signal_overlays.py",
            "chip": chip,
            "mode": args.mode,
            "limit": args.limit,
            "inputs": {
                "schematic_bmp": str(spec.schematic_bmp.relative_to(ROOT)),
                "signals_txt": str(spec.signals_txt.relative_to(ROOT)),
                "schematic_sha256": sha256(spec.schematic_bmp),
                "signals_sha256": sha256(spec.signals_txt),
                "report_json": str(report_path.relative_to(ROOT)) if report_path.exists() else None,
                "report_sha256": sha256(report_path) if report_path.exists() else None,
            },
            "outputs": {"overlay_png": str(out_png.relative_to(ROOT))},
        }
        out_meta.write_text(json.dumps(meta, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

