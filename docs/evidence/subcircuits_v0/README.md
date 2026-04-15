# Subcircuits v0 Artifacts

This directory stores extracted anchor-centered transistor subcircuits and
summary metrics used by the evidence pipeline.

## Structure

- `4001/`, `4002/`, `4003/`, `4004/`: per-chip extraction outputs.
- `manifest.json` files: canonical listing of generated subcircuit files.
- `metrics.json` and `metrics.md`: per-chip aggregate statistics.

## Regeneration

Use `scripts/extract_subcircuit_v0.py` and `scripts/subcircuit_metrics_v0.py`
as documented in `docs/NETLIST_WORKFLOW.md`.
