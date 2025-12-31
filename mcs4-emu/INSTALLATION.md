# Installation Requirements

## Workspace Prerequisites
- Rust toolchain (1.75+), cargo, rustfmt, clippy
- Linux build tools: cmake (optional), perf (optional)
- GUI: egui/eframe via cargo

## Build
- cargo build --workspace --locked
- cargo test --workspace

## Module Notes
- mcs4-chips: no native deps; enable benches with `criterion`
- mcs4-gui: needs X11/Wayland runtime; run `cargo run -p mcs4-gui`
