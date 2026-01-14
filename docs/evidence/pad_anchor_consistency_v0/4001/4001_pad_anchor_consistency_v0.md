# 4001 pad↔anchor consistency (v0)

Inputs:
- anchors: `docs/evidence/schematic_layout_anchors_v1.json`
- netlist_v0: `docs/evidence/netlists_v0/4001_netlist_v0.json`
- layout_pads_v0: `docs/evidence/layout_pads_v0/4001/4001_layout_pads_v0.json`

## Anchor→pad mapping

| Anchor | layout_node_src | pad_idx_perimeter_ccw | method | score |
|---|---:|---:|---|---:|
| `CL` | 2540 | 9 | suggested_nodes | 0.979118 |
| `CLK1` | 13 | 5 | suggested_nodes | 35.997967 |
| `CLK2` | 14 | 5 | suggested_nodes | 46.862979 |
| `CM` | 2522 | 10 | suggested_nodes | 12.783584 |
| `D0_PAD` | 0 | 0 | suggested_nodes | 1.102773 |
| `D1_PAD` | 1 | 1 | suggested_nodes | 1.433377 |
| `D2_PAD` | 3 | 3 | suggested_nodes | 20.30649 |
| `D3_PAD` | 4 | 1 | suggested_nodes | 26.074248 |
| `IO0` | 2539 | 12 | suggested_nodes | 1.054671 |
| `IO1` | 2538 | 13 | suggested_nodes | 1.010746 |
| `IO2` | 2519 | 14 | suggested_nodes | 0.835874 |
| `IO3` | 111 | 15 | suggested_nodes | 0.874947 |
| `RESET` | 2514 | 8 | suggested_nodes | 20.153909 |
| `SYNC` | 1536 | 7 | suggested_nodes | 28.709667 |
| `VDD` | 2523 | 11 | suggested_nodes | 7.546899 |
| `VSS` | 4 | 1 | suggested_nodes | 26.074248 |

## Pad→anchors (collisions)

| pad_idx_perimeter_ccw | anchors |
|---:|---|
| 1 | `D1_PAD`, `D3_PAD`, `VSS` |
| 5 | `CLK1`, `CLK2` |

