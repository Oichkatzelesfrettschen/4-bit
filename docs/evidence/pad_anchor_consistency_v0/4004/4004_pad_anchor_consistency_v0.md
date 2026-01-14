# 4004 pad↔anchor consistency (v0)

Inputs:
- anchors: `docs/evidence/schematic_layout_anchors_v1.json`
- netlist_v0: `docs/evidence/netlists_v0/4004_netlist_v0.json`
- layout_pads_v0: `docs/evidence/layout_pads_v0/4004/4004_layout_pads_v0.json`

## Anchor→pad mapping

| Anchor | layout_node_src | pad_idx_perimeter_ccw | method | score |
|---|---:|---:|---|---:|
| `CLK1` | 415 | 6 | suggested_nodes | 115.627577 |
| `CLK2` | 1230 | 3 | bbox_distance | -254.0 |
| `CMRAM0` | 3431 | 11 | bbox_distance | -37.0 |
| `CMRAM1` | 3428 | 12 | bbox_distance | -62.0 |
| `CMRAM2` | 2997 | 13 | bbox_distance | -186.63467 |
| `CMRAM3` | 441 | 14 | suggested_nodes | 9.054918 |
| `CMROM` | 71 | 15 | suggested_nodes | 5.189697 |
| `D0_PAD` | 3442 | 9 | bbox_distance | -62.5 |
| `D1_PAD` | 598 | 9 | bbox_distance | -128.0 |
| `D2_PAD` | 3426 | 8 | bbox_distance | -120.080182 |
| `D3_PAD` | 2815 | 7 | bbox_distance | -89.5 |
| `POC` | 1 | 1 | suggested_nodes | 2.977485 |
| `POC_PAD` | 1 | 1 | suggested_nodes | 2.977485 |
| `RESET` | 1 | 1 | suggested_nodes | 2.977485 |
| `SYNC` | 2 | 2 | suggested_nodes | 22.719339 |
| `TEST` | 0 | 0 | suggested_nodes | 0.849984 |
| `TEST_PAD` | 0 | 0 | suggested_nodes | 0.849984 |
| `VCC` | 415 | 6 | suggested_nodes | 115.627577 |
| `VSS` | 3 | 12 | suggested_nodes | 125.867101 |

## Pad→anchors (collisions)

| pad_idx_perimeter_ccw | anchors |
|---:|---|
| 0 | `TEST`, `TEST_PAD` |
| 1 | `POC`, `POC_PAD`, `RESET` |
| 6 | `CLK1`, `VCC` |
| 9 | `D0_PAD`, `D1_PAD` |
| 12 | `CMRAM1`, `VSS` |

