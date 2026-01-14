# Lacunae / open validation items (rolling)

This file tracks **what is still missing** for chip-accurate (and eventually transistor-/electron-accurate) reconstruction.

## Cross-cutting (all chips)

- **Schematic↔layout coordinate transforms**: `docs/emulators/i400x-signals.txt` coordinates are in schematic bitmap space and are **not directly comparable** to metal-mask pixel coordinates; each chip needs an explicit transform (homography/affine + validation).
- **Power-rail anchoring**: `VSS/VDD/VCC` anchors exist in `docs/evidence/schematic_layout_anchors_v1.json`, but confidence varies:
  - 4004 rails are tied to edge-label evidence (tokens `G`/`V`) and are treated as **evidence-backed**.
  - 4001–4003 rails are currently seeded via `pad_pin_template_v0` (CCW DIP-order hypothesis) and should be treated as **low-confidence** until corroborated by additional pin anchors.
- **Pad label OCR limits**: metal-mask periphery rarely prints full signal names; short tokens (`S`, `T`, `R3`, `RM`, …) are ambiguous and must be validated against primary sources + geometry.

## 4001

- **VSS/VDD anchors**: `VSS/VDD` are present, but currently seeded via `pad_pin_template_v0` (low confidence). See `docs/evidence/POWER_RAIL_EVIDENCE.md`.
- **Periphery pin mapping**: seeded via cyclic angle-alignment (`docs/evidence/anchor_seed_suggestions_v0/4001_angle_alignment.json`) then remapped to incident nodes; still needs validation once net segmentation improves (pads currently collapse onto shared trunks in some cases).

## 4002

- **Chip-select (`P0`/`Po`) naming**: schematic files use `CS` while the primary pinout calls this `P0/Po`. Decide a canonical name and add an alias mapping.
- **Output pins naming**: schematic files use `OUT0..OUT3`; the primary pinout describes a 4-bit output port without a fixed bit labeling. Confirm `OUT0..OUT3` ↔ DIP pins `13..16` via a primary diagram (or annotated schematic).
- **VSS/VDD anchors**: present, but currently seeded via `pad_pin_template_v0` (low confidence). See `docs/evidence/POWER_RAIL_EVIDENCE.md`.

## 4003

- **VDD pin evidence**: the MCS-40 User’s Manual pinout (Figure 4-21) is partially text-extractable; the expected `VDD` pin is not currently captured by `pdftotext` and must be confirmed via OCR of the figure (or another primary source).
- **VSS/VDD anchors**: present, but currently seeded via `pad_pin_template_v0` (low confidence). See `docs/evidence/POWER_RAIL_EVIDENCE.md`.
- **Pad labels**: most detected “labels” are die markings (`4003`, `intel`), not per-pin labels; pad↔pin mapping remains human/geometry-driven.

## 4004

- **RESET pin naming vs schematic labels**: primary pinout names the external reset pin `RESET`, while our current schematic labeling uses `POC`/`POC_PAD` (“power-on clear”). We currently anchor `RESET` as an alias of `POC_PAD` (see `docs/evidence/schematic_layout_anchors_v1.json` and `docs/emulators/readme.txt` which treats external `POC` as reset).
- **Power rails (`VSS/VCC`)**: anchored via edge-label crops (tokens `G`/`V`). See `docs/evidence/POWER_RAIL_EVIDENCE.md`.
