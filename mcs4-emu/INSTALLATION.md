# Installation Requirements

## Toolchain (Required)
- Rust nightly pinned in `rust-toolchain.toml` (2026-01-06) with components: rustfmt, clippy, miri, llvm-tools-preview.
- Edition: 2024 (workspace-wide).
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
- `go-yq` (Arch/CachyOS) or `yq` (Debian/Ubuntu) for `scripts/doc_validate.sh`.
- `cargo-deny` (policy checks), `cargo-audit` (advisories).
- `cargo-llvm-cov` for coverage output into `coverage/`.
- OCR toolchain for scanned PDFs (Arch/CachyOS): `yay -S --needed ocrmypdf jbig2enc` (pulls unpaper/pngquant/img2pdf/python-pikepdf).
- OCR toolchain for scanned PDFs (Debian/Ubuntu): `sudo apt-get install ocrmypdf tesseract-ocr tesseract-ocr-eng poppler-utils`.

## Build and Test
- `cargo build --workspace --locked`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -D warnings`

## Module Notes
- `mcs4-core`: pure Rust; no native deps.
- `mcs4-bus`: pure Rust; no native deps.
- `mcs4-chips`: pure Rust; no native deps.
- `mcs4-system`: pure Rust; no native deps.
- `mcs4-gui`: requires X11/Wayland runtime; run `cargo run -p mcs4-gui`.
- `mcs4-fpga`: stub targets; no external tooling required yet.
