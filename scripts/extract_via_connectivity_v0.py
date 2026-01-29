#!/usr/bin/env python3
"""
Extract via connectivity from mask layer images (netlist_v2).

This script models actual via placement and multi-layer routing to create
more accurate netlists that include explicit via connectivity rather than
assuming transistor A-B nodes are electrically connected.

Input:
- Transistor netlist (v0 or v1)
- Via layer bitmap (i400X-vias.bmp)
- Metal layer bitmap (i400X-metal.bmp)

Output:
- netlist_v2 with explicit via nodes and connectivity
- Via statistics and validation reports
"""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import cv2
import numpy as np

ROOT = Path(__file__).resolve().parents[1]


@dataclass
class Via:
    """Represents a via connection between layers."""

    x: int
    y: int
    diameter: int
    connects_to: list[int]  # Node IDs connected by this via


@dataclass
class NetlistV2Node:
    """Enhanced node with via connectivity."""

    node_id: int
    transistor_terminals: list[tuple[int, str]]  # (transistor_id, terminal)
    via_connections: list[int]  # Via IDs connecting to this node
    metal_area: float
    layer: str  # "poly", "metal", "diffusion"


def detect_vias_from_image(via_image: np.ndarray, min_diameter: int = 3) -> list[Via]:
    """
    Detect via positions from via layer bitmap.

    Args:
        via_image: Binary image of via layer
        min_diameter: Minimum via diameter in pixels

    Returns:
        List of detected vias with positions
    """
    # Detect circular features (vias are typically circular holes)
    circles = cv2.HoughCircles(
        via_image,
        cv2.HOUGH_GRADIENT,
        dp=1,
        minDist=min_diameter * 2,
        param1=50,
        param2=30,
        minRadius=min_diameter // 2,
        maxRadius=min_diameter * 3,
    )

    vias = []
    if circles is not None:
        circles = np.uint16(np.around(circles))
        for circle in circles[0, :]:
            x, y, r = circle
            vias.append(Via(x=int(x), y=int(y), diameter=int(r * 2), connects_to=[]))

    return vias


def extract_via_connectivity(
    chip: str,
    netlist_v1_path: Path,
    via_image_path: Path,
    metal_image_path: Path,
) -> dict[str, Any]:
    """
    Extract via connectivity and generate netlist_v2.

    Args:
        chip: Chip name (4001, 4002, 4003, 4004)
        netlist_v1_path: Path to existing netlist_v1
        via_image_path: Path to via layer bitmap
        metal_image_path: Path to metal layer bitmap

    Returns:
        netlist_v2 dictionary with via connectivity
    """
    # Placeholder implementation
    # In actual implementation:
    # 1. Load netlist_v1
    # 2. Load and process via layer image
    # 3. Load and process metal layer image
    # 4. For each transistor:
    #    a. Check if terminals are connected via metal or via
    #    b. Build via graph
    # 5. Update netlist with explicit via connectivity

    netlist_v2 = {
        "schema_version": 2,
        "chip": chip,
        "description": "Netlist with explicit via connectivity",
        "metadata": {
            "source": "extract_via_connectivity_v0.py",
            "via_detection": "placeholder",
            "metal_routing": "placeholder",
        },
        "nodes": [],
        "vias": [],
        "transistors": [],
        "statistics": {
            "total_nodes": 0,
            "total_vias": 0,
            "total_transistors": 0,
            "via_connected_nodes": 0,
            "direct_metal_connections": 0,
        },
    }

    return netlist_v2


def validate_via_connectivity(netlist_v2: dict[str, Any]) -> dict[str, Any]:
    """
    Validate via connectivity for consistency.

    Checks:
    - All vias connect to valid nodes
    - No floating vias (connected to <2 nodes)
    - Via density reasonable
    - Transistor terminal connectivity preserved

    Returns:
        Validation report
    """
    report = {
        "valid": True,
        "errors": [],
        "warnings": [],
        "statistics": {
            "floating_vias": 0,
            "over_connected_vias": 0,
            "disconnected_transistors": 0,
        },
    }

    # Placeholder validation
    # In actual implementation, check:
    # - Via connectivity consistency
    # - Transistor terminal reachability
    # - Metal routing continuity

    return report


def main():
    parser = argparse.ArgumentParser(description="Extract via connectivity (netlist_v2)")
    parser.add_argument(
        "--chips",
        nargs="+",
        default=["4001", "4002", "4003", "4004"],
        help="Chips to process",
    )
    parser.add_argument(
        "--input-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "netlists_v1",
        help="Input netlist_v1 directory",
    )
    parser.add_argument(
        "--via-dir",
        type=Path,
        default=ROOT / "docs" / "photomicrographs",
        help="Directory containing via layer bitmaps",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "netlists_v2",
        help="Output directory for netlist_v2",
    )

    args = parser.parse_args()

    args.output_dir.mkdir(parents=True, exist_ok=True)

    print("=== Via Connectivity Extraction (netlist_v2) ===")
    print("")

    for chip in args.chips:
        print(f"Processing {chip}...")

        # Construct file paths
        netlist_v1 = args.input_dir / chip / f"{chip}_netlist_v1.json"
        via_image = args.via_dir / f"i{chip}-vias.bmp"
        metal_image = args.via_dir / f"i{chip}-metal.bmp"

        # Check if input files exist
        if not netlist_v1.exists():
            print(f"  ⚠ Netlist v1 not found: {netlist_v1}")
            print(f"    Skipping {chip} (input required)")
            continue

        if not via_image.exists():
            print(f"  ⚠ Via layer not found: {via_image}")
            print(f"    Using placeholder via detection")

        if not metal_image.exists():
            print(f"  ⚠ Metal layer not found: {metal_image}")
            print(f"    Using placeholder metal routing")

        try:
            # Extract via connectivity
            netlist_v2 = extract_via_connectivity(
                chip=chip,
                netlist_v1_path=netlist_v1,
                via_image_path=via_image,
                metal_image_path=metal_image,
            )

            # Validate
            validation = validate_via_connectivity(netlist_v2)

            # Save netlist_v2
            output_file = args.output_dir / chip / f"{chip}_netlist_v2.json"
            output_file.parent.mkdir(parents=True, exist_ok=True)
            output_file.write_text(json.dumps(netlist_v2, indent=2) + "\n", encoding="utf-8")

            # Save validation report
            validation_file = args.output_dir / chip / f"{chip}_via_validation.json"
            validation_file.write_text(json.dumps(validation, indent=2) + "\n", encoding="utf-8")

            print(f"  ✓ Created netlist_v2: {output_file.name}")
            print(f"    Nodes: {netlist_v2['statistics']['total_nodes']}")
            print(f"    Vias: {netlist_v2['statistics']['total_vias']}")
            print(f"    Validation: {'✓ PASS' if validation['valid'] else '✗ FAIL'}")

        except Exception as e:
            print(f"  ✗ Failed: {e}")
            continue

        print("")

    print("✓ Via connectivity extraction complete")
    print("")
    print("NOTE: This is a PLACEHOLDER implementation.")
    print("TODO: Implement actual via detection from bitmap images.")
    print("TODO: Implement metal routing analysis.")
    print("TODO: Build via graph from detected features.")


if __name__ == "__main__":
    main()
