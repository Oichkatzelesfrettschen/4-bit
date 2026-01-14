# 4001 metal-mask pad label readings (manual, v0)

This file is a human-maintained index for converting metal-mask pad-label crops into anchors.
- Canonical crops: `docs/evidence/layout_pad_labels_v0/4001/human_crops/`

Important:

- On **4001**, the metal mask does **not** appear to print per-pin signal names in a way that is reliably OCR’able.
- Many “detections” in this folder are **die markings** (chip id, logo fragments) or pad/block shapes, not signal labels.
- For anchor mapping, prefer primary-source pinouts + geometry (see `docs/evidence/PRIMARY_SOURCE_PINOUTS.md`) and the cyclic angle-alignment seed suggestions in `docs/evidence/anchor_seed_suggestions_v0/4001_angle_alignment.json`.
- The `S` label cropped in `docs/evidence/layout_edge_labels_v0/4001/crops/001_S_node2540_conf0.0.png` aligns with the bottom pad bbox (Pad idx 9). `scripts/sync_layout_bboxes_from_edge_labels_v0.py --signal SYNC` now copies that bbox into `schematic_layout_anchors_v1.json`.

## Candidates

| idx | suggested_node | ocr_best | crop | printed_label | anchor_name | notes |
|---:|---:|---|---|---|---|---|
| 0 | 0 | `MN` | `docs/evidence/layout_pad_labels_v0/4001/human_crops/box_000_node_0.png` | MN |  | AUTO_OCR_V1 conf=32.0 psm=7 inv=1 scale=2 |
| 1 | 1 | `I.` | `docs/evidence/layout_pad_labels_v0/4001/human_crops/box_001_node_1.png` | I. |  | AUTO_OCR_V1 conf=23.0 psm=8 inv=0 scale=3 |
| 2 | 27 | `4` | `docs/evidence/layout_pad_labels_v0/4001/human_crops/box_002_node_27.png` | 4 |  | AUTO_OCR_V1 conf=57.0 psm=11 inv=0 scale=2 |
| 3 | 111 | `OL` | `docs/evidence/layout_pad_labels_v0/4001/human_crops/box_003_node_111.png` | OL |  | AUTO_OCR_V1 conf=77.0 psm=8 inv=0 scale=3 |
| 4 | 2519 | `A` | `docs/evidence/layout_pad_labels_v0/4001/human_crops/box_004_node_2519.png` | A |  | AUTO_OCR_V1 conf=75.0 psm=11 inv=0 scale=4 |
| 5 | 2523 | `_LDAYFS_NODS/_______________________M` | `docs/evidence/layout_pad_labels_v0/4001/human_crops/box_005_node_2523.png` | _LDAYFS_NODS/_______________________M |  | AUTO_OCR_V1 conf=0.0 psm=11 inv=0 scale=3 |
| 6 | 2524 | `40(` | `docs/evidence/layout_pad_labels_v0/4001/human_crops/box_006_node_2524.png` | 40( |  | AUTO_OCR_V1 conf=62.0 psm=6 inv=0 scale=2 |
| 7 | 2525 | `NS&-EATATET-FWAWA4001` | `docs/evidence/layout_pad_labels_v0/4001/human_crops/box_007_node_2525.png` | NS&-EATATET-FWAWA4001 |  | AUTO_OCR_V1 conf=26.3 psm=11 inv=0 scale=4 |
| 8 | 2526 | `L_______________________________________]BATAVW&2WATATEI-FPAWI___________________________________]___________________________________]1001` | `docs/evidence/layout_pad_labels_v0/4001/human_crops/box_008_node_2526.png` | L_______________________________________]BATAVW&2WATATEI-FPAWI___________________________________]___________________________________]1001 |  | AUTO_OCR_V1 conf=16.2 psm=11 inv=0 scale=4 |
| 9 | 2538 | `L___________________________________________________]VAN1.NN` | `docs/evidence/layout_pad_labels_v0/4001/human_crops/box_009_node_2538.png` | L___________________________________________________]VAN1.NN |  | AUTO_OCR_V1 conf=0.0 psm=11 inv=0 scale=4 |
| 10 | 2539 | `PE` | `docs/evidence/layout_pad_labels_v0/4001/human_crops/box_010_node_2539.png` | PE |  | AUTO_OCR_V1 conf=45.0 psm=7 inv=0 scale=4 |
| 11 | 2540 | `ELL_NANGJ~ARST______________________________]` | `docs/evidence/layout_pad_labels_v0/4001/human_crops/box_011_node_2540.png` | ELL_NANGJ~ARST______________________________] |  | AUTO_OCR_V1 conf=0.0 psm=11 inv=0 scale=2 |

