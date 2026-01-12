# OCR Benchmarks v0

This folder contains a small, stable benchmark set for OCR changes.

Goal: make it easy to measure “did OCR get better (accuracy) and faster (runtime)?” before changing evidence workflows.

## Files

- `layout_edge_labels_4004_v0.json`: expected tokens for a handful of 4004 metal-mask edge labels.
- `layout_edge_labels_4004_manifest_run_tesseract_v0.json`: OCR run over a larger set of labeled crops (derived from crop filenames).

## Run

- Small stable benchmark:
  - `python3 scripts/ocr_benchmark_v0.py --bench docs/evidence/ocr_benchmarks_v0/layout_edge_labels_4004_v0.json`

- Manifest run (larger, derived from `docs/evidence/layout_edge_labels_v0/.../crops`):
  - `python3 scripts/ocr_manifest_run_v0.py --manifest docs/evidence/ocr_manifests_v0/layout_edge_labels_4004_crops_v0.json --backend tesseract`

## Optional ONNX

The benchmark/manifest runners can use an optional ONNX CTC model:

- `OCR_ONNX_MODEL=/abs/path/to/model.onnx python3 scripts/ocr_benchmark_v0.py --bench ... --backend auto`
- `python3 scripts/ocr_manifest_run_v0.py --manifest ... --backend onnx --onnx-model /abs/path/to/model.onnx`
