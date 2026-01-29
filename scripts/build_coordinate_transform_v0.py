#!/usr/bin/env python3
"""
Build schematic↔layout coordinate transforms using homography.

This script computes affine/homography transformations to map between
schematic coordinate space and layout pixel space.

Expected inputs:
- Anchor points with known schematic AND layout coordinates
- Minimum 4 points for homography (more is better for robustness)

Output:
- JSON files with transformation matrices
- Validation metrics (residuals, RMSE)
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import cv2
import numpy as np

ROOT = Path(__file__).resolve().parents[1]


def compute_homography(
    src_points: np.ndarray,
    dst_points: np.ndarray,
) -> tuple[np.ndarray, float]:
    """
    Compute homography matrix from source to destination points.

    Args:
        src_points: Nx2 array of source coordinates
        dst_points: Nx2 array of destination coordinates

    Returns:
        (H, rmse): Homography matrix (3x3) and root mean squared error

    Raises:
        ValueError: If fewer than 4 points provided or homography cannot be computed
    """
    if src_points.shape[0] < 4:
        raise ValueError(f"Need at least 4 points for homography, got {src_points.shape[0]}")

    # Compute homography using RANSAC for robustness
    H, mask = cv2.findHomography(
        src_points,
        dst_points,
        method=cv2.RANSAC,
        ransacReprojThreshold=5.0,
    )

    if H is None:
        raise ValueError("Failed to compute homography")

    # Compute residuals for validation
    src_homog = np.hstack([src_points, np.ones((src_points.shape[0], 1))])
    transformed = (H @ src_homog.T).T
    transformed_points = transformed[:, :2] / transformed[:, 2:3]

    residuals = np.linalg.norm(dst_points - transformed_points, axis=1)
    rmse = float(np.sqrt(np.mean(residuals**2)))

    return H, rmse


def build_transform_for_chip(
    chip: str,
    anchor_file: Path,
    output_dir: Path,
) -> dict[str, Any]:
    """
    Build coordinate transform for a specific chip.

    Args:
        chip: Chip name (4001, 4002, 4003, 4004)
        anchor_file: Path to anchor coordinate file (JSON)
        output_dir: Output directory for transform files

    Returns:
        Transform metadata dictionary
    """
    # Placeholder: In real implementation, this would load actual anchor coordinates
    # from schematic and layout files, then compute homography.

    # For now, create an identity transform as a placeholder
    H = np.eye(3)
    rmse = 0.0

    transform_data = {
        "chip": chip,
        "schema_version": 0,
        "transform_type": "homography",
        "matrix": H.tolist(),
        "validation": {
            "rmse_pixels": rmse,
            "num_points": 0,
            "max_residual": 0.0,
        },
        "notes": "PLACEHOLDER: Identity transform - replace with actual homography",
    }

    output_file = output_dir / f"{chip}_transform.json"
    output_file.write_text(json.dumps(transform_data, indent=2) + "\n", encoding="utf-8")

    return transform_data


def main():
    parser = argparse.ArgumentParser(description="Build coordinate transforms")
    parser.add_argument(
        "--chips",
        nargs="+",
        default=["4001", "4002", "4003", "4004"],
        help="Chips to process",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "coordinate_transforms_v0",
        help="Output directory",
    )

    args = parser.parse_args()

    args.output_dir.mkdir(parents=True, exist_ok=True)

    print("Building coordinate transforms...")
    print("")

    for chip in args.chips:
        print(f"Processing {chip}...")

        # Placeholder anchor file
        anchor_file = ROOT / "docs" / "evidence" / "anchor_incidence_v1_canonical" / chip / chip / f"{chip}_anchor_incidence_v0.json"

        if not anchor_file.exists():
            print(f"  ✗ Anchor file not found: {anchor_file}")
            continue

        try:
            transform = build_transform_for_chip(
                chip=chip,
                anchor_file=anchor_file,
                output_dir=args.output_dir,
            )

            print(f"  ✓ Created transform: {args.output_dir / f'{chip}_transform.json'}")
            print(f"    RMSE: {transform['validation']['rmse_pixels']:.2f} pixels")

        except Exception as e:
            print(f"  ✗ Failed: {e}")
            continue

    print("")
    print("✓ Transform generation complete")
    print("")
    print("NOTE: This is a PLACEHOLDER implementation.")
    print("TODO: Replace with actual homography computation from anchor coordinates.")


if __name__ == "__main__":
    main()
