# 4004 metal-mask pad label readings (manual, v0)

This file is a human-maintained index for converting metal-mask pad-label crops into
anchors in `docs/evidence/schematic_layout_anchors_v0.json`.

The canonical crops live in `docs/evidence/layout_pad_labels_v0/4004/human_crops/`.

## Confirmed labels

| Printed label | Layout node | Crop |
|---|---:|---|
| `T` | 0 | `docs/evidence/layout_pad_labels_v0/4004/human_crops/box_000_node_0.png` |
| `02` | 17 | `docs/evidence/layout_pad_labels_v0/4004/human_crops/box_001_node_17.png` |
| `01` | 49 | `docs/evidence/layout_pad_labels_v0/4004/human_crops/box_016_node_49.png` |
| `D3` | 432 | `docs/evidence/layout_pad_labels_v0/4004/human_crops/box_067_node_432.png` |
| `D2` | 597 | `docs/evidence/layout_pad_labels_v0/4004/human_crops/box_102_node_597.png` |
| `D1` | 598 | `docs/evidence/layout_pad_labels_v0/4004/human_crops/box_103_node_598.png` |
| `D0` | 592 | (pad bbox overlay) `docs/evidence/layout_pad_labels_v0/4004/human_crops/debug_bottom_strip_node_bboxes.png` |
| `R1` | 595 | `docs/evidence/layout_pad_labels_v0/4004/human_crops/box_100_node_595.png` (used as `CMRAM1`) |
| `R0` | 596 | `docs/evidence/layout_pad_labels_v0/4004/human_crops/box_101_node_596.png` (used as `CMRAM0`) |
| `R2` | 518 | `docs/evidence/layout_edge_labels_v0/4004/crops/000_R2_node518_conf95.0.png` (used as `CMRAM2`) |
| `R3` | 441 | `docs/evidence/layout_edge_labels_v0/4004/crops/001_R3_node441_conf95.0.png` (used as `CMRAM3`) |
| `RM` | 71 | (node bbox crop) `docs/evidence/layout_edge_labels_v0/4004/rm_node71_crop.png` (used as `CMROM`) |

## Open items (next)

- Confirm `RM` token via robust OCR (currently best-effort; crop is visually unambiguous).
- Determine whether printed `T` corresponds to `TEST_PAD` or a different package/pad marking.
- Identify `SYNC` and `POC_PAD` labels (likely single-letter `S` and `C` on top edge).
