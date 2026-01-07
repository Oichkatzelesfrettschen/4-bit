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
- Edition: 2024 (workspace-wide).
- OCR toolchain: tesseract + ocrmypdf (+ jbig2enc, poppler-utils) for searchable scans.

## Evidence Trails
- Evidence files live under `docs/evidence/` with OCR sidecars and hashes.
- See `docs/evidence/ocr_results.md` for clock period evidence and netlist component counts.

## Build and Test
- `cargo build --workspace --locked`
- `cargo test --workspace`
- `cargo clippy-all`
- `cargo fmt --all`
- `scripts/doc_validate.sh`

## Artifacts
- Build output: `target/`
- Coverage output: `coverage/` (if enabled)
- Cleanup: `scripts/clean.sh`

## Known Gaps
- Primary MCS-4/MCS-40 datasheets are scanned; OCR verified timing (4040 clock period 1.35-2.0 usec), but transistor counts remain unverified in primary docs.
- Forensic netlist counts for the 4004 are documented on 4004.com (transistor/component breakdown).
- Transistor-level simulation stubs and FPGA netlist export remain placeholders.
- See `docs/AUDIT.md` and `docs/ROADMAP.md` for ongoing validation work.
