# Gemini Notes

## Summary
This repository preserves Intel 4-bit microprocessor documentation and hosts a
Rust workspace that emulates the MCS-4/MCS-40 family (4004/4040 and support chips).

## Workspace Layout
- Root is the canonical Cargo workspace.
- Rust crates live under `mcs4-emu/crates/`.
- Documentation and scans live under `docs/`.

## Toolchain and Policies
- Nightly pinned in `rust-toolchain.toml` (2026-01-06) for portable_simd.
- MSRV baseline: Rust 1.92.0 (stable).
- Warnings are errors (`.cargo/config.toml`, `cargo clippy-all` alias).

## Build and Test
- `cargo build --workspace --locked`
- `cargo test --workspace`
- `cargo clippy-all`
- `cargo fmt --all`

## Artifacts
- Build output: `target/`
- Coverage output: `coverage/` (if enabled)
- Cleanup: `scripts/clean.sh`

## Known Gaps
- No CI workflow for build/test/clippy yet (docs-only workflow exists).
- Primary MCS-4 datasheets are scanned; OCR needed for transistor/clock verification.
- See `docs/AUDIT.md` and `docs/ROADMAP.md` for ongoing validation work.
