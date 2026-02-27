# Development Guide

## Tools
- cargo, clippy, rustfmt.
- Optional: cargo-deny, cargo-audit, cargo-llvm-cov.
- go-yq/yq for docs registry validation (scripts expect the Go yq CLI).
- OCR toolchain: tesseract + tesseract-data-eng, ocrmypdf, poppler-utils (pdftotext/pdftoppm), imagemagick.
- OCR optimization: jbig2enc (improves ocrmypdf compression), unpaper, pngquant.
- Python OCR helpers: python-pytesseract, python-pdfplumber.
- CMake: for optional native tooling, keep separate.
- Toolchain baseline: MSRV 1.92.0; nightly 2026-01-07 required for portable_simd.

## Automation
- scripts/clean.sh to remove build, coverage, and core dump artifacts.
- scripts/doc_sync.sh, scripts/doc_validate.sh for docs registry sync/validation.
- scripts/md_validate.sh, scripts/md_lint.sh, scripts/link_check.sh for docs checks.
- Keep `mcs4-emu/STATUS.md` and `docs/ROADMAP.md` synchronized.
- Use `cargo clippy-all` alias to enforce `-D warnings`.
- CI: `.github/workflows/ci.yml` runs fmt/clippy/tests on nightly.

## OCR Workflow
- Create searchable PDFs with sidecar text: `ocrmypdf --skip-text --sidecar /tmp/out.txt docs/4040/4040-datasheet.pdf /tmp/4040-ocr.pdf`
- Search OCR output quickly: `rg -n -i "clock period|transistor|3000|2300" /tmp/out.txt`
- Extract PDF pages as PNGs (avoid PBM/PGM display issues): `pdftoppm -png -r 300 input.pdf /tmp/pages/page`
- Convert PBM/PGM/PPM/PNM → PNG when needed: `python3 scripts/convert_pnm_to_png.py --out-dir /tmp/pages /tmp/pages --recursive`
- Layout edge-label OCR (4004): `python3 scripts/detect_layout_edge_labels_v0.py --chip 4004 --edges left --band 360 --min-area 200 --max-area 240000 --min-fill 0.10 --pad 30 --write-crops`

## Warnings Policy
- `.cargo/config.toml` enforces `-D warnings` for builds; clippy uses `-D warnings`.
