# Netlist v1 Schema (Draft)

Goal
- Provide a **machine-readable bridge** between:
  - `netlist_v0` (layout connectivity from mask layers),
  - schematic-space signal names (`signals.txt` + OCR verification), and
  - progressively richer device extraction (transistors, loads) for switch-level simulation.

This is a *draft schema* intended to unblock:
- deterministic schematic↔layout matching,
- anchor-subcircuit validation (CLK/SYNC, RD/WR/CM, D0..D3),
- and the next fidelity step in `docs/ACCURACY_PROGRAM.md` (Level 2 switch-level).

Non-goals (v1)
- Full schematic component recognition (gates/transistors) from the schematic bitmap.
- Analog/electron-accurate parameters (tracked in `docs/evidence/PRIMARY_SOURCES_BACKLOG.md`).

## File location

Planned output directory:
- `docs/evidence/netlists_v1/`

## Top-level JSON structure

```json
{
  "chip": "4004",
  "schema": { "version": 1, "description": "Layout↔schematic matched netlist (draft)." },
  "inputs": {
    "layout_netlist_v0": "docs/evidence/netlists_v0/4004_netlist_v0.json",
    "schematic_wirenets_v0": "docs/evidence/schematic_wirenets_v0/4004_schematic_wirenets_v0.json",
    "schematic_net_names_v0": "docs/evidence/schematic_net_names_v0/4004_schematic_net_names_v0.json",
    "anchors_v0": "docs/evidence/schematic_layout_anchors_v0.json"
  },
  "signals": [
    {
      "name": "CLK1",
      "layout_node": 49,
      "schematic_component": 1234,
      "evidence": {
        "anchor": true,
        "notes": "Printed pin label `01` on metal mask; alias CLK1<->01."
      }
    }
  ],
  "devices": {
    "transistors": [],
    "loads": []
  }
}
```

## Required keys

- `chip`: one of `4001|4002|4003|4004`.
- `schema.version`: integer, `1` for this draft.
- `inputs.*`: paths to the exact artifacts used to build the file.
- `signals[]`: at minimum, the anchor signals required for validation.

## `signals[]` semantics

Each entry describes a **named signal** that exists in schematic space (`signals.txt`) and is mapped to:
- a layout connectivity node id (`layout_node`, from `netlist_v0`), and
- a schematic pixel connectivity component (`schematic_component`, from `schematic_wirenets_v0`).

The mapping can be partial:
- `layout_node` may be `null` until anchored or inferred.
- `schematic_component` may be `null` if the schematic point does not land on a connected component (e.g., OCR drift or missing pixels).

## Next acceptance checkpoints (v1)

- Anchor set exists for `4004`:
  - `CLK1`, `CLK2`, `SYNC`
  - `D0_PAD..D3_PAD`
  - `CMROM`, `CMRAM0..CMRAM3`
- Each anchor has:
  - a referenced layout node in `schematic_layout_anchors_v0.json`,
  - a stable schematic component id,
  - and at least one deterministic evidence artifact (crop/overlay) that can be regenerated.

## Device candidates (v1 build v0)

`netlist_v1` can embed layout-derived device candidates so downstream tooling can carve out “anchor subcircuits”.

Current implementation (`scripts/build_netlist_v1_v0.py`):
- Imports `devices.transistors` from `netlist_v0`.
- Filters catastrophic bbox outliers (currently a small number of full-image bboxes) into `devices.filtered_transistors`.

This is still *candidate* quality:
- terminals are topology-derived from masks and may be wrong in edge cases,
- device types (enhancement/depletion, pull-ups) are not fully classified yet.
