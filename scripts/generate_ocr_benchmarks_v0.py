#!/usr/bin/env python3
"""
Generate comprehensive OCR benchmark files for all chips.

This script scans OCR crop directories and creates benchmark JSON files
with expected labels extracted from filenames.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def extract_label_from_filename(filename: str) -> str:
    """
    Extract expected label from crop filename.

    Pattern: NNNN_LABEL.png → LABEL

    Examples:
        0000_D0_PAD.png → D0_PAD
        0001_~A11_2.png → ~A11_2
        0042_CHIPSEL.png → CHIPSEL
    """
    # Remove leading number and underscore
    match = re.match(r'^\d+_(.+)\.png$', filename)
    if match:
        return match.group(1)
    return filename.replace('.png', '')


def generate_benchmark_for_chip(chip: str, min_samples: int = 20, max_samples: int = 200) -> dict:
    """
    Generate benchmark JSON for a specific chip.

    Args:
        chip: Chip name (4001, 4002, 4003, 4004)
        min_samples: Minimum number of samples to include
        max_samples: Maximum number of samples to include (for large sets)

    Returns:
        Benchmark dictionary with schema and items
    """
    crop_dir = ROOT / "docs" / "evidence" / "ocr_signal_labels" / chip / "crops"

    if not crop_dir.exists():
        raise FileNotFoundError(f"Crop directory not found: {crop_dir}")

    # Gather all crops
    crops = sorted(crop_dir.glob("*.png"))

    if len(crops) < min_samples:
        print(f"Warning: {chip} has only {len(crops)} crops (min {min_samples})")

    # Limit to max_samples for very large sets
    if len(crops) > max_samples:
        # Sample evenly across the range
        step = len(crops) // max_samples
        crops = crops[::step][:max_samples]

    # Build items
    items = []
    for crop_path in crops:
        label = extract_label_from_filename(crop_path.name)

        # Skip internal/debug crops
        if label.startswith('_') or label.startswith('DEBUG'):
            continue

        # Relative path from project root
        rel_path = crop_path.relative_to(ROOT)

        item = {
            "id": f"{chip}_{label}_{crop_path.stem}",
            "expected": label,
            "image": str(rel_path),
        }
        items.append(item)

    return {
        "schema": {
            "version": 0,
            "description": f"Comprehensive OCR regression benchmark for {chip} signal labels (pad labels and edge labels).",
            "chip": chip,
            "total_samples": len(items),
            "source": "docs/evidence/ocr_signal_labels/",
        },
        "items": items,
    }


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Generate OCR benchmark files")
    parser.add_argument(
        "--chips",
        nargs="+",
        default=["4001", "4002", "4003", "4004"],
        help="Chips to generate benchmarks for",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "ocr_benchmarks_v0",
        help="Output directory for benchmark files",
    )
    parser.add_argument(
        "--min-samples",
        type=int,
        default=20,
        help="Minimum samples per chip",
    )
    parser.add_argument(
        "--max-samples",
        type=int,
        default=200,
        help="Maximum samples per chip",
    )

    args = parser.parse_args()

    args.output_dir.mkdir(parents=True, exist_ok=True)

    for chip in args.chips:
        try:
            print(f"Generating benchmark for {chip}...")
            benchmark = generate_benchmark_for_chip(
                chip=chip,
                min_samples=args.min_samples,
                max_samples=args.max_samples,
            )

            output_file = args.output_dir / f"signal_labels_{chip}_comprehensive_v0.json"
            output_file.write_text(json.dumps(benchmark, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

            print(f"  ✓ Created {output_file.name} with {len(benchmark['items'])} samples")

        except Exception as e:
            print(f"  ✗ Failed for {chip}: {e}")
            continue

    print("\n✓ Benchmark generation complete")


if __name__ == "__main__":
    main()
