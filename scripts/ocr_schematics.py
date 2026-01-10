#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
from dataclasses import dataclass
from pathlib import Path

import cv2
import numpy as np
import pytesseract
from PIL import Image


@dataclass(frozen=True)
class ChipSpec:
    chip: str
    schematic_bmp: Path
    signals_txt: Path


ROOT = Path(__file__).resolve().parents[1]


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


def preprocess(img: Image.Image, *, scale: int, invert: bool) -> Image.Image:
    gray = np.asarray(img.convert("L"))
    if scale != 1:
        gray = cv2.resize(gray, (gray.shape[1] * scale, gray.shape[0] * scale), interpolation=cv2.INTER_CUBIC)

    # Adaptive threshold helps on mixed backgrounds in the analyzer bitmaps.
    th = cv2.adaptiveThreshold(
        gray,
        255,
        cv2.ADAPTIVE_THRESH_GAUSSIAN_C,
        cv2.THRESH_BINARY,
        35,
        5,
    )
    if invert:
        th = 255 - th
    return Image.fromarray(th, mode="L")


def ocr_text(img: Image.Image, *, psm: int) -> str:
    return pytesseract.image_to_string(img, config=f"--psm {psm}")


def ocr_tsv(img: Image.Image, *, psm: int) -> str:
    # TSV output includes bboxes and confidence; it is easier to post-process than plain text.
    return pytesseract.image_to_data(img, config=f"--psm {psm}", output_type=pytesseract.Output.STRING)


def main() -> int:
    parser = argparse.ArgumentParser(description="OCR i400x schematic bitmaps using pytesseract.")
    parser.add_argument("--chip", action="append", choices=sorted(specs().keys()), help="Chip to OCR (repeatable)")
    parser.add_argument("--all", action="store_true", help="OCR all supported chips")
    parser.add_argument("--out-dir", type=Path, default=ROOT / "docs" / "evidence" / "ocr_schematics")
    parser.add_argument("--scale", type=int, default=2, help="Upscale factor before OCR")
    parser.add_argument("--psm", type=int, default=6, help="Tesseract page segmentation mode")
    parser.add_argument("--invert", action="store_true", help="Invert colors after thresholding")
    args = parser.parse_args()

    selected = set(args.chip or [])
    if args.all:
        selected = set(specs().keys())
    if not selected:
        parser.error("select --all or at least one --chip")

    args.out_dir.mkdir(parents=True, exist_ok=True)

    manifest: dict[str, object] = {
        "tool": "scripts/ocr_schematics.py",
        "tesseract_version": str(pytesseract.get_tesseract_version()),
        "params": {"scale": args.scale, "psm": args.psm, "invert": bool(args.invert)},
        "outputs": [],
    }

    for chip in sorted(selected):
        spec = specs()[chip]
        img = Image.open(spec.schematic_bmp)
        pre = preprocess(img, scale=args.scale, invert=bool(args.invert))

        txt = ocr_text(pre, psm=args.psm)
        tsv = ocr_tsv(pre, psm=args.psm)

        base = f"{chip.lower()}_schematic"
        out_txt = args.out_dir / f"{base}.txt"
        out_tsv = args.out_dir / f"{base}.tsv"
        out_meta = args.out_dir / f"{base}.meta.json"

        out_txt.write_text(txt, encoding="utf-8")
        out_tsv.write_text(tsv, encoding="utf-8")

        meta = {
            "chip": chip,
            "inputs": {
                "schematic_bmp": str(spec.schematic_bmp.relative_to(ROOT)),
                "signals_txt": str(spec.signals_txt.relative_to(ROOT)),
                "schematic_sha256": sha256(spec.schematic_bmp),
            },
            "params": {"scale": args.scale, "psm": args.psm, "invert": bool(args.invert)},
            "outputs": {
                "text": str(out_txt.relative_to(ROOT)),
                "tsv": str(out_tsv.relative_to(ROOT)),
            },
        }
        out_meta.write_text(json.dumps(meta, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        manifest["outputs"].append(meta)

    (args.out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

