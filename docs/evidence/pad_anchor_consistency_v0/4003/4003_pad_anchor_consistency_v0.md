# 4003 pad↔anchor consistency (v0)

Inputs:
- anchors: `docs/evidence/schematic_layout_anchors_v1.json`
- netlist_v0: `docs/evidence/netlists_v0/4003_netlist_v0.json`
- layout_pads_v0: `docs/evidence/layout_pads_v0/4003/4003_layout_pads_v0.json`

## Anchor→pad mapping

| Anchor | layout_node_src | pad_idx_perimeter_ccw | method | score |
|---|---:|---:|---|---:|
| `CLOCK` | 0 | 0 | suggested_nodes | 12.731048 |
| `DATA` | 1 | 1 | suggested_nodes | 10.999941 |
| `EN` | 19 | 15 | suggested_nodes | 6.312795 |
| `OUT` | 109 | 14 | suggested_nodes | 4.814593 |
| `Q0` | 3 | 2 | suggested_nodes | 3.507927 |
| `Q1` | 4 | 3 | suggested_nodes | 3.302461 |
| `Q2` | 44 | 5 | suggested_nodes | 1.29455 |
| `Q3` | 98 | 6 | suggested_nodes | 5.049607 |
| `Q4` | 122 | 7 | suggested_nodes | 3.763848 |
| `Q5` | 138 | 8 | suggested_nodes | 1.259659 |
| `Q6` | 7 | 8 | suggested_nodes | 67.672506 |
| `Q7` | 132 | 10 | suggested_nodes | 4.843292 |
| `Q8` | 130 | 11 | suggested_nodes | 9.474166 |
| `Q9` | 129 | 13 | suggested_nodes | 9.743798 |
| `VDD` | 129 | 13 | suggested_nodes | 9.743798 |
| `VSS` | 5 | 4 | suggested_nodes | 1.267682 |

## Pad→anchors (collisions)

| pad_idx_perimeter_ccw | anchors |
|---:|---|
| 8 | `Q5`, `Q6` |
| 13 | `Q9`, `VDD` |

