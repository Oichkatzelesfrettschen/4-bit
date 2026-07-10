# Debt Roadmap: Multi-Surface Audit and Resolution Plan

This document is the durable, in-repo registry for the debt-resolution
program. Earlier debt phases (D0, D1, D3, D4.1/D4.2/D4.5, D5.4, D10.x)
are recorded only in commit messages (9ffb9b0, f55d706) and in a per-user
plan file (`~/.claude/plans/elucidate-and-build-out-merry-gadget.md`) that
no longer exists although `mcs4-system/src/lib.rs` cited it. That gap --
the debt registry itself living outside the repository -- is closed by
this file. New phases carry mechanism-first names; the D-codes are
secondary metadata continuing the established numbering from D11 to avoid
collision with prior phases.

## Audit provenance (2026-07-09)

Five bounded sweeps (Rust code, tests, build environment + Python,
documentation + organization, Verilog + FPGA) plus direct instrumented
capture. Baseline measured, not quoted:

| gate | result |
|------|--------|
| `cargo test --workspace` | 1,053 passed / 0 failed / 1 ignored (matches canonical docs) |
| `cargo clippy --workspace --all-targets` | FAILED at audit time (manual_range_contains in proptest_solvers.rs:299); fixed in this pass, now clean |
| `scripts/status_sync_check.sh` | FAILED at audit time (STATUS.md 1,042 vs CLAUDE.md 1,053); fixed in this pass, now green |
| `scripts/doc_validate.sh` | pass |
| `ruff check --select E,F,W,B,S scripts/` | 718 findings (643 E501; 18 B023; 9 S110/S112; 6 F401) |
| `shellcheck -S warning scripts/*.sh` | 3 findings |
| `verilator --lint-only -Wall` (gowin RTL) | 3 errors without generated build/ modules; masked by 9 -Wno flags in Makefile lint target |
| `iverilog -t null` (4 gate-level netlists) | all pass, <0.2 s each |

Call-graph maps of the load-bearing paths (solver core, chip bridge,
schematic pipeline) are retained under `docs/evidence/callgraphs_v0/`.

Refuted finding (recorded so it is not re-raised): a sweep reported
`deny.toml` missing; it exists at the repo root with real cargo-deny
configuration. Verified 2026-07-09.

## Load-bearing observations

1. Gate authoring is complete; gate enforcement is dormant. ci.yml and
   docs.yml carry fmt, clippy -D warnings, --locked tests, miri over 8
   crates, cargo-deny, cargo-audit, semver-checks, a no-default-features
   matrix, pytest smoke, and 30 s fuzz smoke -- and both workflows
   trigger on `workflow_dispatch` only ("Quota hold"). The measurable
   consequence: two regressions (clippy failure, status-count drift)
   entered main while the gates were dormant and were caught only by this
   audit's direct run. The debt is one trigger stanza, not missing
   authorship.
2. One generator stub cascades across the hardware surface.
   `gate_to_verilog_v0.py` never extracts I/O (line 338 TODO), so all
   four gate-level netlists expose only VDD/VSS ports; therefore the four
   testbenches cannot drive anything, the default `MODE=gate` synthesis
   target would produce an empty bitstream, and the .pcf/.xdc constraint
   files reference ports no module has. The unblock data already exists:
   `subcircuits_v0/*/manifest.json` carries named outputs (CLOCK, DATA,
   EN, OUT, Q0..Q9) with seed anchor node ids. This is a wiring task with
   subsystem-wide leverage, not a research task.
3. The status checksum works when summed. STATUS.md drift was detectable
   because its per-suite rows summed to 1,042 while the headline claimed
   the canonical total. `status_sync_check.sh` compares headlines across
   files; asserting sum-of-rows == headline inside each file catches the
   class, not just the instance.
4. pMOS ratioed logic defeats CMOS gate recognition. Gate extraction
   yields zero inverters on all four chips because the Intel 10 um
   process has a single transistor type: an inverter is an
   enhancement-mode pull-down with a depletion/resistive load, not a
   complementary pair. Classification needs a ratioed-logic cell library
   (enhancement driver network + load device topologies), and the
   emitted primitive library's tgate branch has never seen real data.