## Current anchor mapping (v1)

Source of truth: `docs/evidence/schematic_layout_anchors_v1.json`.

Important:

- `layout_node_src` is the **pad-metal seed** (pad candidate / bbox hint).
- `layout_node` is the **remapped incident node** (used to seed subcircuit extraction).
- Earlier revisions used cyclic angle-alignment suggestions (`docs/evidence/anchor_seed_suggestions_v0/4001_angle_alignment.json`) to pick `layout_node_src` seeds; those pre-remap node ids are **not** stable and should not be treated as current anchors.

| signal | layout_node | layout_node_src |
|---|---:|---:|
| `CLK1` | 242 | 13 |
| `CLK2` | 3183 | 14 |
| `SYNC` | 1607 | 1536 |
| `RESET` | 4563 | 2514 |
| `CL` | 5457 | 2540 |
| `CM` | 5454 | 2522 |
| `VDD` | 5657 | 2523 |
| `VSS` | 2570 | 4 |
| `D0_PAD` | 34 | 0 |
| `D1_PAD` | 2599 | 1 |
| `D2_PAD` | 2618 | 3 |
| `D3_PAD` | 2647 | 4 |
| `IO0` | 2159 | 2539 |
| `IO1` | 1955 | 2538 |
| `IO2` | 5604 | 2519 |
| `IO3` | 3253 | 111 |

## Bond-pad candidates (from metal mask)

Source: `docs/evidence/layout_pads_v0/4001/4001_layout_pads_v0.json` (tuned: open-kernel=19, min_bbox_area=9000).
These are *pad-like* metal blocks near the periphery; they are the right starting point for mapping the remaining external pins when the mask itself lacks per-pin text labels.

| pad_idx | bbox (x0,y0,x1,y1) | edge | suggested_pad_node | edge_label | anchor_name | notes |
|---:|---|---|---:|---|---|---|
| 0 | (194,60,333,149) | top | 0 |  | D0_PAD | Stable pad-metal seed (`layout_node_src=0`). |
| 1 | (636,60,774,154) | top | 1 |  | D1_PAD | Stable pad-metal seed (`layout_node_src=1`). |
| 2 | (1100,89,1246,186) | top | 3 |  | D2_PAD | Stable pad-metal seed (`layout_node_src=3`). |
| 3 | (1587,89,1735,178) | top | 4 |  | D3_PAD | Stable pad-metal seed (`layout_node_src=4`). |
| 4 | (2097,90,2248,179) | top | 4 |  | VSS | Shares pad-metal seed with `D3_PAD` (`layout_node_src=4`); treat `VSS` as low-confidence until corroborated. |
| 5 | (2566,296,2652,419) | right | 13 |  | CLK1 | Stable pad-metal seed (`layout_node_src=13`). |
| 6 | (2566,649,2653,761) | right | 14 |  | CLK2 | Stable pad-metal seed (`layout_node_src=14`). |
| 7 | (2565,1135,2651,1270) | right | 1536 |  | SYNC | Stable pad-metal seed (`layout_node_src=1536`). |
| 8 | (2555,1436,2640,1592) | right | 2514 |  | RESET | Stable pad-metal seed (`layout_node_src=2514`). |
| 9 | (2179,1659,2425,1768) | bottom | 2540 | S | CL | Stable pad-metal seed (`layout_node_src=2540`); token `S` is ambiguous on 4001 and not treated as `SYNC`, but the crop above is reused to copy `layout_bbox` for `SYNC`. |
| 10 | (1995,1587,2097,1708) | bottom | 2522 |  | CM | Stable pad-metal seed (`layout_node_src=2522`). |
| 11 | (1620,1584,1723,1708) | bottom | 2523 |  | VDD | Stable pad-metal seed (`layout_node_src=2523`); treat as low-confidence until corroborated. |
| 12 | (770,1620,935,1708) | bottom | 2539 |  | IO0 | Stable pad-metal seed (`layout_node_src=2539`). |
| 13 | (435,1620,582,1709) | bottom | 2538 |  | IO1 | Stable pad-metal seed (`layout_node_src=2538`). |
| 14 | (62,1483,159,1603) | left | 2519 | C | IO2 | Stable pad-metal seed (`layout_node_src=2519`); token `C` is ambiguous on 4001 and not treated as `CM`. |
| 15 | (105,405,191,567) | left | 111 | V | IO3 | Stable pad-metal seed (`layout_node_src=111`); token `V` may be supply-related and is treated as untrusted OCR. |
