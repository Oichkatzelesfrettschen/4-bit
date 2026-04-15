# Troubleshooting

## Build Fails

### Missing system libraries (Linux)

The GUI crate (mcs4-gui) depends on egui/eframe which requires X11/Wayland libraries.

**Debian/Ubuntu:**
```sh
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libssl-dev
```

**Arch/CachyOS:**
```sh
sudo pacman -S --needed libxcb libxkbcommon libxrender openssl
```

### Nightly toolchain not found

This project requires a specific nightly Rust toolchain pinned in `rust-toolchain.toml`.
If `rustup` cannot find it, run:
```sh
rustup toolchain install nightly-2026-04-05 --profile minimal \
  --component rustfmt,clippy,miri,llvm-tools-preview
```

### MSRV issues (stable baseline)

The workspace declares `rust-version = "1.92"` but some features (portable_simd) require
nightly. If building without nightly, use `cargo +1.92 check --workspace` (check only, not
test). The `simd` feature is behind a feature gate and will not compile unless requested.

### Cargo.toml duplicate keys

If you see "duplicate key" errors, verify that workspace dependencies appear only once in the
root `Cargo.toml` `[workspace.dependencies]` section. Crate-level `Cargo.toml` files should
use `dep.workspace = true` syntax.

## GUI Errors

### GUI does not launch (X11)

Ensure your display server is running and `DISPLAY` is set. For Wayland, you may need
`WAYLAND_DISPLAY` set. If running over SSH, use X forwarding (`ssh -X`) or a VNC session.

### egui/eframe version mismatch

If you see trait-mismatch errors between egui and eframe, verify that both are pinned to the
same version in the workspace dependencies (currently 0.33).

## Test Failures

### All tests should pass

Run `cargo test --workspace` from the repo root. If tests fail:
1. Verify you are on the correct nightly toolchain (`rustup show`).
2. Verify the build is clean: `cargo clean && cargo test --workspace`.
3. Check `mcs4-emu/CLAUDE.md` for the expected test count baseline.

### Clippy warnings treated as errors

The workspace enforces `-D warnings` via `.cargo/config.toml`. Fix all warnings before
committing. Run: `cargo clippy --all-targets -- -D warnings`

## OCR Pipeline

### Tesseract not found

Install the OCR toolchain:
- Arch: `yay -S --needed ocrmypdf jbig2enc`
- Debian: `sudo apt-get install ocrmypdf tesseract-ocr tesseract-ocr-eng`

### GPU-accelerated OCR not available

The default OCR backend is Tesseract (CPU). For GPU acceleration, install
`python-onnxruntime-opt-cuda`. Backends fall back automatically:
ONNX (CUDA) -> ONNX (CPU) -> Tesseract (CPU).

## Performance

### Benchmarks show regression

Benchmarks in CI use a 20% regression threshold. If benchmarks fail locally, ensure you are
not running other CPU-intensive tasks. For consistent results:
```sh
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo bench --workspace
```
