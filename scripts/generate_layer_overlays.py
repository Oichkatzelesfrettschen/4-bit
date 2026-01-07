#!/usr/bin/env python3
from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont


@dataclass(frozen=True)
class ChipSpec:
    chip: str
    schematic_bmp: Path
    signals_txt: Path
    poly_bmp: Path
    diffusion_bmp: Path
    out_labels_png: Path
    out_transistors_png: Path
    label_signals: set[str]


ROOT = Path(__file__).resolve().parents[1]


def load_mask(path: Path, threshold: int = 128) -> np.ndarray:
    img = Image.open(path).convert("L")
    arr = np.asarray(img)
    return arr > threshold


def save_poly_diff_overlay(poly: np.ndarray, diffusion: np.ndarray, out_path: Path) -> None:
    if poly.shape != diffusion.shape:
        raise ValueError(f"shape mismatch: poly={poly.shape} diffusion={diffusion.shape}")

    rgb = np.zeros((poly.shape[0], poly.shape[1], 3), dtype=np.uint8)
    rgb[poly, 0] = 255
    rgb[diffusion, 1] = 255
    # intersections become yellow (R+G)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(rgb, mode="RGB").save(out_path)


def parse_signals(path: Path) -> list[tuple[int, int, str]]:
    out: list[tuple[int, int, str]] = []
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
        name = parts[2]
        out.append((x, y, name))
    return out


def save_schematic_labels(
    schematic_bmp: Path,
    signals_txt: Path,
    label_signals: set[str],
    out_path: Path,
) -> None:
    img = Image.open(schematic_bmp).convert("RGB")
    draw = ImageDraw.Draw(img)
    font = ImageFont.load_default()

    for x, y, name in parse_signals(signals_txt):
        if name not in label_signals:
            continue
        if not (0 <= x < img.width and 0 <= y < img.height):
            continue

        r = 6
        draw.ellipse((x - r, y - r, x + r, y + r), outline=(255, 0, 0), width=2)
        draw.text((x + r + 2, y - r - 2), name, fill=(255, 255, 0), font=font)

    out_path.parent.mkdir(parents=True, exist_ok=True)
    img.save(out_path)


def specs() -> dict[str, ChipSpec]:
    def emu(p: str) -> Path:
        return ROOT / "docs" / "emulators" / p

    def out(chip: str, p: str) -> Path:
        return ROOT / "docs" / chip / "annotated" / p

    return {
        "4001": ChipSpec(
            chip="4001",
            schematic_bmp=emu("i4001-schematic.bmp"),
            signals_txt=emu("i4001-signals.txt"),
            poly_bmp=emu("i4001-poly.bmp"),
            diffusion_bmp=emu("i4001-diffusion.bmp"),
            out_labels_png=out("4001", "i4001-schematic-bus-labels.png"),
            out_transistors_png=out("4001", "i4001-poly-diffusion-transistors.png"),
            label_signals={
                "CLK1",
                "CLK2",
                "RESET",
                "CL",
                "SYNC",
                "CM",
                "D0_PAD",
                "D1_PAD",
                "D2_PAD",
                "D3_PAD",
                "IO0",
                "IO1",
                "IO2",
                "IO3",
            },
        ),
        "4002": ChipSpec(
            chip="4002",
            schematic_bmp=emu("i4002-schematic.bmp"),
            signals_txt=emu("i4002-signals.txt"),
            poly_bmp=emu("i4002-poly.bmp"),
            diffusion_bmp=emu("i4002-diffusion.bmp"),
            out_labels_png=out("4002", "i4002-schematic-bus-labels.png"),
            out_transistors_png=out("4002", "i4002-poly-diffusion-transistors.png"),
            label_signals={
                "CLK1",
                "CLK2",
                "RESET",
                "CS",
                "SYNC",
                "CM",
                "D0_PAD",
                "D1_PAD",
                "D2_PAD",
                "D3_PAD",
                "OUT0",
                "OUT1",
                "OUT2",
                "OUT3",
            },
        ),
        "4003": ChipSpec(
            chip="4003",
            schematic_bmp=emu("i4003-schematic.bmp"),
            signals_txt=emu("i4003-signals.txt"),
            poly_bmp=emu("i4003-poly.bmp"),
            diffusion_bmp=emu("i4003-diffusion.bmp"),
            out_labels_png=out("4003", "i4003-schematic-bus-labels.png"),
            out_transistors_png=out("4003", "i4003-poly-diffusion-transistors.png"),
            label_signals={"CLOCK", "DATA", "EN", "OUT", "Q0", "Q1", "Q2", "Q3", "Q4", "Q5", "Q6", "Q7", "Q8", "Q9"},
        ),
        "4004": ChipSpec(
            chip="4004",
            schematic_bmp=emu("i4004-schematic.bmp"),
            signals_txt=emu("i4004-signals.txt"),
            poly_bmp=emu("i4004-poly.bmp"),
            diffusion_bmp=emu("i4004-diffusion.bmp"),
            out_labels_png=out("4004", "i4004-schematic-bus-labels.png"),
            out_transistors_png=out("4004", "i4004-poly-diffusion-transistors.png"),
            label_signals={
                "CLK1",
                "CLK2",
                "SYNC",
                "CMROM",
                "CMRAM0",
                "CMRAM1",
                "CMRAM2",
                "CMRAM3",
                "POC_PAD",
                "TEST_PAD",
                "D0_PAD",
                "D1_PAD",
                "D2_PAD",
                "D3_PAD",
                "D0",
                "D1",
                "D2",
                "D3",
            },
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate annotated overlays for i400x analyzer bitmap assets.")
    parser.add_argument("--chip", action="append", choices=sorted(specs().keys()), help="Chip to generate (repeatable)")
    parser.add_argument("--all", action="store_true", help="Generate for all supported chips")
    args = parser.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = set(specs().keys())
    if not selected:
        parser.error("select --all or at least one --chip")

    for chip in sorted(selected):
        spec = specs()[chip]

        poly = load_mask(spec.poly_bmp)
        diffusion = load_mask(spec.diffusion_bmp)
        save_poly_diff_overlay(poly, diffusion, spec.out_transistors_png)
        save_schematic_labels(spec.schematic_bmp, spec.signals_txt, spec.label_signals, spec.out_labels_png)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
