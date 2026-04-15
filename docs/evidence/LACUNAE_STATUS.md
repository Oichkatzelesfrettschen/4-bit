# Lacunae / open validation items (rolling)

This file tracks **what is still missing** for chip-accurate (and eventually transistor-/electron-accurate) reconstruction.

## Cross-cutting (all chips)

- **Schematic↔layout coordinate transforms**: `docs/emulators/i400x-signals.txt` coordinates are in schematic bitmap space and are **not directly comparable** to metal-mask pixel coordinates; each chip needs an explicit transform (homography/affine + validation).
  - **Status (2026-04-06)**: QUALITY-GATED (UNRESOLVED) - `scripts/build_coordinate_transform_v0.py` now enforces acceptance thresholds (`min_inliers=6`, `min_inlier_ratio=0.50`, `max_rmse_pixels=250`). Current outputs are:
    - `4001`: `unresolved_collinear`
    - `4002`: `unresolved_quality_gate`
    - `4003`: `unresolved_quality_gate`
    - `4004`: `unresolved_low_inliers`
    Remaining gap: correspondences are still largely heuristic remap outputs and need primary-source/manual validation before any chip is promoted to accepted homography.
- **Via↔node routing extraction (`netlists_v2`)**: `scripts/extract_via_connectivity_v0.py` now detects vias from `i400x-vias.bmp` using connected-component fallback and maps vias to node candidates via bbox/proximity heuristics.
  - **Status (2026-04-06)**: PARTIALLY RESOLVED - `docs/evidence/netlists_v2/*/*_netlist_v2.json` now contains non-empty `vias`, `routing_graph.edges`, and per-chip validation reports.
  - Cross-check summary against anchor-mapped nodes (`docs/evidence/netlists_v2/via_route_validation_summary.json`):
    - 4001: 19/21 anchor-trace nodes have via evidence (90.5%)
    - 4002: 20/22 anchor-trace nodes have via evidence (90.9%)
    - 4003: 13/16 anchor-trace nodes have via evidence (81.2%)
    - 4004: 16/19 anchor-trace nodes have via evidence (84.2%)
    - 2026-04-07 first-wave alias mapping applied for 4001/4002: `(RESET)` + `D0..D3` seeded from mapped `RESET`/`D*_PAD` anchors via `scripts/apply_priority_anchor_aliases_v1.py`.
    - Coverage caveat: many anchors still lack mapped trace nodes (especially 4002), so this is not end-to-end anchor coverage yet.
  - Priority queue for next manual trace-node mapping pass (from `priority_signals_*` in the summary JSON):
    - 4001: focus shifts to clock-gated decode/control nets and reset via evidence (`(RESET)`/`RESET` currently mapped but lacking via edges).
    - 4002: focus remains clocked RAM control nets (`CLK*`, `RAM_*`); `[RESET]` alias now maps to `RESET`, while `D3`/`D3_PAD` still lack via evidence.
    - 4003: `VDD` has trace-node presence but currently no via evidence.
    - 4004: `CMROM`, `D0_PAD`, and `D1_PAD` currently have no via evidence.
  - Remaining gap: connectivity is heuristic (bbox/proximity), not yet validated against explicit physical route traces or manual layer-by-layer correspondence.
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
