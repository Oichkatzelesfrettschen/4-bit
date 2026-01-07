# Netlist Extraction Workflow (i400x Analyzer)

Purpose
- Describe how the i400x analyzer derives netlists from mask layers and schematic bitmaps.
- Focus on Intel 4001/4002/4003/4004 assets bundled under docs/emulators/.

Inputs (per chip)
- Layer bitmaps: metal, vias, poly, diffusion (plus contacts for 4004).
- Schematic bitmap: i400x-schematic.bmp.
- Signal map: i400x-signals.txt.

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

Notes
- The analyzer highlights transistor states and bus waveforms during simulation.
- Differences noted in 4004 between layout and schematic include TEST pin revision and gate input order.

License
- Intel historical materials are CC BY-NC-SA; see docs/emulators/license.txt and docs/evidence/photomicrograph_permissions.md.
