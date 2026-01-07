# Documentation Quality Checklist
- README up-to-date
- Build/run instructions
- Architecture documented
- API documented with examples
- Configuration documented
- Troubleshooting
- Roadmap synced with STATUS
- Warnings treated as errors (build, test, lint)
- Automation scripts pass
- Clean script removes artifacts (target/coverage)

## Latest Verification
- 2026-01-06: `cargo fmt --all -- --check`
- 2026-01-06: `cargo clippy-all`
- 2026-01-06: `cargo test --workspace --locked`
- 2026-01-06: `scripts/doc_validate.sh`
- 2026-01-06: `scripts/doc_validate.sh` (post-audit update)
- 2026-01-06: `cargo fmt --all -- --check` (edition 2024)
- 2026-01-06: `cargo clippy-all` (edition 2024)
- 2026-01-06: `cargo test --workspace --locked` (edition 2024)
- 2026-01-06: `scripts/doc_validate.sh` (post-edition update)
- 2026-01-07: `scripts/doc_validate.sh` (post-OCR/tooling updates)
- 2026-01-07: `scripts/doc_validate.sh` (evidence trails)
- 2026-01-07: `scripts/doc_validate.sh` (artifact catalog)
- 2026-01-07: `scripts/doc_validate.sh` (photomicrograph links)
- 2026-01-07: `scripts/doc_validate.sh` (provenance notes)
- 2026-01-07: `scripts/doc_validate.sh` (photomicrograph updates)
- 2026-01-07: `scripts/doc_validate.sh` (siliconprawn references)
- 2026-01-07: `scripts/doc_validate.sh` (installation tooling update)
- 2026-01-07: `scripts/doc_validate.sh` (4001-4003 overlays)
- 2026-01-07: `scripts/doc_validate.sh` (catalog OCR updates)

## Open Warnings
- OCR pipeline emits Ghostscript 10.6 JPEG warnings and tesseract diacritic alerts; upgrade Ghostscript or rerun with higher-quality scans.
- Chunked OCR of the 1975 Intel Data Catalog still reports large output-size warnings due to --force-ocr; consider skip-text or higher-quality scans.
