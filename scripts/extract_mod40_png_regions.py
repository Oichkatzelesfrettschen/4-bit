#!/usr/bin/env python3
"""Render registered MOD 40 schematic regions as source-derived PNG OCR candidates."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import warnings
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

import cv2
from PIL import Image
from rapidocr_onnxruntime import RapidOCR

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE_PDF = REPO_ROOT / "docs/MCS-40/Intel_Intellec_4_MOD_40_Reference_Schematics.pdf"
DEFAULT_REFERENCE_RENDER = (
    REPO_ROOT / "docs/evidence/ocr/mod40_98013_20260713/full-sheet-v3/render"
)
DEFAULT_INPUT_CROPS = REPO_ROOT / "docs/evidence/ocr/mod40_98013_20260713/targeted-1200/crops"
DEFAULT_OUTPUT = REPO_ROOT / "docs/evidence/ocr/mod40_98013_20260713/png-regions-v1"
REFERENCE_DPI = 600
MAX_REGION_PIXELS = 200_000_000
PAGE_PATTERN = re.compile(r"^page-(\d{2})-")


@dataclass(frozen=True)
class RegionRegistration:
    name: str
    page: int
    reference_crop: str
    reference_crop_sha256: str
    reference_render: str
    reference_render_sha256: str
    registration_score: float
    reference_x: int
    reference_y: int
    reference_width: int
    reference_height: int
    output_png: str
    output_png_sha256: str
    output_x: int
    output_y: int
    output_width: int
    output_height: int


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as input_file:
        for chunk in iter(lambda: input_file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def one_based_pages(value: str) -> set[int]:
    pages: set[int] = set()
    for item in value.split(","):
        if not item.isdecimal():
            raise argparse.ArgumentTypeError(f"invalid one-based PDF page: {item}")
        page = int(item)
        if page < 1 or page > 35:
            raise argparse.ArgumentTypeError(f"page outside 1..35: {page}")
        pages.add(page)
    if not pages:
        raise argparse.ArgumentTypeError("page list is empty")
    return pages


def page_for_crop(path: Path) -> int:
    match = PAGE_PATTERN.match(path.name)
    if match is None:
        raise ValueError(f"crop name lacks a page prefix: {path.name}")
    return int(match.group(1))


def grayscale_image(path: Path) -> Any:
    image = cv2.imread(str(path), cv2.IMREAD_GRAYSCALE)
    if image is None:
        raise ValueError(f"cannot decode image: {path}")
    return image


def image_dpi(path: Path) -> float:
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", Image.DecompressionBombWarning)
        with Image.open(path) as image:
            pixel_count = image.width * image.height
            dpi = image.info.get("dpi")
    if pixel_count > MAX_REGION_PIXELS:
        raise ValueError(f"image exceeds {MAX_REGION_PIXELS} pixels: {path}")
    if not isinstance(dpi, tuple) or len(dpi) != 2:
        raise ValueError(f"image lacks DPI metadata: {path}")
    horizontal_dpi, vertical_dpi = dpi
    if horizontal_dpi <= 0 or vertical_dpi <= 0:
        raise ValueError(f"image has nonpositive DPI metadata: {path}")
    if abs(horizontal_dpi - vertical_dpi) > 0.01:
        raise ValueError(f"image has unequal X/Y DPI metadata: {path}")
    return float(horizontal_dpi)


def register_crop(reference_render: Path, crop: Path) -> tuple[int, int, float, int, int]:
    page_image = grayscale_image(reference_render)
    crop_image = grayscale_image(crop)
    crop_scale = image_dpi(crop) / REFERENCE_DPI
    if crop_scale <= 0:
        raise ValueError(f"invalid crop scale: {crop}")
    if abs(crop_scale - 1) > 0.01:
        crop_image = cv2.resize(
            crop_image,
            dsize=(round(crop_image.shape[1] / crop_scale), round(crop_image.shape[0] / crop_scale)),
            interpolation=cv2.INTER_AREA,
        )
    page_height, page_width = page_image.shape
    crop_height, crop_width = crop_image.shape
    if crop_height > page_height or crop_width > page_width:
        raise ValueError(f"crop exceeds reference render: {crop}")

    stride = 8
    page_small = page_image[::stride, ::stride]
    crop_small = crop_image[::stride, ::stride]
    _, coarse_score, _, coarse_location = cv2.minMaxLoc(
        cv2.matchTemplate(page_small, crop_small, cv2.TM_CCOEFF_NORMED)
    )
    coarse_x = coarse_location[0] * stride
    coarse_y = coarse_location[1] * stride
    search_x = max(0, coarse_x - stride * 2)
    search_y = max(0, coarse_y - stride * 2)
    search_right = min(page_width, coarse_x + crop_width + stride * 2)
    search_bottom = min(page_height, coarse_y + crop_height + stride * 2)
    search_image = page_image[search_y:search_bottom, search_x:search_right]
    _, exact_score, _, exact_location = cv2.minMaxLoc(
        cv2.matchTemplate(search_image, crop_image, cv2.TM_CCOEFF_NORMED)
    )
    if exact_score < coarse_score:
        return coarse_x, coarse_y, float(coarse_score), crop_width, crop_height
    return (
        search_x + exact_location[0],
        search_y + exact_location[1],
        float(exact_score),
        crop_width,
        crop_height,
    )


def render_png(
    *,
    pdftocairo: str,
    source_pdf: Path,
    page: int,
    output_png: Path,
    dpi: int,
    x: int,
    y: int,
    width: int,
    height: int,
) -> None:
    scale = dpi / REFERENCE_DPI
    command = [
        pdftocairo,
        "-png",
        "-singlefile",
        "-f",
        str(page),
        "-l",
        str(page),
        "-r",
        str(dpi),
        "-x",
        str(round(x * scale)),
        "-y",
        str(round(y * scale)),
        "-W",
        str(round(width * scale)),
        "-H",
        str(round(height * scale)),
        str(source_pdf),
        str(output_png.with_suffix("")),
    ]
    # The executable comes from PATH resolution and all file paths exist locally.
    subprocess.run(command, check=True, capture_output=True, text=True)  # noqa: S603


def run_tesseract(tesseract: str, image_path: Path, psm: int) -> dict[str, Any]:
    completed = subprocess.run(  # noqa: S603
        [tesseract, str(image_path), "stdout", "--psm", str(psm)],
        check=False,
        capture_output=True,
        text=True,
    )
    return {
        "exit_status": completed.returncode,
        "psm": psm,
        "stderr": completed.stderr,
        "text": completed.stdout,
    }


def run_rapidocr(engine: RapidOCR, image_path: Path) -> dict[str, Any]:
    items, timing = engine(str(image_path))
    return {
        "items": [
            {"bbox": box, "text": text, "confidence": confidence}
            for box, text, confidence in items or []
        ],
        "timing_seconds": timing,
    }


def find_crops(input_directory: Path, pages: set[int]) -> list[Path]:
    crops = []
    for crop in sorted(input_directory.glob("*.jpg")):
        if page_for_crop(crop) in pages:
            crops.append(crop)
    if not crops:
        raise ValueError(f"no matching JPEG crops in {input_directory}")
    return crops


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-pdf", type=Path, default=DEFAULT_SOURCE_PDF)
    parser.add_argument("--reference-render", type=Path, default=DEFAULT_REFERENCE_RENDER)
    parser.add_argument("--input-crops", type=Path, default=DEFAULT_INPUT_CROPS)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--pages", type=one_based_pages, default={3, 5, 7, 10, 13, 29})
    parser.add_argument("--dpi", type=int, default=1200)
    parser.add_argument("--minimum-registration-score", type=float, default=0.9)
    parser.add_argument("--tesseract-psm", type=int, default=11)
    arguments = parser.parse_args()

    if arguments.dpi < REFERENCE_DPI:
        parser.error(f"--dpi must be at least {REFERENCE_DPI}")
    pdftocairo = shutil.which("pdftocairo")
    tesseract = shutil.which("tesseract")
    if pdftocairo is None:
        parser.error("missing required command: pdftocairo")
    if tesseract is None:
        parser.error("missing required command: tesseract")
    for input_path in (arguments.source_pdf, arguments.reference_render, arguments.input_crops):
        if not input_path.exists():
            parser.error(f"missing input: {input_path}")

    png_directory = arguments.output / "png"
    candidate_directory = arguments.output / "candidates"
    metadata_directory = arguments.output / "metadata"
    png_directory.mkdir(parents=True, exist_ok=True)
    candidate_directory.mkdir(parents=True, exist_ok=True)
    metadata_directory.mkdir(parents=True, exist_ok=True)

    rapidocr = RapidOCR()
    registrations: list[RegionRegistration] = []
    rejections: list[dict[str, Any]] = []
    for crop in find_crops(arguments.input_crops, arguments.pages):
        page = page_for_crop(crop)
        reference_render = arguments.reference_render / f"page-{page:02d}.jpg"
        if not reference_render.is_file():
            raise ValueError(f"missing reference render: {reference_render}")
        x, y, score, width, height = register_crop(reference_render, crop)
        if score < arguments.minimum_registration_score:
            rejections.append(
                {
                    "crop": str(crop),
                    "page": page,
                    "registration_score": score,
                    "minimum_registration_score": arguments.minimum_registration_score,
                    "status": "rejected-no-source-png-rendered",
                    "reason": "the legacy crop cannot establish a defensible source coordinate",
                }
            )
            continue
        output_png = png_directory / f"{crop.stem}-{arguments.dpi}.png"
        if not output_png.is_file():
            render_png(
                pdftocairo=pdftocairo,
                source_pdf=arguments.source_pdf,
                page=page,
                output_png=output_png,
                dpi=arguments.dpi,
                x=x,
                y=y,
                width=width,
                height=height,
            )
        scale = arguments.dpi / REFERENCE_DPI
        registration = RegionRegistration(
            name=crop.stem,
            page=page,
            reference_crop=str(crop),
            reference_crop_sha256=file_sha256(crop),
            reference_render=str(reference_render),
            reference_render_sha256=file_sha256(reference_render),
            registration_score=score,
            reference_x=x,
            reference_y=y,
            reference_width=width,
            reference_height=height,
            output_png=str(output_png),
            output_png_sha256=file_sha256(output_png),
            output_x=round(x * scale),
            output_y=round(y * scale),
            output_width=round(width * scale),
            output_height=round(height * scale),
        )
        registrations.append(registration)
        candidate = {
            "schema": "mcs4.mod40.png-region-ocr-candidate.v1",
            "source_class": "ocr-discovery-only",
            "registration": asdict(registration),
            "tesseract": run_tesseract(tesseract, output_png, arguments.tesseract_psm),
            "rapidocr": run_rapidocr(rapidocr, output_png),
            "verification": "requires visual primary-sheet endpoint and polarity review",
        }
        candidate_path = candidate_directory / f"{crop.stem}.json"
        candidate_path.write_text(json.dumps(candidate, indent=2) + "\n", encoding="utf-8")

    manifest = {
        "schema": "mcs4.mod40.png-region-ocr-manifest.v1",
        "source_pdf": str(arguments.source_pdf),
        "source_pdf_sha256": file_sha256(arguments.source_pdf),
        "reference_dpi": REFERENCE_DPI,
        "output_dpi": arguments.dpi,
        "minimum_registration_score": arguments.minimum_registration_score,
        "registrations": [asdict(registration) for registration in registrations],
        "rejections": rejections,
        "verification": "all OCR text remains a candidate until visually traced on the source PDF",
    }
    (metadata_directory / "manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
