# Changelog

## 2025-12-31
- Added documentation skeleton and metadata registry.
- Began 4040 scaffolding.

## 2026-01-06
- Added bitsavers URLs to audit sources and documented derived 4040 clock period interpretation.
- Updated README with system clock period note and discrete-transistor claim source link.
- Expanded roadmap to track Rust edition upgrade audit.
- Upgraded workspace and rustfmt edition to 2024 and resolved clippy collapsible-if warnings.
- Re-ran fmt, clippy, and workspace tests after edition update.
- Updated installation and gemini notes to reflect Rust 2024 edition and OCR status.
- Added OCR toolchain guidance (ocrmypdf/jbig2enc) and refreshed audit sources for 4040 clock period and 4004 netlist counts.
- Consolidated workspace configuration at repo root.
- Added audit log and updated installation, deployment, and roadmap docs.
- Corrected 4040 clock-rate entry in README.
- Pinned nightly toolchain to 2026-01-06.
- Added MSRV 1.92.0 metadata and workspace lint inheritance.
- Added CI workflow for fmt/clippy/tests; doc sync now preserves registry section.
- Updated ARCHITECTURE and AUDIT with primary-source references and gaps.
- Added gemini.md summary and expanded roadmap tasks.
- Added MIT and Apache-2.0 license files at repo root.
- OCR-verified MCS-4 datasheet timing; refreshed audit for chip design references.
- Clarified installation tooling (go-yq, OCR) and refreshed docs index timestamps.
- Updated clean script to include core dumps and corrected find logic.
- Added OCR-derived clock-period citations and noted remaining 4040 clock gaps.
- Corrected MCS-40 device roles and flagged unverified process/implementation claims in README.
- Logged latest fmt/clippy/test/doc validation runs in quality checklist.
