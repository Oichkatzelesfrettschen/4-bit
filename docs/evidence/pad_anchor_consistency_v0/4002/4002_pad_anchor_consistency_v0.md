# 4002 pad↔anchor consistency (v0)

Inputs:
- anchors: `docs/evidence/schematic_layout_anchors_v1.json`
- netlist_v0: `docs/evidence/netlists_v0/4002_netlist_v0.json`
- layout_pads_v0: `docs/evidence/layout_pads_v0/4002/4002_layout_pads_v0.json`

## Anchor→pad mapping

| Anchor | layout_node_src | pad_idx_perimeter_ccw | method | score |
|---|---:|---:|---|---:|
| `CLK1` | 292 | 5 | suggested_nodes | 1.77696 |
| `CLK2` | 536 | 6 | suggested_nodes | 13.531582 |
| `CM` | 4 | 3 | suggested_nodes | 131.402049 |
| `CS` | 817 | 9 | suggested_nodes | 0.9506 |
| `D0_PAD` | 0 | 0 | suggested_nodes | 5.376268 |
| `D1_PAD` | 1 | 1 | suggested_nodes | 4.431682 |
| `D2_PAD` | 2 | 2 | suggested_nodes | 1.789867 |
| `D3_PAD` | 3 | 3 | suggested_nodes | 3.720838 |
| `OUT0` | 738 | 12 | suggested_nodes | 0.962675 |
| `OUT1` | 672 | 13 | suggested_nodes | 0.714932 |
| `OUT2` | 320 | 14 | suggested_nodes | 4.339944 |
| `OUT3` | 41 | 15 | suggested_nodes | 4.416917 |
| `RESET` | 818 | 8 | suggested_nodes | 0.924893 |
| `SYNC` | 819 | 7 | suggested_nodes | 0.988356 |
| `VDD` | 816 | 11 | suggested_nodes | 1.032826 |
| `VSS` | 128 | 4 | suggested_nodes | 1.796187 |

## Pad→anchors (collisions)

| pad_idx_perimeter_ccw | anchors |
|---:|---|
| 3 | `CM`, `D3_PAD` |

