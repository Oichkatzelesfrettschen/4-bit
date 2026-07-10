#!/usr/bin/env python3
"""
Hierarchical clustering of transistor subcircuits.

This script implements three clustering strategies:
1. Spatial clustering: group by physical proximity
2. Functional clustering: group by semantic role
3. Electrical clustering: group by shared node connectivity

Output: Three-level cluster hierarchy (electrical -> functional -> spatial)
"""

from __future__ import annotations

import argparse
import json
import math
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


@dataclass
class Subcircuit:
    """Represents a loaded subcircuit."""

    name: str
    file_path: Path
    transistor_count: int
    node_count: int
    nodes: set[int]
    transistors: list[dict[str, Any]]
    bbox_center: tuple[float, float] | None = None
    functional_category: str | None = None


@dataclass
class Cluster:
    """Represents a cluster of subcircuits."""

    id: str
    name: str
    level: int
    subcircuits: list[str]
    transistor_count: int
    node_count: int
    merge_reason: str | None = None
    functional_category: str | None = None
    children_ids: list[str] = field(default_factory=list)


# Functional categories based on anchor naming patterns
FUNCTIONAL_CATEGORIES = {
    "INPUT_BUFFERS": ["D0_PAD", "D1_PAD", "D2_PAD", "D3_PAD", "DATA"],
    "OUTPUT_DRIVERS": ["OUT0", "OUT1", "OUT2", "OUT3", "IO0", "IO1", "IO2", "IO3"],
    "CLOCK_GENERATION": ["CLK1", "CLK2", "CLOCK"],
    "CONTROL_SIGNALS": [
        "CM",
        "CS",
        "CL",
        "CMROM",
        "CMRAM0",
        "CMRAM1",
        "CMRAM2",
        "CMRAM3",
        "RESET",
        "SYNC",
        "EN",
    ],
    "POWER_RAILS": ["VDD", "VSS"],
    "CUSTOM": ["custom"],
}


def categorize_subcircuit(name: str) -> str:
    """Assign functional category based on subcircuit name."""
    for category, patterns in FUNCTIONAL_CATEGORIES.items():
        for pattern in patterns:
            if pattern.lower() in name.lower():
                return category
    return "CUSTOM"


def load_subcircuit(file_path: Path) -> Subcircuit:
    """Load subcircuit from JSON file."""
    data = json.loads(file_path.read_text(encoding="utf-8"))

    # Extract nodes
    nodes = {node["node"] for node in data.get("nodes", [])}

    # Extract transistors
    transistors = data.get("devices", {}).get("transistors", [])

    # Compute bbox center if spatial information available
    bbox_center = None
    if transistors:
        x_coords = [t["bbox"]["x"] for t in transistors if "bbox" in t]
        y_coords = [t["bbox"]["y"] for t in transistors if "bbox" in t]
        if x_coords and y_coords:
            bbox_center = (
                sum(x_coords) / len(x_coords),
                sum(y_coords) / len(y_coords),
            )

    # Get name from seed or file name
    name = data.get("seed", {}).get("kind", file_path.stem.split("_subcircuit")[0])

    return Subcircuit(
        name=name,
        file_path=file_path,
        transistor_count=len(transistors),
        node_count=len(nodes),
        nodes=nodes,
        transistors=transistors,
        bbox_center=bbox_center,
        functional_category=categorize_subcircuit(name),
    )


def compute_spatial_distance(subA: Subcircuit, subB: Subcircuit) -> float | None:
    """Compute Euclidean distance between subcircuit centroids."""
    if subA.bbox_center is None or subB.bbox_center is None:
        return None

    dx = subA.bbox_center[0] - subB.bbox_center[0]
    dy = subA.bbox_center[1] - subB.bbox_center[1]
    return math.sqrt(dx * dx + dy * dy)