5. simd_cluster.rs is scheduled work parked as rot. 2,040 lines, never
   declared as a module, 147 build errors under its (inert) feature flag,
   tracked in a plan file that no longer exists. It needs an explicit
   resurrect-or-retire decision, not continued limbo.

## Phase gate-enforcement-reactivation (meta: D11)

Objective: every authored gate either enforces automatically or is
explicitly documented as manual-only with an owner decision.

| task | anchor | action | acceptance |
|------|--------|--------|------------|
| D11.1 | .github/workflows/ci.yml:5, docs.yml:5 | DECISION (owner): lift quota hold to `on: [push, pull_request]`, or adopt scheduled/nightly, or keep manual | decision recorded here; workflows match it |
| D11.2 | ci.yml (absent job) | add ruff job: `ruff check scripts/` | job present; repo ruff-clean at gated select |
| D11.3 | ci.yml (absent job) | add shellcheck job over scripts/*.sh | job present; 0 warnings at -S warning |
| D11.4 | scripts/pyproject.toml:26 | add "S" to ruff lint select | S-rules run locally and in CI |
| D11.5 | scripts/status_sync_check.sh | assert per-file sum-of-rows == headline test count | drift class caught, verified by mutation test |
| D11.6 | docs/QUALITY-CHECKLIST.md:8,14-32 | reconcile claims with D11.1 outcome; refresh stale 2026-01 verification log | checklist states what is actually enforced |
| D11.7 | justfile:15 | align `just lint` with CI clippy-all alias (add --all-features) | identical lint surface local vs CI |
| D11.8 | rustfmt.toml:1 | set edition = "2021" to match workspace edition | fmt idioms match crate edition |
| D11.9 | ci.yml:52 | benchmark step: remove continue-on-error or mark advisory in-line | no silently-ignored failing gate |

## Phase dependency-and-error-type-hygiene (meta: D12)

Objective: manifests declare exactly what compiles; error types are
uniform per layer.

| task | anchor | action | acceptance |
|------|--------|--------|------------|
| D12.1 | Cargo.toml [workspace.dependencies] | remove anyhow, parking_lot (0 refs); decide thiserror: adopt for FixtureError or remove | cargo build clean; deny-check green |
| D12.2 | mcs4-bus/src/data_bus.rs:165,209 | pick one of bytemuck / zerocopy for byte-casting; drop the other derive + dep | one zero-copy stack workspace-wide |
| D12.3 | mcs4-chips/Cargo.toml | remove unused deps: mimalloc feature, bytemuck, zerocopy, seq-macro, num-traits, bitflags, rkyv, smallvec | crate builds with only used deps |
| D12.4 | mcs4-core/Cargo.toml, mcs4-bus/Cargo.toml | remove unused bytemuck/zerocopy (core), rkyv (bus) | same |
| D12.5 | mcs4-chips/src/disasm.rs:58 | replace Result<_, String> with DisasmError enum | stringly-typed error gone; callers updated |
| D12.6 | mcs4-core/src/layout_netlist.rs:56 | wrap io::Error in typed loader error consistent with FixtureError pattern | uniform loader error surface |
| D12.7 | mcs4-chips/src/i4211.rs:88 | restructure mod-4 match to cover 0..=3 without unreachable!() | no panic path in non-test code |
| D12.8 | mcs4-gui/src/panels/waveform.rs:279 | handle poisoned trace lock without panicking the render loop | GUI renders error state instead of aborting |
| D12.9 | mcs4-system/src/lib.rs:109 | add SAFETY comment to test mmap block | both unsafe sites documented |
| D12.10 | Cargo.toml:20-64 | pin workspace dep minor versions | manifests reproducible without lockfile |

## Phase test-and-claims-coverage (meta: D13)

Objective: every headline claim has a named asserting test; parser
surfaces are fuzzed; the largest untested module gains direct coverage.

| task | anchor | action | acceptance |
|------|--------|--------|------------|
| D13.1 | mcs4-chips/src/i4004/mod.rs (558 L, 0 tests) | unit tests for tick()/pc/accumulator/carry/ram_address phase orchestration | direct coverage of 4004 top-level state machine |
| D13.2 | mcs4-core/src/netlist_v0.rs:136 | add fuzz target netlist_v0_parser | 4th fuzz target with seed corpus, in fuzz smoke |
| D13.3 | docs/CLAIMS_TO_TESTS.md:13 | add test asserting 4004 instruction cycle = 10.8 us at nominal clock | named test exists and is cited |
| D13.4 | docs/CLAIMS_TO_TESTS.md:14 | add decoder census test: 4040 opcode set size == 60 (14 new) | named test exists and is cited |
| D13.5 | docs/CLAIMS_TO_TESTS.md:11-37 | replace bare `cargo test` enforcement cells with crate::module::test_fn names | machine-checkable claims map |
| D13.6 | mcs4-fpga/src/verilog.rs:2770 | #[ignore = "codegen helper; run with --ignored"] | reason string present |
| D13.7 | mcs4-fpga/src/verilog.rs:2476 | assert emitted Verilog content, not just is_ok() | export test checks ports/decls |
| D13.8 | mcs4-chips/src/i4040/stack.rs:60 | assert popped values equal pushed sequence | value-checking assertion |
| D13.9 | mcs4-bus, mcs4-gui, mcs4-periph | add tests/ integration dirs (bus cycle end-to-end; waveform logic; uart+keyboard+7seg) | each crate has at least one integration suite |

## Phase verilog-port-extraction-and-testbench (meta: D14)

Objective: gate-level netlists gain functional I/O; testbenches drive
real stimulus; constraints match a real top.

| task | anchor | action | acceptance |
|------|--------|--------|------------|
| D14.1 | scripts/gate_to_verilog_v0.py:338 | derive module ports from subcircuit manifest outputs[].name + seed.nodes | i*_gates.v tops expose functional ports |
| D14.2 | docs/evidence/verilog_v0/* | regenerate all 4 chips after D14.1 | iverilog pass; ports beyond VDD/VSS |
| D14.3 | tb_i400{1,2,3,4}_gates.v:21 | replace TODO stub with two-phase non-overlapping clock, reset, per-chip vectors | simulation exercises DUT; waveform sanity-checked |
| D14.4 | constraints/mcs4_ice40.pcf, mcs4_spartan7.xdc | reconcile pin names to the real post-D14.1 top (or mcs4_top); use self-consistent Gowin .cst as template | every constrained pin exists on its top module |
| D14.5 | mcs4-fpga/Makefile:75-181 | guard each tool target with command -v; message and skip when absent | absent toolchains fail gracefully |
| D14.6 | mcs4-fpga/Makefile:107-126 | SHELL := /bin/bash or convert Vivado heredocs to -source file.tcl | targets run under default make shell |
| D14.7 | mcs4-fpga/build/ | gitignore generated .v/.vcd/sim binaries or document as checked-in artifacts with regen provenance | no stale-vs-source ambiguity |
| D14.8 | mcs4-fpga/coverage/*.profraw | remove ~180 stale profraw files; gitignore | no coverage clutter in crate tree |
| D14.9 | gate_to_verilog_v0.py:128-133 | exercise tgate lowering on a chip with pass devices, or remove the dead branch | no never-executed emission path |

## Phase python-packaging-and-pipeline-hardening (meta: D15)

Objective: the 94-script evidence pipeline has pinned dependencies,
gated lint, and per-family smoke coverage.

| task | anchor | action | acceptance |
|------|--------|--------|------------|
| D15.1 | scripts/pyproject.toml | add [project.dependencies] with == pins (Pillow, numpy, opencv-python, pytesseract) matching INSTALLATION.md prose | machine-readable pinning exists |
| D15.2 | ci.yml:279 | pin pip installs (pytest==, pyyaml==) | CI installs reproducible |
| D15.3 | scripts/*.py | ruff check --fix for the 9 auto-fixables (F401/F841/F541); then burn down B023 (18 loop-var closures, 16 in extract_netlist_v0.py:266) | ruff clean at gated select |
| D15.4 | S110/S112 sites (9) | add logging to bare except bodies | no silent error swallowing |
| D15.5 | detect_layout_edge_labels_v0.py:33, extract_netlist_v0.py:529 | hashlib usedforsecurity=False | S324 clean |
| D15.6 | scripts/tests/ | add smoke tests for top extractor families (extract_netlist, extract_gates, build_netlist_v1, ocr_signal_labels) | >= 1 test per major family; 4 -> ~12 tests |
| D15.7 | check_ocr_versions.sh:9, fetch_sources.sh:37, fetch_sources_test.sh:86 | fix SC2034 x2, SC3045 | shellcheck clean |
| D15.8 | E501 x643 | decide: reflow, or set gate to ignore E501 explicitly | lint config states intent; zero ungated noise |

## Phase docs-consolidation-and-archive (meta: D16)

Objective: one source of truth per fact class; snapshots archived;
the TODO scanner reports code debt, not noise.

| task | anchor | action | acceptance |
|------|--------|--------|------------|
| D16.1 | PHASE_2_CHECKPOINT.md, NEXT_STEPS.md (root); docs/PHASE_2_STATUS.md, PHASE_2_DEBUG_NOTES.md, PHASE_3_STATUS.md | move superseded snapshots to docs/archive/ with ARCHIVE-NOTE headers; update registry roles -- DONE 2026-07-09 | no stale snapshot in a live location |
| D16.2 | claude.md:22-27, STATUS.md:171-176, PHASE_LOG.md:8-14 | keep one Status File Convention block; others link | convention stated once |
| D16.3 | STATUS.md vs CLAUDE.md test breakdowns | CLAUDE.md TEST COUNTS is sole current source; STATUS keeps session log + links | no dual-maintained diverging tables |
| D16.4 | ARCHITECTURE.md / STATUS.md / SCOPING chip tables | single chip-status table (STATUS); others cross-reference | one table to update |
| D16.5 | scripts/todo_scan.sh | exclude .claude_plans/, docs/archive/, and doc-mentions of the word TODO; regenerate docs/TODO.md | tracker reports actionable markers only |
| D16.6 | guide/ (27 of 30 chapters are 42-297 byte stubs) | fill chapters or mark WIP in SUMMARY; add guide/ to registry or scope registry to docs/ explicitly | no silent stub book |
| D16.7 | claude.md (root) vs mcs4-emu/CLAUDE.md | DECISION (owner): rename root file or qualify all "CLAUDE.md is canonical" references with the path | no ambiguous canonical pointer |
| D16.8 | docs/CHIP_EXTRACTION_STATUS.md | rename mechanism-first (CHIP_EXTRACTION.md) with registry update | naming discipline holds for living docs |
| D16.9 | docs/archive/NEXT_STEPS.md:31 | correct 3,668 -> 3,705 kept transistors (or cite SCOPE:230) -- DONE 2026-07-09 | transistor census consistent |

## Phase simd-cluster-resurrection-or-retirement (meta: D17, continues D1.4.3/D2.2)

Objective: end the limbo of the orphaned 2,040-line SIMD module.

| task | anchor | action | acceptance |
|------|--------|--------|------------|
| D17.1 | mcs4-system/src/lib.rs:13-19, src/simd_cluster.rs | DECISION (owner): resurrect or retire | decision recorded here |
| D17.2a | (resurrect) | add #![feature(portable_simd)] gating, declare module under cfg(feature), burn down the 147 errors in compilable slices | --features simd_cluster builds and tests |
| D17.2b | (retire) | move file to docs/archive or a branch; delete inert feature from Cargo.toml; update lib.rs comment | no dead feature, no orphan file |

## Phase measurement-and-reproducibility (meta: D18, continues D10.x)

| task | anchor | action | acceptance |
|------|--------|--------|------------|
| D18.1 | ci.yml:55 | commit docs/evidence/benchmarks_baseline_v0.json or drop --baseline | benchmark step compares against real data |
| D18.2 | ci.yml, docs.yml action tags | pin actions to commit SHAs | no mutable tags |
| D18.3 | docs.yml:72 | mdbook URL: releases/download/v0.4.44/ (not latest/) | fetch cannot silently break |
| D18.4 | workflows | export SOURCE_DATE_EPOCH for doc builds | reproducible artifacts |
| D18.5 | deny.toml:11 | time-box RUSTSEC-2024-0436 ignore (paste unmaintained); review for replacement | ignore has expiry note |

## Phase scientific-fidelity (meta: D19)

Objective: extraction and simulation claims grounded in the actual
Intel 10 um pMOS process physics and the datasheet timing.

| task | anchor | action | acceptance |
|------|--------|--------|------------|
| D19.1 | gates_v0 classification (0 inverters on all chips) | build ratioed-logic cell library: enhancement pull-down networks + depletion/resistive load topologies; reclassify | inverter/NAND/NOR counts nonzero and reviewed against die photos |
| D19.2 | docs/NETLIST_V1_SCHEMA.md; subcircuit JSONs | add signals field (VDD/VSS/clock identification) to schema and emitters | solvers stop inferring rails heuristically |
| D19.3 | docs/evidence/PHASE_0.5_1_COMPLETION_SUMMARY.md:76 | replace identity-matrix homography placeholder with anchor-point computation | coordinate transform evidence real, not placeholder |
| D19.4 | process model (mcs4-core/src/process) | validate solver timing against ARCHITECTURE.md datasheet table (t0D1 400-550 ns, t0D2 150 ns) with a named test | simulation-vs-datasheet delta recorded |

## Execution order and dependencies

D11 first: it is the multiplier (dormant gates let every other class
regress silently). D12 and D13 are independent of each other and of D14.
D14.1 blocks D14.2/D14.3/D14.4 (the cascade). D15 and D16 are parallel.
D17 and D19 carry owner decisions and research-grade work; schedule after
the mechanical phases land. Every phase ends with the standing gate set:
`cargo test --workspace`, `cargo clippy --workspace --all-targets`
(clean), `scripts/status_sync_check.sh`, `scripts/doc_validate.sh`, and
after D11 the ruff/shellcheck jobs.

## Resolved in the audit pass that produced this document (2026-07-09)

- proptest_solvers.rs:299 manual range check -> RangeInclusive::contains
  (clippy gate green again).
- STATUS.md test total 1,042 -> 1,053 with proptest_solvers row and
  mcs4-system 45 -> 50 (status_sync_check green again).
- mcs4-emu/CLAUDE.md internal mcs4-fpga contradiction (24 vs 42) fixed;
  phase-block counts annotated as as-of-completion values.
- ARCHITECTURE.md module map mcs4-fpga 24 -> 42.
- Call-graph evidence bundle created (docs/evidence/callgraphs_v0/).
- mcs4-system/src/lib.rs debt pointer redirected from the missing
  per-user plan file to this document.

## Resolved in the execution pass (2026-07-09, same day)

Gate state after the pass: 1,057 tests / 0 failures / 1 ignored; clippy
clean at --workspace --all-targets --all-features; fmt clean at edition
2021; ruff clean; shellcheck clean; doc_validate + checksum-extended
status_sync_check green.

- D11 (all but D11.1): script_lint CI job (ruff + shellcheck), ruff "S"
  select with an explicit E501 ignore decision, status_sync_check
  sum-of-rows checksum (mutation-tested), justfile lint aligned to the CI
  clippy-all surface, rustfmt edition 2021, benchmark step labeled
  advisory, QUALITY-CHECKLIST enforcement-model section + fresh
  verification log.
- D11.1 DECISION: quota hold stays; both workflows remain
  workflow_dispatch. Every gate is enforced locally (see
  QUALITY-CHECKLIST enforcement model) and by manual dispatch before
  merges. Revisit when CI quota returns.
- D12 (all): 7 orphaned workspace deps removed (anyhow, thiserror,
  parking_lot, bytemuck, bitflags, num-traits, seq-macro); zerocopy kept
  as the single byte-cast stack (no call sites existed for either
  library; zerocopy derives carry compile-time checks); mcs4-chips
  reduced to tracing + workspace crates; DisasmError and NetlistLoadError
  enums replace String / io::Error / Box<dyn Error> loader errors;
  i4211 unreachable!() removed; waveform panel survives a poisoned trace
  lock; test mmap SAFETY comment; tempfile pinned to 3.26.
- D13.2-D13.8: netlist_v0_parser fuzz target (4th target, seeded, in CI
  fuzz smoke); timing_claims tests (10.8 us cycle, datasheet clock
  bounds); instruction_census tests (4004=46 excluding the Invalid
  sentinel discriminant, 4040 +14 = 60); CLAIMS_TO_TESTS enforcement
  cells now name the tests; #[ignore] reason string added. D13.7 and
  D13.8 were already satisfied in source -- the audit rows misread
  passing assertions (recorded so they are not re-raised).
- D14.5-D14.8: shared require-tool/require-path guards on all 15
  tool-invoking Makefile targets; SHELL := /bin/bash for the Vivado
  herestrings; crate .gitignore for sim outputs and profraw; 149 stale
  profraw files removed. Discovery: build/*.v were never git-tracked (a
  global ~/.gitignore "build/" rule hides them), so docs describing them
  as checked-in artifacts overstate provenance -- resolving that is
  D14.7's remaining scope.
- D15.3-D15.8 + D16.5: ruff clean across 60 script files (B023 closures
  bound, S110/S112 swallows logged or narrowed, S324
  usedforsecurity=False, S603 justified per-site); shellcheck clean;
  todo_scan.sh excludes .claude_plans/ and docs/archive/.
- D16.1/D16.9: five phase snapshots archived with ARCHIVE-NOTE headers,
  registry and INDEX synced, transistor total corrected to 3,705.
- D17 DECISION: simd_cluster.rs retired (working cluster.rs already
  serves the role; the file remains in git history); inert feature flag
  deleted.

## Resolved in the port-extraction pass (2026-07-09, follow-up)

- D14.1/D14.2: gate_to_verilog_v0.py derives module ports from
  netlist_v1 signal anchors (name -> layout_node), with structural
  direction inference (gate-driven node -> output), alias collapse for
  signals sharing a pad node, power-rail exclusion, and logged skips for
  anchors absent from the recognized-gate subgraph. All four chips
  regenerated: 4001 = 2 in / 4 out, 4002 = 1 in / 0 out, 4003 = 3 in /
  1 out, 4004 = 0 in / 3 out. Every module compiles under iverilog.
- D14.3 (structural tier): testbenches emit a 1350 ns two-phase
  non-overlapping clock, a reset pulse, and a walking data pattern; all
  four simulate to completion under vvp with VCD dumps. The 4003 (full
  pad coverage) settles to defined outputs; 4001/4004 outputs remain x
  because their partial gate subgraphs contain undriven interior cones
  -- the truthful structural state, not a testbench defect. Functional
  per-chip vectors stay open pending fuller gate extraction (D19.1).

## Resolved in the agent-wave pass (2026-07-09/10)

- D13.1: mcs4-chips/tests/i4004_cpu.rs -- 31 tests driving the CPU
  through the public tick() API (8-phase walk, ALU group, control flow,
  SRC/RAM I/O). Total 1,057 -> 1,088.
- D15.1/D15.2: scripts/pyproject.toml [project.dependencies] with ==
  pins (Pillow/numpy/opencv-python/pytesseract + dev extras); CI pip
  installs pinned to match.
- D18.2/D18.3/D18.4: every workflow action SHA-pinned with tag comments
  (annotated-tag vs branch resolution handled per action); mdbook URL
  pinned to its release tag; SOURCE_DATE_EPOCH exported from the commit
  timestamp in both artifact-producing docs jobs.
- D16.2: Status File Convention stated once (root claude.md); STATUS.md
  links instead of restating.

New findings from the i4004 suite (behavior reported by tests, fixes
deliberately deferred to their own reviewed change):
- D13.10: Fin executes as a stub -- reads pair 0, ignores its operand,
  never loads the target pair. Real hardware fetches ROM[P0] into the
  addressed pair.
- D13.11: FIN/JIN complete in one machine cycle while
  Instruction::cycles() says 2 and hardware takes 2 -- decoder marks
  opr=3 single-cycle; three-way mismatch.
- D13.12: Dcl stores acc & 0xF raw into ram_bank without CM-RAM line
  decode; CM-RAM assertion lives entirely in the system layer.
- D13.13: Jcn treats test_pin==true as condition-satisfied; the 4004
  TEST pin is active-low -- verify polarity against the datasheet.

D19.1 status (in progress, artifacts uncommitted): a first
extract_gates_v1.py pass runs ruff-clean and produces
docs/evidence/gates_v1/ drafts, but fails its own acceptance targets
(INV=0 on 4001/4002/4003; 4003 rails unresolved). Root-cause evidence:
the 4004 rail search resolved node 415, which is the CLK1 signal
anchor -- rail identification by incidence picks clock spines over
supply rails. The completion pass must identify rails from netlist_v1
VDD/VSS signal anchors first and fall back to incidence only with a
clock-anchor exclusion.

## Resolved in the classification + constraints pass (2026-07-09/10)

- D19.1: extract_gates_v1.py lands with rail identification from signal
  anchors (incidence fallback excludes named non-rail anchors), a
  data-verified load definition, zero double-claimed transistors, and a
  confirmed vs role-unconfirmed coverage split. Its headline result is
  negative and evidence-backed: netlist_v1 rails on 4001/4002/4003 are
  fragmented (anchor incidence <= 2, metal_area 0 on every
  channel-touching node), so gate recognition there is impossible from
  this data -- INV=0 is the true value, not an extractor defect. Only
  the 4004 resolves rails (and contains exactly one rail-to-rail
  inverter, hand-verified). Two findings follow:
  - gates_v0 coverage is structurally spurious: v0 emits a gate per
    ordered series pair, double-claiming transistors by the hundreds
    (independently confirmed). The 7,525-line gate-level Verilog derives
    from those counts and inherits the over-statement.
  - D19.5 (new, upstream unblock): generate schematic_wirenets_v0 /
    schematic_connectivity_v0 for 4001/4002/4003 (they exist only for
    4004) so rail nets consolidate; the v1 extractor then works
    unmodified.
- D14.4: per-chip .pcf/.xdc generated from the real gate-top headers
  (orphan mcs4_ice40.pcf/mcs4_spartan7.xdc deleted); Makefile device
  targets corrected (hx8k/ct256, xc7s25csga324). All four gate tops
  synthesize AND place-and-route cleanly through yosys + nextpnr-ice40
  -- the first successful hardware flow in this environment. One
  invalid CT256 ball (E1) was caught by running the real chipdb.
  Spartan-7 .xdc pins remain Vivado-unverified (stated in
  constraints/README.md); pins are placeholders pending board bring-up.

## Resolved in the wirenet pass (2026-07-10): D19.5 executed, hypothesis falsified

schematic_wirenets_v0 and schematic_connectivity_v0 now exist for all
four chips (generated from the real i400x schematic bitmaps with the
4004's parameters; 0 unmapped signal points). The unblock hypothesis is
falsified: build_netlist_v1_v0.py consumes wirenets only to annotate
signals (schematic_component / connectivity hits) and never merges
layout nodes, so rail incidence is untouched -- gates_v1 output is
byte-identical before and after (proven by revert-and-rerun). The
discriminating diagnostic: 4004 rail anchors touch 53-64 channels with
large metal_area; 4001/4002/4003 rail anchors touch <= 2 channels with
metal_area 0.

Deeper finding: schematic_layout_anchors_v0.json carries NO rail
anchors for any chip since commit 783b895 (remap to transistor-incident
nodes) -- the committed netlist_v1 files bake in rail anchors from an
older anchors file and are not reproducible from current inputs
(regenerating even the working 4004 drops its rails). A regeneration
was performed, diagnosed, and deliberately reverted to avoid shipping
that regression.

Corrected unblock chain (replaces D19.5's premise):
- D19.6: restore VDD/VSS entries in schematic_layout_anchors_v0.json
  mapped to high-incidence rail nodes (the 4004 pattern), restoring
  netlist_v1 reproducibility.
- D19.7: add a connectivity-driven node-merge step in
  build_netlist_v1_v0.py so electrically-common layout nodes
  consolidate before rail identification.
- D19.8: netlist_v0 extraction records metal_area for channel-touching
  nodes on 4001/4002/4003 (all currently 0), bounding what recognition
  can confirm even after D19.6/D19.7.

## Resolved: D19.6 rail anchors restored, netlist_v1 reproducible (2026-07-10)

Framing correction from primary sources: the v0 anchors file never held
rail entries in any revision; rails lived only in
schematic_layout_anchors_v1.json via an ensure_power_reset pass, and
committed netlist_v1 provenance names the v1 file. Rail entries are now
restored into v0 in pre-remap shape (the remap script maps all 8 rails
exactly onto the committed node IDs -- no exemption needed), and all
four netlist_v1 files regenerate byte-identically on rebuild with the
provenance-recorded anchors (4004 proof: VCC=415/VSS=3, zero deltas).
Rust hardcoded rail IDs preserved. 4001 pad ports renamed
D0/D2/D3 (aliases kept); verilog_v0, .pcf/.xdc, and PnR re-verified.
gates_v1 output is byte-identical as expected -- recognition on
4001/4002/4003 still waits on D19.7 (node merge) and D19.8
(metal_area).

## Falsified: D19.7 build-stage node-merge cannot raise rail incidence (2026-07-10)

D19.7 premised a connectivity-driven node-merge in
build_netlist_v1_v0.py that consolidates electrically-common layout
nodes before rail identification, unblocking gate recognition on
4001/4002/4003. Primary-source investigation falsifies that premise on
two independent legs; no honest merge exists to add, so the build stage
is left unchanged and netlist_v1 stays byte-identical.

Leg 1 -- the artifact D19.7 names is unusable for merging. The
schematic_connectivity_v0 flood-fill reports junctions_est=0 on every
target for all three chips, so it treats each wire crossing as a
junction and bleeds across the whole schematic: each spine seed
(CLK1/CLK2/SYNC/CM/RESET) hits 19-27 of the ~13 named nets, i.e.
all-to-all. Its seed set is name_regex
"^(CLK1|CLK2|SYNC|CM|RESET|D[0-3]|D[0-3]_PAD)$" -- VDD/VSS are not
seeded at all, so it carries no rail net by construction. The only
localized targets (D0-D3, 8 hits each) are data lines, not rails, so no
slice of this artifact serves the objective. It operates in
schematic-name space, not layout-node space, and cannot merge layout
fragments.

Leg 2 -- the node-merge already happens upstream, and its governing
input is absent for these chips. extract_netlist_v0.py builds nodes by
connected-components over the metal/poly/diffusion masks plus a
union-find that stitches metal<->poly through the vias mask and
metal<->diffusion through the contacts mask. netlist_v1 node IDs are
therefore finished electrical nets; a downstream merge in
build_netlist_v1 has no new committed evidence to draw on. The rail nets
reach the transistor source/drain channels only through
metal<->diffusion contacts, and the contacts artifact exists solely for
4004 (docs/emulators/i4004-contacts.bmp; specs() sets contacts_bmp=None
for 4001/4002/4003, and no i4001/i4002/i4003-contacts bitmap exists on
disk). Controlled comparison from the netlist_v0 counts: 4004 has 586
contact stitches and resolves rails (VCC=415 metal_area 232361 inc 57,
VSS=3 metal_area 411436 inc 50, INV=1); 4001/4002/4003 have 0 contact
stitches and unresolved rails (metal_area 0, terminal incidence 1-2,
INV=0). 4002 VSS node 73 makes the mechanism visible: its rail metal
exists (metal_area 8027) and the channels exist, but with no contact
stitch nothing joins them, so terminal incidence is 0.

Rejected alternative: merging nodes on metal_bbox overlap. Connected-
components already proved distinct metal node IDs are separate metal
components; a bounding-box overlap merge would fabricate connectivity
between unconnected parallel wires, violating the "do not invent
connectivity" constraint. No such merge was added.

gates_v1 before == after (no netlist change): 4001 rails=[2570,5657]
resolved=False INV=0; 4002 rails=[926,3251] resolved=True INV=0; 4003
rails=[152,359] resolved=False INV=0; 4004 rails=[3,415] resolved=True
INV=1. Rust hardcoded rail IDs are untouched (no netlist_v1 delta).
Reproducibility: netlist_v1 rebuilds byte-identically only with
--anchors docs/evidence/schematic_layout_anchors_v1.json (the committed
provenance file); the build script default (anchors_v0) does not
reproduce the committed files.

True blocker: the metal<->diffusion contact layer for 4001/4002/4003 is
missing primary evidence, which is exactly the D19.8 symptom
(metal_area 0 on channel-touching nodes). Rail incidence stays < 8 on
these chips until a contacts bitmap (or equivalent metal-diffusion
stitch evidence) is extracted and fed to extract_netlist_v0 -- a
netlist_v0 regeneration that would reassign node IDs and must be
coordinated with the Rust hardcoded rail IDs. D19.7 is closed as
falsified; the recognition unblock moves entirely to D19.8.

Remaining open phases: D13.9-D13.13, D14.9, D15.6, D16.3/D16.4,
D16.6-D16.8, D18.1, D18.5, D19.2-D19.4, D19.8.
