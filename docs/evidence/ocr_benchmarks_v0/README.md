# OCR Benchmarks v0

This folder contains a small, stable benchmark set for OCR changes.

Goal: make it easy to measure “did OCR get better (accuracy) and faster (runtime)?” before changing evidence workflows.

## Files

- `layout_edge_labels_4004_v0.json`: expected tokens for a handful of 4004 metal-mask edge labels.

## Run

- `python3 scripts/ocr_benchmark_v0.py --bench docs/evidence/ocr_benchmarks_v0/layout_edge_labels_4004_v0.json`