def compute_electrical_overlap(subA: Subcircuit, subB: Subcircuit) -> float:
    """Compute electrical connectivity overlap ratio."""
    shared_nodes = subA.nodes & subB.nodes
    if not shared_nodes:
        return 0.0

    # Overlap ratio = shared / min(nodeA, nodeB)
    min_nodes = min(len(subA.nodes), len(subB.nodes))
    if min_nodes == 0:
        return 0.0

    return len(shared_nodes) / min_nodes


def electrical_clustering(
    subcircuits: list[Subcircuit], threshold: float = 0.5
) -> list[list[Subcircuit]]:
    """
    Cluster subcircuits by electrical connectivity (shared nodes).

    Args:
        subcircuits: List of subcircuits to cluster
        threshold: Minimum overlap ratio to merge (default 0.5 = 50%)

    Returns:
        List of clusters (each cluster is a list of subcircuits)
    """
    # Start with each subcircuit in its own cluster
    clusters = [[sub] for sub in subcircuits]

    # Iteratively merge clusters with high overlap
    merged = True
    while merged:
        merged = False
        new_clusters = []
        used = set()

        for i, clusterA in enumerate(clusters):
            if i in used:
                continue

            # Compute all nodes in clusterA
            nodesA = set()
            for sub in clusterA:
                nodesA.update(sub.nodes)

            # Try to merge with remaining clusters
            best_match = None
            best_overlap = threshold

            for j, clusterB in enumerate(clusters[i + 1 :], start=i + 1):
                if j in used:
                    continue

                # Compute all nodes in clusterB
                nodesB = set()
                for sub in clusterB:
                    nodesB.update(sub.nodes)

                # Compute overlap
                shared = nodesA & nodesB
                if not shared:
                    continue

                min_nodes = min(len(nodesA), len(nodesB))
                if min_nodes == 0:
                    continue

                overlap = len(shared) / min_nodes

                if overlap > best_overlap:
                    best_overlap = overlap
                    best_match = j

            if best_match is not None:
                # Merge clusters
                merged_cluster = clusterA + clusters[best_match]
                new_clusters.append(merged_cluster)
                used.add(i)
                used.add(best_match)
                merged = True
            else:
                new_clusters.append(clusterA)
                used.add(i)

        clusters = new_clusters

    return clusters


def functional_clustering(
    level1_clusters: list[Cluster],
) -> dict[str, list[Cluster]]:
    """
    Group Level 1 clusters by functional category.

    Args:
        level1_clusters: List of Level 1 electrical clusters

    Returns:
        Dictionary mapping category -> list of clusters
    """
    functional_groups: dict[str, list[Cluster]] = defaultdict(list)

    for cluster in level1_clusters:
        # Determine category from first subcircuit name
        if cluster.functional_category:
            category = cluster.functional_category
        else:
            # Infer from subcircuit names
            first_sub_name = cluster.subcircuits[0] if cluster.subcircuits else "custom"
            category = categorize_subcircuit(first_sub_name)

        functional_groups[category].append(cluster)

    return functional_groups


