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
DEFAULT_MIN_INLIERS = 6
DEFAULT_MIN_INLIER_RATIO = 0.5
DEFAULT_MAX_RMSE_PIXELS = 250.0


def compute_homography(
    src_points: np.ndarray,
    dst_points: np.ndarray,
) -> tuple[np.ndarray, float, np.ndarray, np.ndarray]:
    """
    Compute homography matrix from source to destination points.

    Args:
        src_points: Nx2 array of source coordinates
        dst_points: Nx2 array of destination coordinates

    Returns:
        (H, rmse, residuals, inlier_mask):
            Homography matrix (3x3), root mean squared error, per-point residuals,
            and RANSAC inlier mask (1=inlier, 0=outlier)

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
    w = transformed[:, 2]
    if np.any(np.isclose(w, 0.0)):
        raise ValueError("Degenerate homography (w≈0 for at least one point)")
    transformed_points = transformed[:, :2] / w[:, np.newaxis]

    residuals = np.linalg.norm(dst_points - transformed_points, axis=1)
    rmse = float(np.sqrt(np.mean(residuals**2)))
    inlier_mask = mask.reshape(-1) if mask is not None else np.ones(src_points.shape[0], dtype=np.uint8)

    return H, rmse, residuals, inlier_mask


def bbox_center(bbox: dict[str, Any] | None) -> tuple[float, float] | None:
    """Return bbox center if bbox has x0/x1/y0/y1."""
    if not isinstance(bbox, dict):
        return None
    required = ("x0", "x1", "y0", "y1")
    if any(k not in bbox or not isinstance(bbox[k], (int, float)) for k in required):
        return None
    return (
        float(bbox["x0"] + bbox["x1"]) / 2.0,
        float(bbox["y0"] + bbox["y1"]) / 2.0,
    )


def extract_correspondences(anchors_payload: dict[str, Any], chip: str) -> list[dict[str, Any]]:
    """Extract schematic↔layout point correspondences for a chip."""
    all_anchors = anchors_payload.get("anchors")
    if not isinstance(all_anchors, dict):
        raise ValueError("anchors file missing top-level 'anchors' object")

    chip_anchors = all_anchors.get(chip)
    if not isinstance(chip_anchors, dict):
        raise ValueError(f"anchors file missing chip block: {chip}")

    correspondences: list[dict[str, Any]] = []
    for signal_name, anchor in sorted(chip_anchors.items()):
        if not isinstance(anchor, dict):
            continue

        schematic_point = anchor.get("schematic_point")
        if not isinstance(schematic_point, dict):
            continue
        sx = schematic_point.get("x")
        sy = schematic_point.get("y")
        if not isinstance(sx, (int, float)) or not isinstance(sy, (int, float)):
            continue

        layout_point = bbox_center(anchor.get("layout_bbox"))
        layout_source = "layout_bbox"
        if layout_point is None:
            remap = anchor.get("remap_v1")
            if isinstance(remap, dict):
                layout_point = bbox_center(remap.get("src_bbox"))
                layout_source = "remap_v1.src_bbox"
        if layout_point is None:
            continue

        correspondences.append(
            {
                "signal": signal_name,
                "schematic_point": [float(sx), float(sy)],
                "layout_point": [layout_point[0], layout_point[1]],
                "layout_source": layout_source,
            }
        )

    return correspondences


def build_transform_for_chip(
    chip: str,
    anchors_payload: dict[str, Any],
    output_dir: Path,
    min_inliers: int,
    min_inlier_ratio: float,
    max_rmse_pixels: float,
) -> dict[str, Any]:
    """
    Build coordinate transform for a specific chip.

    Args:
        chip: Chip name (4001, 4002, 4003, 4004)
        anchors_payload: Parsed anchors JSON payload
        output_dir: Output directory for transform files

    Returns:
        Transform metadata dictionary
    """
    correspondences = extract_correspondences(anchors_payload, chip)
    if len(correspondences) < 4:
        raise ValueError(
            f"Need at least 4 correspondences for homography, found {len(correspondences)}"
        )

    src_points = np.array([c["schematic_point"] for c in correspondences], dtype=np.float64)
    dst_points = np.array([c["layout_point"] for c in correspondences], dtype=np.float64)

    source_counts: dict[str, int] = {}
    for c in correspondences:
        source_counts[c["layout_source"]] = source_counts.get(c["layout_source"], 0) + 1

    centered = src_points - np.mean(src_points, axis=0, keepdims=True)
    src_rank = int(np.linalg.matrix_rank(centered))
    if src_rank < 2:
        transform_data = {
            "chip": chip,
            "schema_version": 1,
            "transform_type": "unresolved_collinear",
            "matrix": None,
            "validation": {
                "rmse_pixels": None,
                "num_points": len(correspondences),
                "max_residual": None,
                "inliers": 0,
                "inlier_ratio": 0.0,
                "outliers": len(correspondences),
                "layout_sources": source_counts,
                "source_rank": src_rank,
                "quality_thresholds": {
                    "min_inliers": min_inliers,
                    "min_inlier_ratio": min_inlier_ratio,
                    "max_rmse_pixels": max_rmse_pixels,
                },
                "quality_failures": ["source_rank<2 (collinear schematic points)"],
            },
            "correspondences": correspondences,
            "notes": (
                "Insufficient non-collinear schematic points for 2D homography. "
                "Add anchors with varied schematic X/Y positions for this chip."
            ),
        }
        output_file = output_dir / f"{chip}_transform.json"
        output_file.write_text(json.dumps(transform_data, indent=2) + "\n", encoding="utf-8")
        return transform_data

    H, rmse, residuals, inlier_mask = compute_homography(src_points, dst_points)
    inlier_count = int(np.count_nonzero(inlier_mask))
    inlier_ratio = float(inlier_count) / float(len(correspondences))
    inlier_rmse = None
    if inlier_count > 0:
        inlier_rmse = float(np.sqrt(np.mean((residuals[inlier_mask.astype(bool)]) ** 2)))

    quality_failures: list[str] = []
    if inlier_count < min_inliers:
        quality_failures.append(f"inliers<{min_inliers}")
    if inlier_ratio < min_inlier_ratio:
        quality_failures.append(f"inlier_ratio<{min_inlier_ratio:.2f}")
    if rmse > max_rmse_pixels:
        quality_failures.append(f"rmse_pixels>{max_rmse_pixels:.1f}")

    if inlier_count < 4:
        transform_data = {
            "chip": chip,
            "schema_version": 1,
            "transform_type": "unresolved_low_inliers",
            "matrix": None,
            "validation": {
                "rmse_pixels": rmse,
                "rmse_inliers": inlier_rmse,
                "num_points": len(correspondences),
                "max_residual": float(np.max(residuals)),
                "inliers": inlier_count,
                "inlier_ratio": inlier_ratio,
                "outliers": int(len(inlier_mask) - inlier_count),
                "layout_sources": source_counts,
                "source_rank": src_rank,
                "quality_thresholds": {
                    "min_inliers": min_inliers,
                    "min_inlier_ratio": min_inlier_ratio,
                    "max_rmse_pixels": max_rmse_pixels,
                },
                "quality_failures": quality_failures,
            },
            "correspondences": correspondences,
            "notes": (
                "Homography fit is unstable (fewer than 4 inliers). "
                "Refine anchor correspondences before using this transform."
            ),
        }
        output_file = output_dir / f"{chip}_transform.json"
        output_file.write_text(json.dumps(transform_data, indent=2) + "\n", encoding="utf-8")
        return transform_data

    if quality_failures:
        transform_data = {
            "chip": chip,
            "schema_version": 1,
            "transform_type": "unresolved_quality_gate",
            "matrix": None,
            "validation": {
                "rmse_pixels": rmse,
                "rmse_inliers": inlier_rmse,
                "num_points": len(correspondences),
                "max_residual": float(np.max(residuals)),
                "inliers": inlier_count,
                "inlier_ratio": inlier_ratio,
                "outliers": int(len(inlier_mask) - inlier_count),
                "layout_sources": source_counts,
                "source_rank": src_rank,
                "quality_thresholds": {
                    "min_inliers": min_inliers,
                    "min_inlier_ratio": min_inlier_ratio,
                    "max_rmse_pixels": max_rmse_pixels,
                },
                "quality_failures": quality_failures,
            },
            "correspondences": correspondences,
            "notes": (
                "Homography fit failed acceptance thresholds. "
                "Refine correspondences before using this transform."
            ),
        }
        output_file = output_dir / f"{chip}_transform.json"
        output_file.write_text(json.dumps(transform_data, indent=2) + "\n", encoding="utf-8")
        return transform_data

    transform_data = {
        "chip": chip,
        "schema_version": 1,
        "transform_type": "homography",
        "matrix": H.tolist(),
        "validation": {
            "rmse_pixels": rmse,
            "rmse_inliers": inlier_rmse,
            "num_points": len(correspondences),
            "max_residual": float(np.max(residuals)),
            "inliers": inlier_count,
            "inlier_ratio": inlier_ratio,
            "outliers": int(len(inlier_mask) - inlier_count),
            "layout_sources": source_counts,
            "source_rank": src_rank,
            "quality_thresholds": {
                "min_inliers": min_inliers,
                "min_inlier_ratio": min_inlier_ratio,
                "max_rmse_pixels": max_rmse_pixels,
            },
            "quality_failures": [],
        },
        "correspondences": correspondences,
        "notes": "Computed from schematic_layout_anchors_v1.json using RANSAC homography fit.",
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
        "--anchors",
        type=Path,
        default=ROOT / "docs" / "evidence" / "schematic_layout_anchors_v1.json",
        help="Anchor mapping JSON with schematic/layout coordinates",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "coordinate_transforms_v0",
        help="Output directory",
    )
    parser.add_argument(
        "--min-inliers",
        type=int,
        default=DEFAULT_MIN_INLIERS,
        help="Minimum RANSAC inlier count required for an accepted transform",
    )
    parser.add_argument(
        "--min-inlier-ratio",
        type=float,
        default=DEFAULT_MIN_INLIER_RATIO,
        help="Minimum inlier ratio required for an accepted transform",
    )
    parser.add_argument(
        "--max-rmse-pixels",
        type=float,
        default=DEFAULT_MAX_RMSE_PIXELS,
        help="Maximum overall RMSE (pixels) allowed for an accepted transform",
    )

    args = parser.parse_args()

    anchors_path = args.anchors if args.anchors.is_absolute() else (ROOT / args.anchors)
    if not anchors_path.exists():
        raise SystemExit(f"Anchors file not found: {anchors_path}")

    anchors_payload = json.loads(anchors_path.read_text(encoding="utf-8"))

    args.output_dir.mkdir(parents=True, exist_ok=True)

    print("Building coordinate transforms...")
    print("")

    failures: list[str] = []
    for chip in args.chips:
        print(f"Processing {chip}...")

        try:
            transform = build_transform_for_chip(
                chip=chip,
                anchors_payload=anchors_payload,
                output_dir=args.output_dir,
                min_inliers=args.min_inliers,
                min_inlier_ratio=args.min_inlier_ratio,
                max_rmse_pixels=args.max_rmse_pixels,
            )

            print(f"  ✓ Created transform: {args.output_dir / f'{chip}_transform.json'}")
            rmse = transform["validation"]["rmse_pixels"]
            if isinstance(rmse, (int, float)):
                print(f"    RMSE: {rmse:.2f} pixels")
            else:
                print("    RMSE: n/a (insufficient geometric diversity)")
            print(f"    Transform type: {transform['transform_type']}")
            print(
                "    Points: "
                f"{transform['validation']['num_points']} "
                f"(inliers={transform['validation']['inliers']}, "
                f"outliers={transform['validation']['outliers']}, "
                f"rank={transform['validation']['source_rank']})"
            )

        except Exception as e:
            print(f"  ✗ Failed: {e}")
            failures.append(chip)
            continue

    print("")
    print("✓ Transform generation complete")
    if failures:
        print(f"⚠ Chips without transform output: {', '.join(failures)}")


if __name__ == "__main__":
    main()
