# Chip Extraction Status

This repo aims to extract MCS‑4 / MCS‑40 family chips from first‑party mask + schematic sources into
reproducible artifacts (`netlist_v0`, `netlist_v1`, device graphs, bounded subcircuits) that can
eventually drive a switch‑level (and later, higher‑fidelity) emulator.

## Current state (as of this commit)

## Quick metrics

| Chip | netlist_v0 nodes | netlist_v0 transistors | netlist_v1 transistors | netlist_v1 anchors |
|---:|---:|---:|---:|---:|
| 4001 | 5744 | 2000 | 1999 | 14 |
| 4002 | 3280 | 640 | 639 | 14 |
| 4003 | 490 | 38 | 37 | 14 |
| 4004 | 3448 | 1031 | 1030 | 19 |

Notes:
- `netlist_v1` may have fewer transistors than `netlist_v0` due to bbox sanity filters (see `scripts/build_netlist_v1_v0.py`).

### 4004
- Layout masks + schematic sources: `docs/emulators/i4004-*.bmp` (source) + `docs/emulators/i4004-*.png` (preview), plus `docs/emulators/i4004-signals.txt`.
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
- `SYNC`, `POC_PAD`, `TEST_PAD` are now anchored to layout nodes (via edge-label detection). `RESET` is also anchored (treated as an alias of the external `POC`/`POC_PAD` pad in the emulator schematics).
- Several anchors land on **terminal-only** incidence (gate incidence is 0). This may be correct for data pads but is suspicious for clock/control lines; requires refinement.

### 4001 / 4002 / 4003
- Layout masks + schematic sources exist:
  - `docs/emulators/i400{1,2,3}-*.bmp` (source) + `docs/emulators/i400{1,2,3}-*.png` (preview)
  - `docs/emulators/i400{1,2,3}-signals.txt`
- Layout extraction exists (`netlist_v0`):
  - `docs/evidence/netlists_v0/4001_netlist_v0.json`
  - `docs/evidence/netlists_v0/4002_netlist_v0.json`
  - `docs/evidence/netlists_v0/4003_netlist_v0.json`
- netlist_v1 exists (with a minimal anchored signal set derived from pinouts + geometry + incidence):
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
- 4001/4002/4003 anchors are present with **medium confidence** for power rails (geometry-corroborated against PRIMARY_SOURCE_PINOUTS.md).
- Expand the anchored signal set beyond the pinout-minimum and validate subcircuit topology against primary sources.
- 4003 uses Q0-Q9 naming for parallel outputs (primary source uses O0-O9) - naming alignment documented in ANCHOR_COVERAGE_V0.md.

Recent improvements:
- **2026-01-14**: Power rail anchors (VSS/VDD) for 4001/4002/4003 upgraded from low-confidence to **medium-confidence** via pad geometry corroboration against primary pinouts. See `docs/evidence/POWER_RAIL_EVIDENCE.md`.
- **2026-01-14**: CI schematic pipeline passes all checks (anchor audit, pad consistency, incidence, uniqueness).
- 4001/4002/4003 pad↔pin seeding is now driven by `docs/evidence/layout_pad_labels_v0/<chip>/pad_pin_template_v0.md` (primary pinouts + pad geometry), applied via `scripts/apply_pad_pin_template_v0.py`.
- Anchor remapping now enforces **unique incident dst nodes per pad** when seeded from the template (see `scripts/remap_anchors_to_incident_nodes_v1.py`), preventing the previous “many pads collapse to one incident trunk” failure mode.
- CI now includes an anchor uniqueness check for required signals: `python -W error scripts/check_anchor_uniqueness_v0.py --chip 4001 --chip 4002 --chip 4003` (also run by `scripts/ci_schematic_pipeline_v0.sh`).
