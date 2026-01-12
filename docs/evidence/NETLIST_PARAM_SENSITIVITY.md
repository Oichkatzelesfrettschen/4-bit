# Netlist Parameter Sensitivity (Notes)

`netlist_v0` node ids are **not stable** across extraction parameter changes.

Why it matters
- The manual anchor file `docs/evidence/schematic_layout_anchors_v0.json` stores `layout_node` integers.
- If extraction parameters change (`--dilate`, `--diffusion-split`, thresholds), the DSU/component ordering changes and node ids can shift.

Current mitigation
- When evaluating alternative extraction parameters, compare **regions** using `metal_bbox` overlap (IoU) to map old anchor regions to new node ids.

Observed (scratch test)
- Running `scripts/extract_netlist_v0.py` with `--dilate 1` in a scratch out-dir changes node numbering.
- Mapping anchored pad regions via bbox IoU shows most pad nets still do not touch transistor candidates; only a small subset (e.g. `D1_PAD`) has incident devices in the current `netlist_v0`.

Next step (planned)
- Introduce a stable `node_uid` (content-derived identifier) so anchors can survive netlist regeneration and parameter tuning.
