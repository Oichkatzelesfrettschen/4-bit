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

## Enforcement model
Both GitHub workflows trigger on `workflow_dispatch` only (quota hold; the
lift decision is tracked as `docs/DEBT_ROADMAP.md` task D11.1). Until the
hold lifts, every gate above is enforced locally -- `just verify` for the
Rust, Python, script, and 4003 gate-HDL structural surfaces,
`scripts/doc_validate.sh` for docs -- and by manual workflow dispatch before
merges.

Tracked local Markdown and YAML targets, Markdown anchors, and reference-style
local links are required to resolve offline. Run `just link-check`; the Docs CI
validation job runs the same verifier before deployment can proceed.

## Latest Verification

- 2026-07-12: `cargo fmt --all -- --check`, strict workspace clippy, `cargo test --workspace --locked`,
  scripts/doc_validate.sh, scripts/md_validate.sh, scripts/status_sync_check.sh, and `just link-check`
  (pass; 1,159 Rust tests and 77 Python tests)
- 2026-07-12: `cargo test --workspace --locked` (1,159 passed / 0 failed / 0 ignored)
- 2026-07-12: `just virtual-fpga-release-check` (Qt6 and Verilator release build;
  5 headless scenario tests passed, including the common behavioral and FPGA stimulus)
- 2026-07-12: `scripts/callgraph_capture.sh target/callgraph-capture-20260712-common-stimulus`
  (1,241 checksummed artifacts; 67 status records, including 64 required records; all zero exit; 336
  source inputs archived and hash-verified; 169 Rust, 124 Python, 19 Verilog,
  and 3 C++ or header sources; dirty-tree provenance retained and verified)
- 2026-07-11: `cargo clippy --workspace --locked --all-targets --all-features -- -D warnings` (clean)
- 2026-07-11: `cargo fmt --all -- --check` (clean)
- 2026-07-11: `ruff check --no-cache scripts/` (clean)
- 2026-07-11: `shellcheck -S warning scripts/*.sh` (clean)
- 2026-07-11: `just link-check` (clean; tracked local Markdown links, anchors, and
  reference-style links resolve offline)
- 2026-07-12: `python3 -m pytest scripts/tests` (77 passed; export, capture, capability, timing, netlist, transaction, clock-contract, local-link, and common-stimulus comparison coverage included)
- 2026-07-11: `python3 scripts/gate_to_verilog_v0.py --chips 4003 --check-export-contract --check-generated` (pass; Q4 resolves through declared inputs and retained HDL matches source evidence)
- 2026-07-11: `make -C mcs4-emu/crates/mcs4-fpga sim CHIP=4003 MODE=gate` (pass; all eight input vectors resolve without X or Z)
- 2026-07-11: `cargo llvm-cov --workspace --all-features --summary-only` (91.07% regions, 87.71% functions, 89.44% lines)
- 2026-07-11: `cargo deny check advisories --config deny.toml` and `python3 scripts/verify_advisory_exceptions.py` pass. Live audit reports four registered time-bounded debt records, not a clean audit.
- 2026-07-11: `scripts/doc_validate.sh` + `scripts/status_sync_check.sh` must pass after every canonical-status edit.
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
- quick-xml 0.38.4 has two registered vulnerability advisories through the GUI
  dependency stack. paste and ttf-parser have separate unmaintained-dependency
  records. Dependency remediation remains required before 2026-10-01.
- OCR pipeline emits Ghostscript 10.6 JPEG warnings and tesseract diacritic alerts; upgrade Ghostscript or rerun with higher-quality scans.
- Chunked OCR of the 1975 Intel Data Catalog still reports large output-size warnings due to --force-ocr; consider skip-text or higher-quality scans.
