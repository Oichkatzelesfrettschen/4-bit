# Claude Notes

## What This Repo Is
- Documentation archive for Intel's 4-bit microprocessors (4004/4040 + MCS-4/MCS-40 families).
- Rust workspace implementing an emulator/simulator stack (`mcs4-emu/crates/*`).

## Where Things Live
- Canonical Cargo workspace: repo root (`Cargo.toml`).
- Rust crates: `mcs4-emu/crates/`
- Documentation + evidence: `docs/` (registry: `docs/meta/registry.yaml`)
- Evidence OCR sidecars and provenance notes: `docs/evidence/`

## Quality Gates (Treat Warnings As Errors)
- Rust warnings are denied via `.cargo/config.toml` (`-D warnings` + `RUSTDOCFLAGS=-D warnings`).
- CI runs: `cargo fmt -- --check`, `cargo clippy-all`, `cargo test --workspace --locked`.
- Docs CI runs: `scripts/doc_validate.sh`, `scripts/doc_sync.sh`, `scripts/md_lint.sh`, `scripts/md_validate.sh`, `scripts/link_check.sh`.

## Requirements + TODO Tracking
- Stable requirements entrypoint: `requirements.md` (canonical details in `mcs4-emu/INSTALLATION.md`).
- TODO scan generator: `scripts/todo_scan.sh` → `docs/TODO.md`.

## Status File Convention

- `mcs4-emu/CLAUDE.md` = canonical status (phase %, test counts, priorities) -- single source of truth
- `docs/ROADMAP.md` = forward plan (what to build next, dependency order)
- `mcs4-emu/STATUS.md` = session log and chip status tables
- All three synchronized after each milestone; contradictions are bugs.

## Known Gaps
- Primary-source confirmation for 4004/4040 transistor counts and IPS is still pending; see `docs/AUDIT.md`.
- 4040 die shots and layer imagery are not yet in-repo; see `docs/CHIP_ARTIFACTS.md`.
