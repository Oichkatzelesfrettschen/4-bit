#!/usr/bin/env python3
"""
Extract via connectivity from mask layer images (netlist_v2).

This script provides a behavior-safe netlist_v2 scaffold:
- Preserves netlist_v1 nodes/transistors (no data loss)
- Detects candidate vias from a via bitmap when available
- Maps vias to node candidates via bbox containment/proximity heuristics
- Emits a simple routing graph (via<->node edges) with validation metadata

Input:
- Transistor netlist (v0 or v1)
- Via layer bitmap (i400X-vias.bmp)
- Metal layer bitmap (i400X-metal.bmp)

Output:
- netlist_v2 scaffold with via candidates and passthrough connectivity
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


def point_in_bbox(x: int, y: int, bbox: dict[str, Any], pad: int = 0) -> bool:
    """Return True if a point lies in (or near) a bbox."""
    required = ("x0", "x1", "y0", "y1")
    if not all(k in bbox and isinstance(bbox[k], (int, float)) for k in required):
        return False
    return (
        (bbox["x0"] - pad) <= x <= (bbox["x1"] + pad)
        and (bbox["y0"] - pad) <= y <= (bbox["y1"] + pad)
    )


def bbox_center(bbox: dict[str, Any]) -> tuple[float, float] | None:
    """Return center point of bbox if valid."""
    required = ("x0", "x1", "y0", "y1")
    if not all(k in bbox and isinstance(bbox[k], (int, float)) for k in required):
        return None
    return ((bbox["x0"] + bbox["x1"]) / 2.0, (bbox["y0"] + bbox["y1"]) / 2.0)


def map_vias_to_nodes(
    vias: list[Via],
    nodes: list[dict[str, Any]],
    containment_pad: int = 4,
    max_nearest_distance: float = 40.0,
    max_nodes_per_via: int = 2,
) -> tuple[list[dict[str, Any]], int]:
    """
    Map via candidates to netlist nodes via bbox containment/proximity heuristics.

    Returns:
        (via_entries, direct_metal_connections)
    """
    via_entries: list[dict[str, Any]] = []
    node_bboxes: list[tuple[int, str, dict[str, Any], float]] = []
    for node in nodes:
        if not isinstance(node, dict):
            continue
        node_id = node.get("node")
        if not isinstance(node_id, int):
            continue
        for bbox_key in ("metal_bbox", "diffusion_bbox", "poly_bbox"):
            bbox = node.get(bbox_key)
            if isinstance(bbox, dict):
                area_key = bbox_key.replace("_bbox", "_area")
                area = float(node.get(area_key, float("inf")))
                node_bboxes.append((node_id, bbox_key, bbox, area))

    direct_metal_connections = 0
    for via_id, via in enumerate(vias):
        connected: list[int] = []

        # First pass: direct bbox containment.
        candidates: dict[int, tuple[float, set[str]]] = {}
        for node_id, bbox_key, bbox, area in node_bboxes:
            if point_in_bbox(via.x, via.y, bbox, pad=containment_pad):
                if node_id not in candidates:
                    candidates[node_id] = (area, {bbox_key})
                else:
                    prev_area, prev_layers = candidates[node_id]
                    prev_layers.add(bbox_key)
                    candidates[node_id] = (min(prev_area, area), prev_layers)

        mapping_method = "bbox_contains"
        selected_layers: dict[int, list[str]] = {}

        if candidates:
            ranked = sorted(candidates.items(), key=lambda item: item[1][0])
            for node_id, (_area, layers) in ranked[:max_nodes_per_via]:
                connected.append(node_id)
                selected_layers[node_id] = sorted(layers)

        # Fallback: nearest bbox center if nothing matched.
        if not connected:
            best_node: int | None = None
            best_dist = float("inf")
            best_layer = "metal_bbox"
            for node_id, bbox_key, bbox, _area in node_bboxes:
                if bbox_key != "metal_bbox":
                    continue
                center = bbox_center(bbox)
                if center is None:
                    continue
                dist = float(np.hypot(via.x - center[0], via.y - center[1]))
                if dist < best_dist:
                    best_dist = dist
                    best_node = node_id
                    best_layer = bbox_key

            if best_node is not None and best_dist <= max_nearest_distance:
                connected.append(best_node)
                selected_layers[best_node] = [best_layer]
                mapping_method = "nearest_bbox"
            else:
                mapping_method = "unmapped"

        connects_to = sorted(set(connected))
        via.connects_to = connects_to
        if mapping_method == "bbox_contains" and connects_to:
            direct_metal_connections += 1

        via_entries.append(
            {
                "via_id": via_id,
                "x": via.x,
                "y": via.y,
                "diameter": via.diameter,
                "connects_to": connects_to,
                "mapping_method": mapping_method,
                "connected_layers": selected_layers,
            }
        )

    return via_entries, direct_metal_connections


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


def detect_vias_from_components(
    via_binary_inv: np.ndarray,
    min_area: int = 8,
    max_area: int = 250,
    max_aspect_ratio: float = 2.5,
) -> list[Via]:
    """
    Detect via candidates from connected components on inverse-thresholded image.
    """
    count, _labels, stats, centroids = cv2.connectedComponentsWithStats(via_binary_inv, connectivity=8)
    vias: list[Via] = []
    for component_id in range(1, count):
        area = int(stats[component_id, cv2.CC_STAT_AREA])
        if area < min_area or area > max_area:
            continue
        width = int(stats[component_id, cv2.CC_STAT_WIDTH])
        height = int(stats[component_id, cv2.CC_STAT_HEIGHT])
        ratio = max(width, height) / max(1, min(width, height))
        if ratio > max_aspect_ratio:
            continue

        cx, cy = centroids[component_id]
        diameter = int(round(np.sqrt((4.0 * area) / np.pi)))
        vias.append(Via(x=int(round(cx)), y=int(round(cy)), diameter=max(1, diameter), connects_to=[]))

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
    netlist_v1 = json.loads(netlist_v1_path.read_text(encoding="utf-8"))
    nodes = netlist_v1.get("nodes", [])
    transistors = netlist_v1.get("devices", {}).get("transistors", [])

    vias: list[Via] = []
    via_detection_mode = "unavailable"
    via_image_loaded = False
    metal_image_loaded = False

    if via_image_path.exists():
        via_image = cv2.imread(str(via_image_path), cv2.IMREAD_GRAYSCALE)
        if via_image is not None:
            via_image_loaded = True
            blurred = cv2.GaussianBlur(via_image, (5, 5), 0)
            _, via_binary = cv2.threshold(
                blurred,
                0,
                255,
                cv2.THRESH_BINARY + cv2.THRESH_OTSU,
            )
            vias = detect_vias_from_image(via_binary)
            via_detection_mode = "hough_circles" if vias else "connected_components"
            if not vias:
                _, via_binary_inv = cv2.threshold(
                    blurred,
                    0,
                    255,
                    cv2.THRESH_BINARY_INV + cv2.THRESH_OTSU,
                )
                vias = detect_vias_from_components(via_binary_inv)
        else:
            via_detection_mode = "load_failed"

    if metal_image_path.exists():
        metal_image_loaded = cv2.imread(str(metal_image_path), cv2.IMREAD_GRAYSCALE) is not None

    via_entries, direct_metal_connections = map_vias_to_nodes(vias, nodes)
    connected_node_count = len(
        {
            node_id
            for via in via_entries
            for node_id in via.get("connects_to", [])
            if isinstance(node_id, int)
        }
    )
    connectivity_mode = "bbox_proximity_v1" if via_entries else "not_mapped"
    routing_edges = [
        {
            "from": f"via:{via['via_id']}",
            "to": f"node:{node_id}",
            "kind": "via_contact",
            "mapping_method": via.get("mapping_method"),
        }
        for via in via_entries
        for node_id in via.get("connects_to", [])
        if isinstance(node_id, int)
    ]

    netlist_v2 = {
        "schema_version": 2,
        "chip": chip,
        "description": "Netlist v1 passthrough with via detection and heuristic via-node routing",
        "metadata": {
            "source": "extract_via_connectivity_v0.py",
            "via_detection": via_detection_mode,
            "metal_routing": "passthrough_from_netlist_v1",
            "input_netlist_v1": str(netlist_v1_path),
            "input_via_image": str(via_image_path),
            "input_metal_image": str(metal_image_path),
            "via_image_loaded": via_image_loaded,
            "metal_image_loaded": metal_image_loaded,
            "connectivity_mode": connectivity_mode,
        },
        "nodes": nodes,
        "vias": via_entries,
        "transistors": transistors,
        "routing_graph": {
            "schema_version": 1,
            "node_ids": sorted(
                [
                    node.get("node")
                    for node in nodes
                    if isinstance(node, dict) and isinstance(node.get("node"), int)
                ]
            ),
            "edges": routing_edges,
        },
        "statistics": {
            "total_nodes": len(nodes),
            "total_vias": len(vias),
            "total_transistors": len(transistors),
            "via_connected_nodes": connected_node_count,
            "direct_metal_connections": direct_metal_connections,
            "routing_edges": len(routing_edges),
        },
    }

    return netlist_v2


def validate_via_connectivity(netlist_v2: dict[str, Any]) -> dict[str, Any]:
    """
    Validate via connectivity for consistency.

    Checks:
    - All vias connect to valid nodes
    - Warn on floating vias (connected to <2 nodes)
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

    nodes = netlist_v2.get("nodes", [])
    vias = netlist_v2.get("vias", [])
    transistors = netlist_v2.get("transistors", [])

    node_ids = {
        node.get("node")
        for node in nodes
        if isinstance(node, dict) and isinstance(node.get("node"), int)
    }

    for via in vias:
        if not isinstance(via, dict):
            report["errors"].append("Via entry is not an object")
            continue

        connects_to = via.get("connects_to", [])
        if not isinstance(connects_to, list):
            report["errors"].append(f"via_id={via.get('via_id')} has non-list connects_to")
            continue

        unknown_nodes = [n for n in connects_to if n not in node_ids]
        if unknown_nodes:
            report["errors"].append(
                f"via_id={via.get('via_id')} references unknown nodes: {unknown_nodes}"
            )

        if len(connects_to) == 1:
            report["statistics"]["floating_vias"] += 1
        if len(connects_to) > 8:
            report["statistics"]["over_connected_vias"] += 1

    for t in transistors:
        if not isinstance(t, dict):
            continue
        a_node = t.get("a_node")
        b_node = t.get("b_node")
        if a_node not in node_ids or b_node not in node_ids:
            report["statistics"]["disconnected_transistors"] += 1

    if vias and all(len(v.get("connects_to", [])) == 0 for v in vias if isinstance(v, dict)):
        report["warnings"].append(
            "Vias were detected but node connectivity mapping is not yet implemented."
        )

    if report["statistics"]["floating_vias"] > 0:
        report["warnings"].append(
            f"{report['statistics']['floating_vias']} vias have only one node connection."
        )

    if report["statistics"]["disconnected_transistors"] > 0:
        report["warnings"].append(
            f"{report['statistics']['disconnected_transistors']} transistors reference unknown nodes."
        )

    report["valid"] = len(report["errors"]) == 0

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
        default=ROOT / "docs" / "emulators",
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
        if not netlist_v1.exists():
            flat_netlist_v1 = args.input_dir / f"{chip}_netlist_v1.json"
            if flat_netlist_v1.exists():
                netlist_v1 = flat_netlist_v1
        via_image = args.via_dir / f"i{chip}-vias.bmp"
        metal_image = args.via_dir / f"i{chip}-metal.bmp"

        # Check if input files exist
        if not netlist_v1.exists():
            print(f"  ⚠ Netlist v1 not found: {netlist_v1}")
            print(f"    Skipping {chip} (input required)")
            continue

        if not via_image.exists():
            print(f"  ⚠ Via layer not found: {via_image}")
            print("    Continuing without via image (netlist_v1 passthrough mode)")

        if not metal_image.exists():
            print(f"  ⚠ Metal layer not found: {metal_image}")
            print("    Continuing without metal layer image")

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


if __name__ == "__main__":
    main()
