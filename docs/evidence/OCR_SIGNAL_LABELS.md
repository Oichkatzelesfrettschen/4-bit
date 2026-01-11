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

## ROI / coordinate schema

All `signals.txt` files use the same coordinate system:
- Absolute pixel coordinates in the corresponding `i400x-schematic.bmp`
- Origin at the top-left corner
- 0-based indexing

Current ROI strategy is point-centered and parameterized (see `./scripts/ocr_signal_labels.py --help`):
- Base region around the `(x,y)` point: `--region-w 640`, `--region-h 240`
- Candidate OCR crops are derived from detected “text-like components” inside that region, padded by `--pad 10`
- For fallback alias scans (e.g. pin numbers like `01`/`02`), the script probes multiple offsets around the point
  and runs token OCR on small windows (bounded search, not an unbounded scan).
- To avoid long-running `tesseract` hangs, use `--tesseract-timeout` (default `2.0` seconds per call).

## Performance

Quick local benchmark (4004 subset; 40 points; `CLK*`, `SYNC`, `D0..D3`):
- Before call-budget tuning: ~50s wall time
- After tuning + caching + fast-crop pass: ~27s wall time

Recommended fast calibration invocation:
- `./scripts/ocr_signal_labels.py --chip 4004 --name-regex '^(CLK1|CLK2|SYNC|D0|D1|D2|D3)$' --limit 40 --save-mismatches 0`

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
- GPU note: CUDA is available on this workstation, but the current OCR workflow uses `tesseract`.
  CUDA-backed OCR via `onnxruntime-gpu` is not currently viable on Python 3.13 (the wheel reports
  CPU-only providers), while the system `onnxruntime` build *does* expose `CUDAExecutionProvider`.
- Tooling snapshot (local workstation):
  - `tesseract 5.5.x` + `leptonica 1.87.x` installed system-wide.
  - `opencv-cuda` / `python-opencv-cuda` and `python-onnxruntime-opt-cuda` are installed system-wide.
  - In practice, GPU acceleration is currently easiest to apply to **image preprocessing** (OpenCV),
    while OCR itself remains `tesseract`-CPU unless/until we adopt a CUDA-capable OCR engine.
