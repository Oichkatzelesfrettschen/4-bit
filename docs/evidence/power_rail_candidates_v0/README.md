# Power rail candidate overlays (v0)

This folder holds *candidate* layout nodes for power rails (`VSS`, `VDD`, `VCC`) derived from the extracted netlists.

Why this exists:
- `i400x-signals.txt` focuses on externally-visible pads and key internal control signals.
- Power rails are usually not explicitly labeled in the extracted schematics/anchor lists, but they must be fixed for
  any “electron-accurate” netlist/subcircuit work.

Files:
- `*/<chip>_power_rail_candidates_v0.json`: ranked list of layout nodes that “look like” rails (edge-proximal and
  incident-heavy).
- `*/<chip>_power_rail_candidates_v0_overlay.png`: overview overlay of the top candidates.
- `*/crops/*.png`: crops around each candidate’s bbox for manual inspection.

Workflow:
1. Run `python -W error scripts/suggest_power_rail_nodes_v0.py --chip <chip> --top 60`.
2. Inspect `docs/evidence/power_rail_candidates_v0/<chip>/<chip>_power_rail_candidates_v0_overlay.png` and crops.
3. Choose one candidate for each rail (chip-specific: `VSS/VDD` for 4001–4003, `VSS/VCC` for 4004).
4. Record the decision in `docs/evidence/POWER_RAIL_EVIDENCE.md`.
5. Set the chosen seed node via `layout_node_src` and re-run `scripts/remap_anchors_to_incident_nodes_v1.py`.