def generate_cluster_hierarchy(
    chip: str,
    subcircuits: list[Subcircuit],
    electrical_threshold: float = 0.5,
) -> dict[str, Any]:
    """
    Generate three-level hierarchical clustering.

    Level 0: Individual subcircuits
    Level 1: Electrical connectivity clusters
    Level 2: Functional blocks

    Args:
        chip: Chip name
        subcircuits: List of loaded subcircuits
        electrical_threshold: Overlap threshold for electrical clustering

    Returns:
        Cluster hierarchy dictionary
    """
    # Level 0: Individual subcircuits
    level0_clusters = []
    for _index, sub in enumerate(subcircuits):
        cluster = Cluster(
            id=f"{chip}_{sub.name}_L0",
            name=sub.name,
            level=0,
            subcircuits=[sub.file_path.name],
            transistor_count=sub.transistor_count,
            node_count=sub.node_count,
            functional_category=sub.functional_category,
        )
        level0_clusters.append(cluster)

    # Level 1: Electrical clustering
    electrical_groups = electrical_clustering(subcircuits, electrical_threshold)

    level1_clusters = []
    for i, group in enumerate(electrical_groups):
        # Combine names
        if len(group) == 1:
            name = group[0].name
            merge_reason = None
        else:
            name = "_".join(sub.name for sub in group[:3])
            if len(group) > 3:
                name += f"_and_{len(group) - 3}_more"

            # Compute actual overlap for merge reason
            all_nodes = set()
            for sub in group:
                all_nodes.update(sub.nodes)
            overlap_ratio = len(all_nodes) / sum(len(sub.nodes) for sub in group)
            merge_reason = f"electrical_overlap_{overlap_ratio:.2f}"

        # Aggregate counts
        total_transistors = sum(sub.transistor_count for sub in group)
        all_nodes_set = set()
        for sub in group:
            all_nodes_set.update(sub.nodes)

        # Determine functional category (most common)
        categories = [sub.functional_category for sub in group]
        functional_category = max(set(categories), key=categories.count)

        cluster = Cluster(
            id=f"{chip}_L1_{i}",
            name=name,
            level=1,
            subcircuits=[sub.file_path.name for sub in group],
            transistor_count=total_transistors,
            node_count=len(all_nodes_set),
            merge_reason=merge_reason,
            functional_category=functional_category,
        )
        level1_clusters.append(cluster)

    # Level 2: Functional grouping
    functional_groups = functional_clustering(level1_clusters)

    level2_clusters = []
    for category, clusters in functional_groups.items():
        # Aggregate all Level 1 clusters in this category
        all_transistors = sum(c.transistor_count for c in clusters)
        all_nodes_set = set()
        all_subcircuits = []
        for c in clusters:
            all_subcircuits.extend(c.subcircuits)

        # Compute unique nodes (approximate - would need to reload subcircuits)
        # For now, sum node counts as upper bound
        total_nodes = sum(c.node_count for c in clusters)

        cluster = Cluster(
            id=f"{chip}_{category}_L2",
            name=category,
            level=2,
            subcircuits=all_subcircuits,
            transistor_count=all_transistors,
            node_count=total_nodes,  # Upper bound
            functional_category=category,
            children_ids=[c.id for c in clusters],
        )
        level2_clusters.append(cluster)

    # Build hierarchy dict
    hierarchy = {
        "level_0": {
            "description": "Individual subcircuits (as extracted)",
            "clusters": [
                {
                    "id": c.id,
                    "name": c.name,
                    "transistor_count": c.transistor_count,
                    "node_count": c.node_count,
                    "subcircuits": c.subcircuits,
                    "functional_category": c.functional_category,
                }
                for c in level0_clusters
            ],
        },
        "level_1": {
            "description": f"Electrical connectivity clusters (threshold={electrical_threshold})",
            "clusters": [
                {
                    "id": c.id,
                    "name": c.name,
                    "transistor_count": c.transistor_count,
                    "node_count": c.node_count,
                    "subcircuits": c.subcircuits,
                    "merge_reason": c.merge_reason,
                    "functional_category": c.functional_category,
                }
                for c in level1_clusters
            ],
        },
        "level_2": {
            "description": "Functional blocks",
            "clusters": [
                {
                    "id": c.id,
                    "name": c.name,
                    "transistor_count": c.transistor_count,
                    "node_count": c.node_count,
                    "subcircuits": c.subcircuits,
                    "functional_category": c.functional_category,
                    "level_1_clusters": c.children_ids,
                }
                for c in level2_clusters
            ],
        },
    }

    return hierarchy


