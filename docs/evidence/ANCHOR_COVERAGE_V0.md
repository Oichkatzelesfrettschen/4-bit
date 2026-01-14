# Anchor coverage vs primary pinouts (v0)

- Anchors: `docs/evidence/schematic_layout_anchors_v1.json`
- Pinouts: `docs/evidence/PRIMARY_SOURCE_PINOUTS.md`

## 4001

| Signal | Anchor present | layout_node | layout_node_src |
|---|---:|---:|---:|
| `D0` (`D0→D0_PAD`) | yes | 34 | 0 |
| `D1` (`D1→D1_PAD`) | yes | 2599 | 1 |
| `D2` (`D2→D2_PAD`) | yes | 2618 | 3 |
| `D3` (`D3→D3_PAD`) | yes | 2647 | 4 |
| `VSS` | yes | 2570 | 4 |
| `SYNC` | yes | 1607 | 1536 |
| `RESET` | yes | 4563 | 2514 |
| `CL` | yes | 5457 | 2540 |
| `CMROM` (`CMROM→CM`) | yes | 5454 | 2522 |
| `VDD` | yes | 5657 | 2523 |
| `IO0` | yes | 2159 | 2539 |
| `IO1` | yes | 1955 | 2538 |
| `IO2` | yes | 5604 | 2519 |
| `IO3` | yes | 3253 | 111 |

## 4002

| Signal | Anchor present | layout_node | layout_node_src |
|---|---:|---:|---:|
| `D0` (`D0→D0_PAD`) | yes | 868 | 0 |
| `D1` (`D1→D1_PAD`) | yes | 872 | 1 |
| `D2` (`D2→D2_PAD`) | yes | 938 | 2 |
| `D3` (`D3→D3_PAD`) | yes | 896 | 3 |
| `VSS` | yes | 73 | 128 |
| `CLK1` | yes | 2259 | 292 |
| `CLK2` | yes | 2619 | 536 |
| `SYNC` | yes | 3261 | 819 |
| `RESET` | yes | 3225 | 818 |
| `P0` (`P0→CS`) | yes | 789 | 817 |
| `CM` | yes | 799 | 4 |
| `CMRAM` (`CMRAM→CM`) | yes | 799 | 4 |
| `VDD` | yes | 3251 | 816 |
| `OUT0` | yes | 3115 | 738 |
| `OUT1` | yes | 2855 | 672 |
| `OUT2` | yes | 2605 | 320 |
| `OUT3` | yes | 1685 | 41 |

## 4003

| Signal | Anchor present | layout_node | layout_node_src |
|---|---:|---:|---:|
| `CP` (`CP→CLOCK`) | yes | 239 | 0 |
| `DATAIN` (`DATAIN→DATA`) | yes | 235 | 1 |
| `O0` | no |  |  |
| `O1` | no |  |  |
| `O2` | no |  |  |
| `O3` | no |  |  |
| `O4` | no |  |  |
| `O5` | no |  |  |
| `O6` | no |  |  |
| `O7` | no |  |  |
| `O8` | no |  |  |
| `O9` | no |  |  |
| `E` (`E→EN`) | yes | 279 | 19 |
| `VSS` | yes | 152 | 5 |
| `VDD` | yes | 359 | 129 |
| `CLOCK` | yes | 239 | 0 |
| `DATA` | yes | 235 | 1 |
| `EN` | yes | 279 | 19 |
| `OUT` | yes | 352 | 109 |

## 4004

| Signal | Anchor present | layout_node | layout_node_src |
|---|---:|---:|---:|
| `D0` (`D0→D0_PAD`) | yes | 3442 | 3442 |
| `D1` (`D1→D1_PAD`) | yes | 598 | 598 |
| `D2` (`D2→D2_PAD`) | yes | 3426 | 3426 |
| `D3` (`D3→D3_PAD`) | yes | 2815 | 2815 |
| `VSS` | yes | 3 | 3 |
| `CLK1` | yes | 415 | 415 |
| `CLK2` | yes | 1230 | 1230 |
| `SYNC` | yes | 1261 | 2 |
| `RESET` | yes | 1232 | 1 |
| `TEST` | yes | 1267 | 0 |
| `CMROM` | yes | 716 | 71 |
| `VCC` | yes | 415 | 415 |
| `CMRAM0` | yes | 3431 | 3431 |
| `CMRAM1` | yes | 3428 | 3428 |
| `CMRAM2` | yes | 2997 | 2997 |
| `CMRAM3` | yes | 405 | 441 |

