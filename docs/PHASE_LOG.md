# Phase Log -- Cross-Reference Index

This is the single entry point for historical phase documentation in the
`4-bit` workspace. Canonical current status lives in `mcs4-emu/CLAUDE.md`;
this file points at the per-phase snapshots that contain detailed session
notes for archaeology.

## Current authoritative status

- `mcs4-emu/CLAUDE.md` -- canonical phase percentages, test counts, priorities.
- `mcs4-emu/STATUS.md` -- session log and chip status tables.
- `docs/ROADMAP.md` -- forward plan and dependency ordering.

`scripts/status_sync_check.sh` enforces consistency between these three.

## Per-phase snapshots

| Phase | Topic | Status | Snapshot doc |
|-------|-------|--------|--------------|
| 0 | Repo hygiene and reproducibility | COMPLETE | `mcs4-emu/CLAUDE.md` (Phase 0 entry) |
| 0.5 | OCR pipeline and evidence extraction | COMPLETE (90%) | `docs/evidence/PHASE_0.5_COMPLETION.md`, `PHASE_0.5_ADDENDUM_OCR_CACHE.md`, `PHASE_0.5_CACHE_IMPLEMENTATION.md`, `PHASE_0.5_1_COMPLETION_SUMMARY.md` |
| 1 | 4004 CPU correctness | COMPLETE | `docs/evidence/PHASE_1_COMPLETION_SUMMARY.md` |
| 2 | 4040 CPU execution | COMPLETE | `docs/archive/PHASE_2_STATUS.md`, `docs/archive/PHASE_2_DEBUG_NOTES.md`, `docs/archive/PHASE_2_CHECKPOINT.md` (snapshot-disclaimed; archived 2026-07-09) |
| 3 | Support chips and GUI | COMPLETE | `docs/archive/PHASE_3_STATUS.md` (cosmic-stream forward-plan section redacted 2026-04-30; archived 2026-07-09) |
| 4 | Clustering, SIMD, solver bridge, process models | COMPLETE | `docs/evidence/PHASE_4_COMPLETION_SUMMARY.md` (and `docs/archive/PHASE_4_STATUS.md` for the abandoned cosmic stream, archived 2026-04-30) |
| 5 | Peripherals, Intellec-4, FPGA design, Verilog | IMPLEMENTED (85%) | `docs/evidence/PHASE_5_COMPLETION_SUMMARY.md` |
| 6 | Gate extraction, subcircuit bridges, full-chip circuit sims | COMPLETE | session log entries in `mcs4-emu/STATUS.md`; planning context in `docs/archive/NEXT_STEPS.md` and `SCOPING_ASSESSMENT.md` (snapshot-disclaimed) |
| 7 | 3205/3404/2101 new chips, full 22-module behavioral Verilog | COMPLETE | session log entries in `mcs4-emu/STATUS.md` |
| 8 | iCE40/Spartan-7 constraints + synthesis Makefile | COMPLETE (90%; toolchain blocked) | session log entries in `mcs4-emu/STATUS.md` |

## Debt resolution

The current debt-elucidation roadmap lives at:

- Plan file: `~/.claude/plans/elucidate-and-build-out-merry-gadget.md`

Phases there are namespaced D0-D12 to avoid clashing with the project phase
numbering captured in this log.
