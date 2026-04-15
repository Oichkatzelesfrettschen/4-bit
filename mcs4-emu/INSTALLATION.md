# Installation Requirements

## Toolchain (Required)
- Rust nightly pinned in `rust-toolchain.toml` (`nightly-2026-04-05`) with components: rustfmt, clippy, miri, llvm-tools-preview.
- Edition: 2021 (workspace-wide).
- MSRV (stable baseline): Rust 1.92.0 (latest stable as of 2026-01-05).
  - Source: https://github.com/rust-lang/rust/releases/tag/1.92.0
- Cargo resolver v2 (workspace already uses `resolver = "2"`).
- Workspace root is the repository root; run Cargo commands from the repo root.

## System Packages (Linux)
- Base build tools (compiler, linker, pkg-config).
- GUI (eframe/winit) deps for Linux:
  - Debian/Ubuntu example:
    `sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev`
  - Arch/CachyOS example:
    `sudo pacman -S --needed libxcb libxkbcommon libxrender openssl`
  - Source: https://github.com/emilk/egui/blob/master/crates/eframe/README.md

## Docs and Validation Tooling
- `yq` for `scripts/doc_validate.sh`.
- `cargo-deny` (policy checks), `cargo-audit` (advisories).
- `cargo-llvm-cov` for coverage output into `coverage/`.
- PyMuPDF (`python-pymupdf` on Arch/CachyOS) for robust PDF page extraction (`import fitz`).
- OCR toolchain for scanned PDFs (Arch/CachyOS): `yay -S --needed ocrmypdf jbig2enc` (pulls unpaper/pngquant/img2pdf/python-pikepdf).
- OCR toolchain for scanned PDFs (Debian/Ubuntu): `sudo apt-get install ocrmypdf tesseract-ocr tesseract-ocr-eng poppler-utils`.

## Optional Analysis Tooling
- Download accelerators: `aria2`, `axel`, `wget2`.
- PDF utilities: `pdfcpu`, `pdftk`, `qpdf`.
- Image/OCR utilities: `gimp`, `gimagereader-qt`, `gocr`, `scantailor-advanced`, `imagemagick`, `graphicsmagick`.
- Layout/netlist tooling: `klayout`, `xschem`, `ngspice`, `netlistsvg`.
- Metadata inspection: `exiftool`, `mediainfo`.

## Build and Test
- `cargo build --workspace --locked`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`

## GPU-Accelerated OCR (Optional)

Repo-default OCR is **Tesseract (CPU)** for reproducible extraction. For higher throughput experiments (page rasterization + label OCR),
you can optionally enable a GPU-backed ONNX stack:

- `python-onnxruntime-opt-cuda` (provides `onnxruntime` with `CUDAExecutionProvider`)
- `python-opencv-cuda` (optional CUDA resize/preproc helpers used by some scripts)

Backends fall back in this order:
- ONNX (CUDA) → ONNX (CPU) → Tesseract (CPU)

To plug in an ONNX model without committing weights into git:
- Set `OCR_ONNX_MODEL=/abs/path/to/model.onnx` (optionally with sidecar JSON: `model.onnx.json` containing `alphabet` and `blank_id`)

Note: some tools (including Codex image attachment) cannot display PNM variants. Convert PBM/PGM/PPM to PNG with:
- `python3 scripts/convert_pnm_to_png.py --recursive --out-dir /tmp/pngs <dir-containing-pbm>`

## Fixture Runner (No GUI Interaction)
The `mcs4-emu` binary supports running `.hex` fixtures from `mcs4-emu/crates/mcs4-system/fixtures`:

- Run a fixture: `cargo run -p mcs4-gui -- --mode fixture --system mcs4 --fixture src_wrm_rdm --cycles 12`
- Provide ROM port input (for `RDR` fixtures): `--rom-io-input 12 --rom-io-chip 0`

## Module Notes
- `mcs4-core`: pure Rust; no native deps.
- `mcs4-bus`: pure Rust; no native deps.
- `mcs4-chips`: pure Rust; no native deps.
- `mcs4-system`: pure Rust; no native deps.
- `mcs4-gui`: requires X11/Wayland runtime; run `cargo run -p mcs4-gui`.
- `mcs4-fpga`: Verilog export for 4 chip modules (12 tests); no external tooling required.
- `mcs4-periph`: pure Rust; peripheral devices (7-segment, keyboard, UART; 30 tests).
- `mcs4-intellec`: pure Rust; Intellec-4 development system (front panel, monitor, PROM programmer; 44 tests).
