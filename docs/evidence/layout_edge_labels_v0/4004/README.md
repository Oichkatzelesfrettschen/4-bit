# Layout edge labels v0 (4004)

This folder contains **evidence artifacts** for mapping metal-mask periphery labels to `netlist_v0` node IDs.

## Files

- `4004_layout_edge_labels_v0.json`: machine-readable detections (OCR token + suggested node).
- `4004_layout_edge_labels_v0.png`: overlay of detected label blocks on the metal mask.
- `crops/`: OCR crops emitted for manual review.
- `rm_node71_crop.png`: manual crop of the `RM` pad label block (node bbox) used for the `CMROM` anchor.

## Notes

- `crops/` may include stale experimental outputs; prefer crops referenced from:
  - `docs/evidence/layout_pad_labels_v0/4004/manual_readings_v0.md`
  - `docs/evidence/schematic_layout_anchors_v0.json`

