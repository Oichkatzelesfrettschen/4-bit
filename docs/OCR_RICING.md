# OCR Ricing (Accuracy + Speed)

This repo does OCR over tiny schematic/layout labels (often white text on a black “callout bubble”). The two recurring failure modes are:
1) Wrong ROI (too much wiring / arrow tail, or too little text).
2) Too many redundant OCR invocations (slow runs).

## Baseline regression benchmark

Use the stable benchmark fixture before/after changes:
- `python3 scripts/ocr_benchmark_v0.py --bench docs/evidence/ocr_benchmarks_v0/layout_edge_labels_4004_v0.json`

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

