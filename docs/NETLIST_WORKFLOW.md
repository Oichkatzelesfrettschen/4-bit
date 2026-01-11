# Netlist Extraction Workflow (i400x Analyzer)

Purpose
- Describe how the i400x analyzer derives netlists from mask layers and schematic bitmaps.
- Focus on Intel 4001/4002/4003/4004 assets bundled under docs/emulators/.

Inputs (per chip)
- Layer bitmaps: metal, vias, poly, diffusion (plus contacts for 4004).
- Schematic bitmap: i400x-schematic.bmp.
- Signal map: `docs/emulators/i400{1,2,3,4}-signals.txt`.

Workflow summary (from docs/emulators/readme.txt)
- Extract netlist from mask layers.
- Extract netlist from schematic bitmap.
- Compare and reconcile differences; match signals and components.
- Use simulator to validate against MCS-4 instruction behavior.

Practical steps
- Unzip i400x analyzer package (Windows executable) into a working directory.
- Ensure the layer bitmaps and schematic bitmap are alongside the analyzer.
- Run i400x_analyzer_x64.exe and open the relevant chip (4001/4002/4003/4004).
- Use the compare view to review netlist differences and matched components.

Linux notes
- The analyzer is a Windows GUI application; on Linux you can try running it under `wine` (not currently required for the repo).
- For evidence trails that only need component counts, the extracted `readme.txt` in the analyzer bundle is sufficient (see `docs/evidence/README.md`).

Repo-local (v0) extraction
- This repo includes an initial, deterministic layout connectivity extractor: `scripts/extract_netlist_v0.py`.
- Outputs are written under `docs/evidence/netlists_v0/`:
  - `*_netlist_v0.json`: per-chip stitched connectivity (layout masks only).
  - `manifest.json`: tool parameters + output list.
  - `metrics.json` / `metrics.md`: summary from `scripts/netlist_v0_metrics.py`.

How to run
- Generate (or regenerate) all netlists: `python3 scripts/extract_netlist_v0.py --all`
- Summarize counts: `python3 scripts/netlist_v0_metrics.py --out-json docs/evidence/netlists_v0/metrics.json --out-md docs/evidence/netlists_v0/metrics.md`

What v0 does (and does not do)
- Uses layout masks only (metal/vias/poly/diffusion, plus contacts for 4004).
- Splits diffusion by removing poly overlap (matching the analyzer's "diffusion split by poly" note) to reduce accidental shorting through transistor gate crossings.
- Stitches nets using:
  - `vias & metal & poly` → connect metal ↔ poly.
  - `contacts & metal & diffusion_split` (4004 only) → connect metal ↔ diffusion.
- Attaches "transistor candidates" from `docs/evidence/transistors/*_poly_diffusion_transistors.json` by mapping bbox regions to poly (gate) and diffusion (terminals) component labels.
- Does not attempt to solve transistor states or build a full switch-level simulation netlist yet.
- The `i400x-signals.txt` reference points are defined on the schematic bitmap (different coordinate space); v0 includes them only as schematic cross-reference data (`signals.space = schematic`).

Notes
- The analyzer highlights transistor states and bus waveforms during simulation.
- Differences noted in 4004 between layout and schematic include TEST pin revision and gate input order.

License
- Intel historical materials are CC BY-NC-SA; see docs/emulators/license.txt and docs/evidence/photomicrograph_permissions.md.
