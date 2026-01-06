# Development Guide

## Tools
- cargo, clippy, rustfmt.
- Optional: cargo-deny, cargo-audit, cargo-llvm-cov.
- go-yq/yq for docs registry validation (scripts expect the Go yq CLI).
- OCR: tesseract + tesseract-data-eng for scanned datasheets.
- CMake: for optional native tooling, keep separate.
- Toolchain baseline: MSRV 1.92.0; nightly 2026-01-06 required for portable_simd.

## Automation
- scripts/clean.sh to remove build, coverage, and core dump artifacts.
- scripts/doc_sync.sh, scripts/doc_validate.sh for docs registry sync/validation.
- scripts/md_validate.sh, scripts/md_lint.sh, scripts/link_check.sh for docs checks.
- Keep `mcs4-emu/STATUS.md` and `docs/ROADMAP.md` synchronized.
- Use `cargo clippy-all` alias to enforce `-D warnings`.
- CI: `.github/workflows/ci.yml` runs fmt/clippy/tests on nightly.

## Warnings Policy
- `.cargo/config.toml` enforces `-D warnings` for builds; clippy uses `-D warnings`.
