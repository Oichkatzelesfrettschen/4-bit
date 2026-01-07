# Tooling Audit (MCS-4 / MCS-40)

Scope
- Reproducible, auditable tooling inventory for MCS-4/MCS-40 document processing and analysis.
- Focus on: downloads, PDF extraction/OCR, image pipelines, mask/layer viewing, and repo doc validation.

Principles
- Prefer OS packages (`pacman`/`yay`) for system tools and native libraries.
- Use `pipx` only for CLI tools that are not well packaged (or need isolation).
- Treat warnings as errors: prefer workflows/options that avoid known OCR tool warnings by default.

## Required (Repo)

**Docs registry validation**
- `yq` (mikefarah/yq) for `scripts/doc_validate.sh`

**PDF + OCR**
- `ocrmypdf` + `tesseract` (+ `tesseract-data-eng`, `tesseract-data-osd`)
- `poppler` tools: `pdftotext`, `pdfimages`, `pdftoppm`
- `qpdf` (page extraction, repair)

**Rust workspace quality**
- `cargo`/`rustc` per `rust-toolchain.toml`
- `cargo-deny`, `cargo-audit`, `cargo-llvm-cov`

## Recommended (Evidence + Analysis)

**PDF analysis (Python)**
- `python-pdfplumber` (text extraction, page-level inspection)
- `python-pytesseract` (OCR integration)
- `python-pymupdf` (PyMuPDF; fast page raster + text extraction)

**PDF utilities**
- `pdfcpu`, `pdftk`, `qpdf`

**Images**
- `imagemagick` (`magick`, `identify`)
- `scantailor-advanced` (scan cleanup), `unpaper` (despeckle/deskew helpers)
- `gimp`, `inkscape`
- `exiftool` (provenance/metadata inspection)

**Acquisition**
- `aria2`, `wget2`, `axel`, `rclone`

**Layout/netlist tooling (optional)**
- `klayout`, `xschem`, `ngspice`, `netlistsvg`

## Arch/CachyOS Install (Reference)

Minimal required:
- `yay -S --needed yq ocrmypdf tesseract tesseract-data-eng tesseract-data-osd poppler qpdf cargo-deny cargo-audit cargo-llvm-cov`

Recommended evidence extras:
- `yay -S --needed python-pdfplumber python-pytesseract python-pymupdf pdfcpu pdftk imagemagick exiftool scantailor-advanced unpaper`

Acquisition + UI helpers:
- `yay -S --needed aria2 wget2 axel rclone gimp inkscape gimagereader-qt gocr`

## pipx (Optional)

Useful audit/QA CLIs (already supported by `pipx`):
- `pipx install linkchecker`
- `pipx install bandit ruff mypy`

Policy
- If a tool exists as a good Arch package, prefer `yay -S`.

## Current Known Tool Warnings

OCR
- Ghostscript 10.06 emits JPEG corruption warnings during OCR; this appears to be a known ocrmypdf warning path.
- `ocrmypdf --force-ocr` can explode output size; prefer `--output-type pdf` and avoid PDF/A unless needed for archival.

Mitigations
- Prefer: `ocrmypdf --skip-text --output-type pdf --sidecar <txt> <in.pdf> /tmp/<out>.pdf`
- Use chunked OCR for very large PDFs to avoid resource limits (see `docs/evidence/README.md`).

Notes
- On some distros, `cargo-llvm-cov` behaves as a Cargo subcommand; use `cargo llvm-cov --version` (not `cargo-llvm-cov --version`) for version checks.
