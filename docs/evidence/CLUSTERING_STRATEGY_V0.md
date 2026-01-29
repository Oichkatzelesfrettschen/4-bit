# Hierarchical Clustering Strategy for Subcircuits (v0)

**Date**: 2026-01-29
**Status**: DESIGN DOCUMENT
**Target**: Phase 4 implementation

---

## Executive Summary

This document defines the hierarchical clustering strategy for grouping extracted transistor subcircuits into functional blocks. The goal is to enable multi-scale simulation (transistor <-> gate <-> RTL) and improve understanding of chip architecture through automated functional block identification.

---

## Research Findings

### Subcircuit Characteristics (Current State)

| Chip | Subcircuits | Transistor Range | Max Nodes | Functional Types |
|------|-------------|------------------|-----------|------------------|
| 4001 | 11 | 1-117 | 150 | PAD, CLK, CM, IO, RESET, SYNC |
| 4002 | 6 | 1-42 | 85 | PAD, OUT, CUSTOM |
| 4003 | 5 | 1-9 | 21 | OUT, CLOCK, EN, DATA |
| 4004 | 11 | up to 437 | 680 | PAD, CLK, CM (ROM/RAM) |

**Key Observations**:
1. Size varies dramatically (1-437 transistors)
2. All subcircuits extracted with BFS radius=3
3. Spatial information available (transistor bboxes)
4. Node connectivity explicit (shared node IDs)
5. Functional categories already partially encoded in names

---

## Clustering Strategies

### Strategy 1: Spatial Clustering (Geometric)

**Objective**: Group subcircuits by physical proximity on die

**Algorithm**:
1. Extract centroid for each subcircuit (average bbox positions)
2. Build distance matrix between all subcircuit centroids
3. Apply agglomerative clustering with distance threshold
4. Merge subcircuits within threshold distance

**Distance Threshold**:
- Small chips (4002/4003): 500 pixels
- Medium chips (4001): 1000 pixels
- Large chips (4004): 2000 pixels

**Advantages**:
- Respects physical chip layout
- Groups related circuits that are physically adjacent
- No semantic knowledge required

**Limitations**:
- May split functionally related but physically distant circuits
- Threshold tuning required per chip

**Use Case**: Initial grouping before semantic refinement

---

### Strategy 2: Functional Clustering (Semantic)

**Objective**: Group subcircuits by functional role (inputs, outputs, control, data)

**Algorithm**:
1. Extract anchor type from subcircuit name/seed
2. Define functional categories:
   - INPUT_BUFFERS: D*_PAD (data pads)
   - OUTPUT_DRIVERS: OUT*, IO*
   - CLOCK_GENERATION: CLK*, CLOCK
   - CONTROL_SIGNALS: CM*, CS*, CL, RESET, SYNC, EN
   - POWER_RAILS: VDD, VSS
   - CUSTOM: unanchored/miscellaneous
3. Group all subcircuits with matching category
4. Within category, optionally merge by size/connectivity

**Functional Categories**:
```
INPUT_BUFFERS:     D*_PAD, DATA
OUTPUT_DRIVERS:    OUT*, IO*
CLOCK_GENERATION:  CLK*, CLOCK
CONTROL_SIGNALS:   CM*, CS*, CL, RESET, SYNC, EN
POWER_RAILS:       VDD, VSS
CUSTOM:            custom, unanchored
```

**Advantages**:
- Natural grouping by chip function
- Aligns with datasheet organization
- Easy to interpret and validate

**Limitations**:
- Requires semantic knowledge of anchor naming
- May create very large clusters (e.g., all PAD circuits)
- Doesn't account for physical layout

**Use Case**: High-level functional block identification

---

### Strategy 3: Electrical Clustering (Connectivity)

**Objective**: Group subcircuits by shared electrical connectivity

**Algorithm**:
1. Build node connectivity graph
2. For each pair of subcircuits:
   - Count shared nodes: intersection(subA.nodes, subB.nodes)
   - Compute overlap ratio: shared / min(len(subA.nodes), len(subB.nodes))
3. Merge subcircuits with overlap ratio > threshold (default: 0.5 = 50%)
4. Iteratively merge until no more merges possible

**Overlap Threshold**:
- Conservative: 0.7 (70% shared nodes)
- Balanced: 0.5 (50% shared nodes) [RECOMMENDED]
- Aggressive: 0.3 (30% shared nodes)

**Advantages**:
- Reflects actual electrical connectivity
- Groups tightly coupled circuits
- Data-driven (no manual categorization)

**Limitations**:
- May create very large clusters if threshold too low
- Sensitive to threshold parameter
- Doesn't respect functional boundaries

**Use Case**: Fine-grained clustering within functional blocks

---

## Hierarchical Clustering Pipeline

### Three-Level Hierarchy

**Level 0**: Individual subcircuits (as extracted, BFS radius=3)
**Level 1**: Electrical clusters (merge by connectivity, threshold=0.5)
**Level 2**: Functional blocks (merge Level 1 by functional category)
**Level 3**: Spatial regions (optional, merge Level 2 by physical proximity)

### Recommended Pipeline

