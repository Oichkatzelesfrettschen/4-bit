# Lacunae / open validation items (rolling)

This file tracks **what is still missing** for chip-accurate (and eventually transistor-/electron-accurate) reconstruction.

## Cross-cutting (all chips)

- **Schematic↔layout coordinate transforms**: `docs/emulators/i400x-signals.txt` coordinates are in schematic bitmap space and are **not directly comparable** to metal-mask pixel coordinates; each chip needs an explicit transform (homography/affine + validation).
  - **Status (2026-01-29)**: BLOCKED - requires manual identification of at least 4 corresponding anchor points per chip (same physical point in both schematic and layout coordinate systems). Script infrastructure exists (`scripts/build_coordinate_transform_v0.py`) but cannot compute homography without ground-truth correspondence data.
- **Power-rail anchoring**: `VSS/VDD/VCC` anchors exist in `docs/evidence/schematic_layout_anchors_v1.json`, with confidence now updated:
  - 4004 rails are tied to edge-label evidence (tokens `G`/`V`) and are treated as **evidence-backed**.
  - 4001-4003 rails are now **medium-confidence** (2026-01-14): pad geometry verified against PRIMARY_SOURCE_PINOUTS.md DIP pin positions. See `docs/evidence/POWER_RAIL_EVIDENCE.md`.
- **Pad label OCR limits**: metal-mask periphery rarely prints full signal names; short tokens (`S`, `T`, `R3`, `RM`, …) are ambiguous and must be validated against primary sources + geometry.

## 4001

- **VSS/VDD anchors**: (RESOLVED 2026-01-14) Now **medium-confidence** via pad geometry corroboration. See `docs/evidence/POWER_RAIL_EVIDENCE.md`.
- **Periphery pin mapping**: seeded via cyclic angle-alignment (`docs/evidence/anchor_seed_suggestions_v0/4001_angle_alignment.json`) then remapped to incident nodes; still needs validation once net segmentation improves (pads currently collapse onto shared trunks in some cases).
- **Subcircuits**: 11 subcircuits extracted with nonzero transistors (max 117). See `docs/evidence/subcircuits_v0/4001/metrics.md`.

## 4002

- **Chip-select (`P0`/`Po`) naming**: schematic files use `CS` while the primary pinout calls this `P0/Po`. Decide a canonical name and add an alias mapping.
- **Output pins naming**: schematic files use `OUT0..OUT3`; the primary pinout describes a 4-bit output port without a fixed bit labeling. Confirm `OUT0..OUT3` <-> DIP pins `13..16` via a primary diagram (or annotated schematic).
- **VSS/VDD anchors**: (RESOLVED 2026-01-14) Now **medium-confidence** via pad geometry corroboration. See `docs/evidence/POWER_RAIL_EVIDENCE.md`.
- **Subcircuits**: 6 subcircuits extracted with nonzero transistors (max 42). See `docs/evidence/subcircuits_v0/4002/metrics.md`.

## 4003

- **VDD pin evidence**: the MCS-40 User's Manual pinout (Figure 4-21) is partially text-extractable; the expected `VDD` pin is not currently captured by `pdftotext` and must be confirmed via OCR of the figure (or another primary source).
- **VSS/VDD anchors**: (RESOLVED 2026-01-14) Now **medium-confidence** via pad geometry corroboration. See `docs/evidence/POWER_RAIL_EVIDENCE.md`.
  - **Note**: VDD node discrepancy between historical POWER_RAIL_EVIDENCE.md (`layout_node=94`) and schematic_layout_anchors_v1.json (`node=359`) - the JSON value (359) is canonical.
- **Pad labels**: most detected "labels" are die markings (`4003`, `intel`), not per-pin labels; pad<->pin mapping remains human/geometry-driven.
- **O0-O9 vs Q0-Q9 naming**: (RESOLVED 2026-01-14) ANCHOR_COVERAGE_V0.md now documents Q0-Q9 = O0-O9 equivalence with explicit note.
- **Subcircuits**: 5 subcircuits extracted with nonzero transistors (max 9). See `docs/evidence/subcircuits_v0/4003/metrics.md`.

## 4004

- **RESET pin naming vs schematic labels**: primary pinout names the external reset pin `RESET`, while our current schematic labeling uses `POC`/`POC_PAD` (“power-on clear”). We currently anchor `RESET` as an alias of `POC_PAD` (see `docs/evidence/schematic_layout_anchors_v1.json` and `docs/emulators/readme.txt` which treats external `POC` as reset).
- **Power rails (`VSS/VCC`)**: anchored via edge-label crops (tokens `G`/`V`). See `docs/evidence/POWER_RAIL_EVIDENCE.md`.
