# 4003 metal-mask pad label readings (manual, v0)

This file is a human-maintained index for converting metal-mask pad-label crops into anchors.
- Canonical crops: `docs/evidence/layout_pad_labels_v0/4003/human_crops/`

Important:

- On **4003**, most detected “labels” are **die markings** (`4003`, `intel`) or pad/block shapes rather than per-pin signal names.
- Anchor mapping for 4003 currently comes from geometry + incidence (see `docs/evidence/anchor_seed_suggestions_v0/4003_angle_alignment.json` and `docs/evidence/schematic_layout_anchors_v0.json`).

## Current anchor mapping (v1)

Source of truth: `docs/evidence/schematic_layout_anchors_v1.json`.

Notes:

- Primary-source pinout (Figure 4-21) names the 10 parallel outputs `O0..O9`; the repo uses `Q0..Q9` (see `docs/emulators/i4003-signals.txt`).
- `layout_node_src` is the pad-metal seed; `layout_node` is the remapped incident node (used to seed subcircuit extraction).

| signal | layout_node | layout_node_src |
|---|---:|---:|
| `CLOCK` | 239 | 0 |
| `DATA` | 235 | 1 |
| `EN` | 279 | 19 |
| `OUT` | 352 | 109 |
| `Q0` | 237 | 3 |
| `Q1` | 151 | 4 |
| `Q2` | 78 | 44 |
| `Q3` | 370 | 98 |
| `Q4` | 389 | 122 |
| `Q5` | 110 | 138 |
| `Q6` | 7 | 7 |
| `Q7` | 386 | 132 |
| `Q8` | 385 | 130 |
| `Q9` | 390 | 129 |
| `VSS` | 152 | 5 |
| `VDD` | 359 | 129 |

## Candidates

| idx | suggested_node | ocr_best | crop | printed_label | anchor_name | notes |
|---:|---:|---|---|---|---|---|
| 0 | 8 | `LE` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_000_node_8.png` | LE |  | AUTO_OCR_V1 conf=68.0 psm=8 inv=1 scale=2 |
| 1 | 8 | `L]` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_001_node_8.png` | L] |  | AUTO_OCR_V1 conf=76.0 psm=8 inv=1 scale=4 |
| 2 | 8 | `HD` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_002_node_8.png` | HD |  | AUTO_OCR_V1 conf=90.0 psm=7 inv=0 scale=2 |
| 3 | 5 | `KJ` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_003_node_5.png` | KJ |  | AUTO_OCR_V1 conf=51.0 psm=8 inv=1 scale=4 |
| 4 | 16 | `K` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_004_node_16.png` | K |  | AUTO_OCR_V1 conf=73.0 psm=11 inv=1 scale=2 |
| 5 | 13 | `03` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_005_node_13.png` | 03 |  | AUTO_OCR_V1 conf=93.0 psm=11 inv=1 scale=2 |
| 6 | 10 | `A` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_006_node_10.png` | A |  | AUTO_OCR_V1 conf=74.0 psm=11 inv=1 scale=3 |
| 7 | 11 | `400` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_007_node_11.png` | 400 |  | AUTO_OCR_V1 conf=96.0 psm=6 inv=0 scale=3 |
| 8 | 12 | `003` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_008_node_12.png` | 003 |  | AUTO_OCR_V1 conf=96.0 psm=8 inv=0 scale=2 |
| 9 | 14 | `1K` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_009_node_14.png` | 1K |  | AUTO_OCR_V1 conf=66.0 psm=11 inv=0 scale=4 |
| 10 | 19 | `FA` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_010_node_19.png` | FA |  | AUTO_OCR_V1 conf=73.0 psm=8 inv=1 scale=3 |
| 11 | 41 | `A` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_011_node_41.png` | A |  | AUTO_OCR_V1 conf=50.0 psm=6 inv=0 scale=3 |
| 12 | 44 | `R` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_012_node_44.png` | R |  | AUTO_OCR_V1 conf=76.0 psm=7 inv=0 scale=4 |
| 13 | 122 | `IN` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_013_node_122.png` | IN |  | AUTO_OCR_V1 conf=93.0 psm=8 inv=1 scale=4 |
| 14 | 122 | `IR` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_014_node_122.png` | IR |  | AUTO_OCR_V1 conf=89.0 psm=11 inv=1 scale=4 |
| 15 | 131 | `TH` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_015_node_131.png` | TH |  | AUTO_OCR_V1 conf=78.0 psm=6 inv=0 scale=3 |
| 16 | 132 | `CR` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_016_node_132.png` | CR |  | AUTO_OCR_V1 conf=61.0 psm=8 inv=1 scale=2 |
| 17 | 7 | `LR` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_017_node_7.png` | LR |  | AUTO_OCR_V1 conf=57.0 psm=6 inv=0 scale=3 |
| 18 | 7 | `JR` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_018_node_7.png` | JR |  | AUTO_OCR_V1 conf=59.0 psm=7 inv=0 scale=4 |
| 19 | 135 | `TEL` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_019_node_135.png` | TEL |  | AUTO_OCR_V1 conf=59.0 psm=8 inv=0 scale=4 |
| 20 | 137 | `INTTM` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_020_node_137.png` | INTTM |  | AUTO_OCR_V1 conf=91.0 psm=11 inv=0 scale=3 |
| 21 | 138 | `1` | `docs/evidence/layout_pad_labels_v0/4003/human_crops/box_021_node_138.png` | 1 |  | AUTO_OCR_V1 conf=15.0 psm=7 inv=1 scale=2 |
