# OCR Ricing (Accuracy + Speed)

This repo does OCR over tiny schematic/layout labels (often white text on a black “callout bubble”). The two recurring failure modes are:
1) Wrong ROI (too much wiring / arrow tail, or too little text).
2) Too many redundant OCR invocations (slow runs).

## Baseline regression benchmark

Use the stable benchmark fixture before/after changes:
- `python3 scripts/ocr_benchmark_v0.py --bench docs/evidence/ocr_benchmarks_v0/layout_edge_labels_4004_v0.json`

## Optional GPU experiments (ONNX/CUDA)

The repo’s default is **Tesseract (CPU)** for reproducibility. For speed/accuracy experiments on large datasets, prefer:
- `onnxruntime` with `CUDAExecutionProvider` (available in this environment)
- a small **CTC-based text recognizer** (word-level) exported to ONNX

Backend fallback order (implemented in `scripts/ocr_backend_v0.py`):
- ONNX (CUDA) → ONNX (CPU) → Tesseract (CPU)

To enable ONNX without committing model weights into git:
- Set `OCR_ONNX_MODEL=/abs/path/to/model.onnx`
- Optional sidecar config: `model.onnx.json` containing `alphabet` and `blank_id`

Run the stable micro-benchmark with ONNX enabled:
- `OCR_ONNX_MODEL=/abs/path/to/model.onnx python3 scripts/ocr_benchmark_v0.py --bench docs/evidence/ocr_benchmarks_v0/layout_edge_labels_4004_v0.json --backend auto`

Run a larger manifest set (derived from crops):
- `python3 scripts/ocr_crops_manifest_v0.py --crops-dir docs/evidence/layout_edge_labels_v0/4004/crops --out /tmp/edge_crops.json --dedupe`
- `python3 scripts/ocr_manifest_run_v0.py --manifest /tmp/edge_crops.json --backend onnx --onnx-model /abs/path/to/model.onnx`

## Preprocessing policy (v0)

Canonical helpers live in `scripts/ocr_preprocess_v0.py`:
- `head_crop()`: for long-arrow labels, keep the top head region.
- `extract_dense_component()`: isolate the dense bubble from thin wiring.
- `crop_label_text_roi()`: isolate the glyph region inside the bubble.
- `preprocess_label_for_ocr()`: normalize → optional CLAHE → scale → threshold → border.

The `scripts/detect_layout_edge_labels_v0.py` fast path uses:
- `invert=True` first (white-on-black glyphs), with fallback to `invert=False`.
- `--psm 7,11` for `image_to_data` (fast, stable).
- `image_to_string` sweep only as a fallback (or always in `--deep` mode).

## Input format hygiene

Some tools (including Codex image attachment) can’t display PNM variants (`.pbm/.pgm/.ppm/.pnm`). Convert them to PNG first:
- `python3 scripts/convert_pnm_to_png.py --recursive --out-dir /tmp/<dir> /tmp/<dir>`

## Primary-source guidance (Tesseract)

Tesseract’s own recommendations are worth following, especially for tiny text:
- https://tesseract-ocr.github.io/tessdoc/ImproveQuality.html
- https://tesseract-ocr.github.io/tessdoc/Command-Line-Usage.html
