# Transistor candidate extraction metrics

Generated from `docs/evidence/transistors/*_poly_diffusion_transistors.json`.

| Chip | Poly∩Diffusion components | Kept (area>=min) |
|------|--------------------------:|----------------:|
| 4001 | 2232 | 2228 |
| 4002 | 834 | 819 |
| 4003 | 102 | 102 |
| 4004 | 1804 (Δ vs analyzer: -3) | 1543 |

## Notes

- This is a best-effort *candidate* list from poly/diffusion intersections, not a transistor-accurate netlist.
- Counts depend on mask-layer thresholding and connected-component settings; see `docs/evidence/transistors/manifest.json`.
- The i400x analyzer readme reports 4004 layout transistors as `1807` and schematic-effective transistors as `1741` (excluding bootstrap-capacitor artifacts).
- The same table also reports passive components (not extracted here): resistors `427` and capacitors `66`.
