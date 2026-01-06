# Installation Requirements

## Toolchain (Required)
- Rust nightly pinned in `rust-toolchain.toml` (2026-01-06) with components: rustfmt, clippy, miri, llvm-tools-preview.
- MSRV (stable baseline): Rust 1.92.0 (latest stable as of 2025-12-11).
  - Source: https://github.com/rust-lang/rust/releases/tag/1.92.0
- Cargo resolver v2 (workspace already uses `resolver = "2"`).
- Workspace root is the repository root; run Cargo commands from the repo root.

## System Packages (Linux)
- Base build tools (compiler, linker, pkg-config).
- GUI (eframe/winit) deps for Linux:
  - Debian/Ubuntu example:
    `sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev`
  - Source: https://github.com/emilk/egui/blob/master/crates/eframe/README.md

## Optional Tooling
- `cargo-deny` (policy checks), `cargo-audit` (advisories).
- `cargo-llvm-cov` for coverage output into `coverage/`.
- `yq` for docs registry validation in `scripts/doc_validate.sh`.

## Build and Test
- `cargo build --workspace --locked`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -D warnings`

## Module Notes
- `mcs4-core`, `mcs4-bus`, `mcs4-chips`, `mcs4-system`: no native deps beyond Rust toolchain.
- `mcs4-gui`: requires X11/Wayland runtime; run `cargo run -p mcs4-gui`.
- `mcs4-fpga`: currently stub; no external tooling required yet.
