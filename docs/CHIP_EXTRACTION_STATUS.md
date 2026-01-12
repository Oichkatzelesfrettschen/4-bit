# Chip Extraction Status

This repo aims to extract MCS‑4 / MCS‑40 family chips from first‑party mask + schematic sources into
reproducible artifacts (`netlist_v0`, `netlist_v1`, device graphs, bounded subcircuits) that can
eventually drive a switch‑level (and later, higher‑fidelity) emulator.

## Current state (as of this commit)

## Quick metrics

| Chip | netlist_v0 nodes | netlist_v0 transistors | netlist_v1 transistors | netlist_v1 anchors |
|---:|---:|---:|---:|---:|
| 4001 | 5744 | 2000 | 1999 | 0 |
| 4002 | 3280 | 640 | 639 | 0 |
| 4003 | 490 | 38 | 37 | 0 |
| 4004 | 3448 | 1031 | 1030 | 14 |

Notes:
- `netlist_v1` may have fewer transistors than `netlist_v0` due to bbox sanity filters (see `scripts/build_netlist_v1_v0.py`).

### 4004
- Layout masks + schematic sources: `docs/emulators/i4004-*.bmp`, `docs/emulators/i4004-signals.txt`.
- Layout extraction:
  - `docs/evidence/netlists_v0/4004_netlist_v0.json`
  - `docs/evidence/netlists_v1/4004_netlist_v1.json` (uses `docs/evidence/schematic_layout_anchors_v1.json`)
- Anchors:
  - `docs/evidence/schematic_layout_anchors_v0.json` (historical / manual)
  - `docs/evidence/schematic_layout_anchors_v1.json` (remapped onto transistor‑incident nodes)
  - Anchor incidence report: `docs/evidence/anchor_incidence_v1_canonical/4004/4004/4004_anchor_incidence_v0.md`
  - Overlay crops (src/dst boxes): `docs/evidence/anchors_v1_overlays/4004/`
- Subcircuits (transistorwise neighborhoods):
  - `docs/evidence/subcircuits_v0/4004/manifest.json`
  - `docs/evidence/subcircuits_v0/4004/metrics.md`

Open lacunae:
- `SYNC`, `POC_PAD`, `TEST_PAD` still lack layout anchors (they are present in `signals.txt` but not yet mapped to layout nodes).
- Several anchors land on **terminal-only** incidence (gate incidence is 0). This may be correct for data pads but is suspicious for clock/control lines; requires refinement.

### 4001 / 4002 / 4003
- Layout masks + schematic sources exist:
  - `docs/emulators/i400{1,2,3}-*.bmp`
  - `docs/emulators/i400{1,2,3}-signals.txt`
- Layout extraction exists (`netlist_v0`):
  - `docs/evidence/netlists_v0/4001_netlist_v0.json`
  - `docs/evidence/netlists_v0/4002_netlist_v0.json`
  - `docs/evidence/netlists_v0/4003_netlist_v0.json`
- netlist_v1 exists (currently no anchors, signals list is empty):
  - `docs/evidence/netlists_v1/4001_netlist_v1.json`
  - `docs/evidence/netlists_v1/4002_netlist_v1.json`
  - `docs/evidence/netlists_v1/4003_netlist_v1.json`
- Layout pad candidates (periphery metal ranking) exist:
  - `docs/evidence/layout_pad_candidates_v0/400{1,2,3}/`
- Layout pad label box detection exists (needs manual labeling / anchor mapping):
  - `docs/evidence/layout_pad_labels_v0/4001/`
  - `docs/evidence/layout_pad_labels_v0/4002/`
  - `docs/evidence/layout_pad_labels_v0/4003/`

Open lacunae:
- No per-chip anchors exist yet in `docs/evidence/schematic_layout_anchors_v1.json` for 4001/4002/4003.
- `netlist_v1`, device graphs, and subcircuits are not yet emitted for 4001/4002/4003.
- Need to run pad-label detection (`scripts/detect_layout_pad_labels_v0.py`) and tie results to anchors.