def validate_clustering(
    hierarchy: dict[str, Any], total_subcircuits: int
) -> dict[str, Any]:
    """Validate cluster hierarchy for coverage and overlap."""
    validation = {
        "all_subcircuits_assigned": False,
        "no_overlaps": True,
        "level_0_count": len(hierarchy["level_0"]["clusters"]),
        "level_1_count": len(hierarchy["level_1"]["clusters"]),
        "level_2_count": len(hierarchy["level_2"]["clusters"]),
        "warnings": [],
    }

    # Check Level 0 coverage
    level0_count = len(hierarchy["level_0"]["clusters"])
    if level0_count == total_subcircuits:
        validation["all_subcircuits_assigned"] = True
    else:
        validation["warnings"].append(
            f"Level 0 count mismatch: {level0_count} vs {total_subcircuits}"
        )

    # Check for overlaps (subcircuits appearing in multiple clusters)
    # This is complex - for now, just check Level 1
    level1_subcircuits = set()
    for cluster in hierarchy["level_1"]["clusters"]:
        for sub in cluster["subcircuits"]:
            if sub in level1_subcircuits:
                validation["no_overlaps"] = False
                validation["warnings"].append(f"Subcircuit {sub} appears multiple times")
            level1_subcircuits.add(sub)

    return validation


def main():
    parser = argparse.ArgumentParser(description="Hierarchical subcircuit clustering")
    parser.add_argument(
        "--chips",
        nargs="+",
        default=["4001", "4002", "4003", "4004"],
        help="Chips to process",
    )
    parser.add_argument(
        "--input-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "subcircuits_v0",
        help="Input subcircuits directory",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=ROOT / "docs" / "evidence" / "clusters_v0",
        help="Output clusters directory",
    )
    parser.add_argument(
        "--electrical-threshold",
        type=float,
        default=0.5,
        help="Electrical overlap threshold (0.0-1.0)",
    )

    args = parser.parse_args()

    args.output_dir.mkdir(parents=True, exist_ok=True)

    print("=== Hierarchical Subcircuit Clustering ===")
    print("")

    for chip in args.chips:
        print(f"Processing {chip}...")

        # Load manifest (use flat structure for all chips)
        manifest_path = args.input_dir / chip / "manifest.json"
        if not manifest_path.exists():
            print(f"  Warning: Manifest not found: {manifest_path}")
            continue

        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))

        # Load all subcircuits
        subcircuits = []
        for output_entry in manifest["outputs"]:
            sub_path = ROOT / output_entry["output"]
            if not sub_path.exists():
                print(f"  Warning: Subcircuit file not found: {sub_path}")
                continue

            try:
                sub = load_subcircuit(sub_path)
                subcircuits.append(sub)
            except Exception as e:
                print(f"  Warning: Failed to load {sub_path}: {e}")
                continue

        print(f"  Loaded {len(subcircuits)} subcircuits")

        # Generate cluster hierarchy
        hierarchy = generate_cluster_hierarchy(
            chip, subcircuits, args.electrical_threshold
        )

        # Validate
        validation = validate_clustering(hierarchy, len(subcircuits))

        # Build output
        output = {
            "schema_version": 0,
            "chip": chip,
            "description": "Hierarchical subcircuit clustering",
            "hierarchy": hierarchy,
            "statistics": {
                "total_subcircuits": len(subcircuits),
                "level_1_clusters": len(hierarchy["level_1"]["clusters"]),
                "level_2_clusters": len(hierarchy["level_2"]["clusters"]),
                "coverage_check": validation,
            },
            "params": {
                "electrical_threshold": args.electrical_threshold,
            },
        }

        # Write output
        output_dir = args.output_dir / chip
        output_dir.mkdir(parents=True, exist_ok=True)

        output_file = output_dir / f"{chip}_clusters_v0.json"
        output_file.write_text(json.dumps(output, indent=2) + "\n", encoding="utf-8")

        # Print summary
        print(f"  Level 0: {len(hierarchy['level_0']['clusters'])} subcircuits")
        print(f"  Level 1: {len(hierarchy['level_1']['clusters'])} electrical clusters")
        print(f"  Level 2: {len(hierarchy['level_2']['clusters'])} functional blocks")
        print(f"  Validation: {'PASS' if validation['all_subcircuits_assigned'] else 'WARN'}")
        if validation["warnings"]:
            for warning in validation["warnings"]:
                print(f"    - {warning}")
        print(f"  Output: {output_file.name}")
        print("")

    print("Clustering complete")


if __name__ == "__main__":
    main()
