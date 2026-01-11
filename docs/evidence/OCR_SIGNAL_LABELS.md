# OCR Signal Label Verification (i400x `signals.txt`)

Goal: turn the `docs/emulators/i400{1,2,3,4}-signals.txt` coordinate maps into a reproducible OCR-based
verification report, and a prioritized mismatch list for manual review.

## Quick start

- Sanity-check `signals.txt` coordinates against schematic dimensions:
  - `python3 scripts/verify_signals_txt.py --all`
- Calibrate on a small subset (fast):
  - `./scripts/ocr_signal_labels.py --chip 4004 --name-regex '^(CLK1|CLK2)$'`
- Generate a broader report (slower):
  - `./scripts/ocr_signal_labels.py --chip 4001 --labels-only`
- Summarize reports into a regression-friendly table:
  - `./scripts/ocr_signal_metrics.py`
- Rebuild the multi-chip manifest (if you ran chips separately):
  - `./scripts/ocr_signal_manifest.py`

Outputs land in `docs/evidence/ocr_signal_labels/<chip>/`.

## Overlays

To visually audit point placement and mismatches on top of the schematic:

- `./scripts/render_signal_overlays.py --all --mode mismatch --limit 200`

Outputs land in `docs/evidence/ocr_signal_labels/<chip>/overlays/`.

## Current status

This workflow is implemented and produces deterministic reports, but OCR accuracy is currently limited:
- Many `signals.txt` entries appear to be *internal net names* that are not printed near the coordinate.
- Many printed labels are vertical and very small, and OCR frequently fails (`reason=ocr_no_tokens` / `ocr_low_conf`).
- Some coordinates point at a node, while the schematic prints a *pin label* nearby; these need aliases.
- Some coordinates are on long wires/empty space; when no text-like components are detected, the report uses `reason=no_text_components`.

You can see per-chip counts in:
- `docs/evidence/ocr_signal_labels/4001/4001_signal_ocr_report.json`
- `docs/evidence/ocr_signal_labels/4002/4002_signal_ocr_report.json`
- `docs/evidence/ocr_signal_labels/4003/4003_signal_ocr_report.json`
- `docs/evidence/ocr_signal_labels/4004/4004_signal_ocr_report.json`
- `docs/evidence/ocr_signal_labels/metrics.md` (aggregated table)

## Alias mapping

Some `signals.txt` entries are *net names* even when the schematic prints a *pin number* at that coordinate.
Example on 4004: expected `CLK1` / `CLK2`, printed `01` / `02`.

Provide these exceptions in `scripts/ocr_signal_aliases.json`, keyed by chip:

```json
{ "4004": { "CLK1": ["01"], "CLK2": ["02"] } }
```

## Notes

- Many labels in `i4004-schematic.bmp` are vertical/rotated; OCR quality depends heavily on preprocessing.
- Expect mismatches; the value is the deterministic mismatch set + annotated crops for inspection.
