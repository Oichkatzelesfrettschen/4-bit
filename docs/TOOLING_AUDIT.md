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
- `python-pillow` (PIL; used by multiple repo scripts for deterministic crop rendering + overlays)
- `scantailor-advanced` (scan cleanup), `unpaper` (despeckle/deskew helpers)
- `gimp`, `inkscape`
- `exiftool` (provenance/metadata inspection)

**Acquisition**
- `aria2`, `wget2`, `axel`, `rclone`

**Layout/netlist tooling (optional)**
- `klayout`, `xschem`, `ngspice`, `netlistsvg`

## GPU-Accelerated OCR (Optional)

This repo’s default OCR path is **Tesseract** (CPU), which is good for reproducible text extraction but can be slow.
If you want higher throughput for large-scale page rasterization + OCR experiments, a GPU stack can help.

**Common building blocks**
- CUDA driver/toolkit (NVIDIA): required for CUDA-backed inference.
- `python-opencv-cuda`: can accelerate parts of image preprocessing (resize/threshold/denoise) when the pipeline supports it.
- `python-onnxruntime-opt-cuda`: provides `onnxruntime` with `CUDAExecutionProvider` (useful for ONNX OCR models).
- `python-pytorch-opt-cuda`: enables CUDA for Torch-based OCR pipelines.

Provider preference order (when available)
- ONNX Runtime: TensorRT → CUDA → DNNL → CPU
- OCR backend fallback (`scripts/ocr_backend_v0.py`): ONNX(TensorRT/CUDA) → ONNX(DNNL/CPU) → Tesseract(CPU)

Packaging note
- Prefer the distro’s CUDA-enabled `onnxruntime` package (`python-onnxruntime-opt-cuda`) over `pip install onnxruntime-gpu` when Python versions/wheels lag behind (the CUDA provider often disappears on newer Python minors).

**OCR frameworks (not required by the repo)**
- EasyOCR, PaddleOCR, or custom ONNX models (typically faster than Tesseract on modern GPUs, but heavier dependencies).

## Repo-local OCR Benchmarking

For label OCR changes, use the stable micro-benchmark set:
- `python3 scripts/ocr_benchmark_v0.py --bench docs/evidence/ocr_benchmarks_v0/layout_edge_labels_4004_v0.json`

Implementation note:
- `scripts/ocr_preprocess_v0.py` sets `pytesseract.pytesseract.tesseract_cmd` to `/usr/bin/tesseract` when present to avoid PATH-related flakiness.
- `scripts/ocr_backend_v0.py` implements backend fallback: ONNX(TensorRT/CUDA) → ONNX(DNNL/CPU) → Tesseract(CPU) (set `OCR_ONNX_MODEL` to enable ONNX).
- `scripts/ocr_capabilities_v0.py` records what the current environment can accelerate (OpenCV CUDA, ONNX providers, Torch CUDA, etc.).

## Tooling Audit Runs

Capture the current environment inventory to a versioned artifact:
- `bash scripts/tooling_audit.sh | tee docs/evidence/tooling_audit_runs/tooling_audit_$(date +%Y%m%d)_v0.txt`
- `python3 scripts/ocr_capabilities_v0.py --out docs/evidence/tooling_audit_runs/ocr_capabilities_$(date +%Y%m%d)_v0.json`

## Arch/CachyOS Install (Reference)

Minimal required:
- `yay -S --needed yq ocrmypdf tesseract tesseract-data-eng tesseract-data-osd poppler qpdf cargo-deny cargo-audit cargo-llvm-cov`

Recommended evidence extras:
- `yay -S --needed python-pdfplumber python-pytesseract python-pymupdf python-pillow pdfcpu pdftk imagemagick exiftool scantailor-advanced unpaper`

Optional GPU stack (NVIDIA):
- `yay -S --needed cuda cudnn python-opencv-cuda python-onnxruntime-opt-cuda python-pytorch-opt-cuda`

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
