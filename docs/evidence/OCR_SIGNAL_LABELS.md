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

## OCR backend order (recommended)

For this repo’s “short token / tiny label” use-case (pin labels like `01`, single letters like `C`, and short net names),
the most reliable and cost-effective order is:

1. **ONNX Runtime + CUDA (CTC OCR model)** when available and validated.
   - Best throughput once a model is adopted and preprocessing is stable.
2. **ONNX Runtime CPU** as a compatibility fallback.
3. **Tesseract CLI fast-path** (`tesseract_cli_fast`): one preprocessing preset + one PSM.
   - Used for batch pipelines and micro-benchmarks where speed matters.
4. **Tesseract CLI full sweep** (`tesseract`): multi-variant preprocessing + multiple PSMs.
   - Use when accuracy matters more than runtime.
5. **OpenCV Hu moments** (`template`): single-glyph fallback when OCR returns no tokens.

Separately, apply acceleration where it is currently most practical:

- **GPU-accelerated preprocessing** (OpenCV CUDA / similar) for denoise, threshold, morphology, rotation, and cropping.
  This can materially reduce total wall time even if OCR remains CPU.
- **SIMD-first OCR**: prefer the system `tesseract` binary (typically built with AVX/AVX2 on modern distros) over
  pure-Python OCR stacks. For our tiny, high-contrast glyphs, the limiting factor is usually preprocessing/ROI, not
  the recognizer model.

## Whitelists / charsets

Different label classes require different allowed-character sets; using a strict whitelist measurably reduces false positives.

- **Edge tokens** (mask periphery): `A–Z` + `0–9` only.
  - Typical: `S`, `T`, `RM`, `R0..R3`, `D0..D3`, `01`, `02`.
- **Schematic net labels**: `A–Z` + `0–9` plus a small operator set.
  - Used by `scripts/ocr_signal_labels.py` for internal nets: `~()+&/._-[]`.
- **Numeric-only** (die markings / chip IDs): `0–9` only.

Implementation notes:

- `scripts/ocr_preprocess_v0.py::ocr_best_token()` enforces `tessedit_char_whitelist=` and filters results by length.
- `scripts/ocr_backend_v0.py::OnnxCtcBackend.best_token()` additionally filters decoded strings against the caller’s whitelist.
- `scripts/ocr_backend_v0.py` selects preprocessing presets heuristically (`digits_tiny` vs `glyph_single` vs `edge_label`) based on whitelist + expected length.
- You can override the preset selection for experiments with `OCR_PRESET=outline_fill` (or any key in `scripts/ocr_preprocess_v0.py::_PRESETS_V0`).

## Preprocess presets (v0)

Current recommended mapping:

- **Digits (`01`, `02`, `14`, `15`)** → `digits_tiny`
- **Single glyphs (`T`, `C`, `S`, `V`, `G`, `L`)** → `glyph_single`
- **Short alnum (`RM`, `R0..R3`, `D0..D3`)** → `edge_label`
- **Outline/engraved text (logos / chip IDs)** → `outline_fill` (use selectively; it reduced edge-label accuracy in our quick bench)

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
  The repo supports an optional ONNX CTC backend via `onnxruntime`; set `OCR_ONNX_MODEL=/path/to/model.onnx`
  and prefer GPU execution providers with `--prefer-cuda` where applicable.
- Tooling snapshot (local workstation):
  - `tesseract 5.5.x` + `leptonica 1.87.x` installed system-wide.
  - `opencv-cuda` / `python-opencv-cuda` and `python-onnxruntime-opt-cuda` are installed system-wide.
  - In practice, GPU acceleration is currently easiest to apply to **image preprocessing** (OpenCV),
    while OCR itself remains `tesseract`-CPU unless/until we adopt a CUDA-capable OCR engine.