```
Step 1: Load all subcircuits for chip
Step 2: Apply electrical clustering (Strategy 3, threshold=0.5)
        - Produces Level 1 clusters
Step 3: Apply functional grouping (Strategy 2)
        - Produces Level 2 clusters (functional blocks)
Step 4: Optionally apply spatial grouping (Strategy 1)
        - Produces Level 3 clusters (chip regions)
Step 5: Validate no overlaps, complete coverage
Step 6: Output cluster hierarchy JSON
```

---

## Cluster Output Format

### Cluster JSON Structure

```json
{
  "schema_version": 0,
  "chip": "4001",
  "description": "Hierarchical subcircuit clustering",
  "hierarchy": {
    "level_0": {
      "description": "Individual subcircuits (as extracted)",
      "clusters": [
        {
          "id": "4001_CL_L0",
          "name": "CL",
          "transistor_count": 6,
          "node_count": 9,
          "subcircuits": ["4001_CL_subcircuit_v0.json"]
        },
        ...
      ]
    },
    "level_1": {
      "description": "Electrical connectivity clusters (threshold=0.5)",
      "clusters": [
        {
          "id": "4001_CLOCK_L1",
          "name": "CLOCK_GROUP",
          "transistor_count": 74,
          "node_count": 110,
          "subcircuits": ["4001_CLK1_subcircuit_v0.json", "4001_CLK2_subcircuit_v0.json"],
          "merge_reason": "electrical_overlap_0.63"
        },
        ...
      ]
    },
    "level_2": {
      "description": "Functional blocks",
      "clusters": [
        {
          "id": "4001_CLOCK_GEN_L2",
          "name": "CLOCK_GENERATION",
          "transistor_count": 74,
          "node_count": 110,
          "level_1_clusters": ["4001_CLOCK_L1"],
          "functional_category": "CLOCK_GENERATION"
        },
        ...
      ]
    }
  },
  "statistics": {
    "total_subcircuits": 11,
    "level_1_clusters": 8,
    "level_2_clusters": 5,
    "coverage_check": {
      "all_subcircuits_assigned": true,
      "no_overlaps": true
    }
  }
}
```

---

## Validation Criteria

### Coverage Validation
- All subcircuits must appear in exactly one Level 0 cluster
- All Level 0 clusters must appear in at least one Level 1 cluster
- No transistor/node should be counted multiple times

### Overlap Validation
- No two clusters at same level should share transistors
- Clusters may share boundary nodes (electrical connections)
- Flag warning if >10% node overlap between clusters

### Size Validation
- Flag clusters with >80% of chip transistors (too coarse)
- Flag clusters with <3 transistors (too fine)
- Recommend re-tuning thresholds if validation fails

---

## Implementation Plan

### Phase 4 Task 26: cluster_subcircuits_v0.py

**Inputs**:
- docs/evidence/subcircuits_v0/{chip}/{chip}/\*.json
- docs/evidence/subcircuits_v0/{chip}/manifest.json

**Outputs**:
- docs/evidence/clusters_v0/{chip}/{chip}_clusters_v0.json
- docs/evidence/clusters_v0/{chip}/metrics.json
- docs/evidence/clusters_v0/{chip}/metrics.md

**Implementation Steps**:
1. Load all subcircuits for chip from manifest
2. Extract spatial, functional, and connectivity features
3. Apply electrical clustering (Level 1)
4. Apply functional grouping (Level 2)
5. Validate coverage and overlap
6. Generate cluster JSON and metrics

**Validation**:
- All subcircuits assigned to clusters
- No overlaps detected
- Cluster statistics reasonable (size distribution)

---

## Tuning Parameters

### Global Parameters
```python
ELECTRICAL_OVERLAP_THRESHOLD = 0.5  # 50% shared nodes
SPATIAL_DISTANCE_THRESHOLD = {
    "4001": 1000,  # pixels
    "4002": 500,
    "4003": 500,
    "4004": 2000,
}
MIN_CLUSTER_SIZE = 3  # transistors
MAX_CLUSTER_SIZE_RATIO = 0.8  # 80% of chip
```

### Per-Chip Overrides
Allow per-chip JSON config:
```json
{
  "4004": {
    "electrical_threshold": 0.4,
    "spatial_threshold": 1500,
    "functional_categories": {
      "CUSTOM": ["custom", "CMRAM0", "CMROM"]
    }
  }
}
```

---

## Future Enhancements

### Phase 5+ Improvements
1. Machine learning clustering (train on labeled examples)
2. Graph-based community detection (Louvain, spectral)
3. Multi-scale simulation with cluster boundaries
4. Interactive cluster refinement (GUI tool)
5. Cross-chip cluster pattern detection

---

## References

- [Phase 4 Plan](../ROADMAP.md#phase-4-clustering-and-performance)
- [Subcircuit Extraction](subcircuits_v0/README.md)
- [Netlist Format v1](netlists_v1/README.md)

---

**Author**: Claude Sonnet 4.5 (claude-sonnet-4-5-20250929)
**Date**: 2026-01-29
**Status**: DESIGN COMPLETE - READY FOR IMPLEMENTATION
