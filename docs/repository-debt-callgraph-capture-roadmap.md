# Repository Debt, Call Graph, and Runtime Capture Roadmap

## Scope and evidence contract

This document records the live repository state observed on 2026-07-11. It
does not convert a passing unit test, a generated HDL file, or a historical
completion percentage into a claim of hardware fidelity. Each claim remains
bounded by its evidence surface and falsification condition.

The repository contains three coupled implementation languages:

- Rust provides the emulator, system integration, solvers, GUI, and HDL model
  generator.
- Python provides extraction, OCR, evidence transformation, and artifact
  validation.
- Verilog provides generated and hand-maintained FPGA-facing artifacts.

The root Cargo workspace is the primary build surface. mcs4-emu/fuzz is a
separate cargo-fuzz workspace. Python and Verilog tooling require independent
environment and provenance control. No single complete label spans those
three surfaces.

## Reproducible call-graph capture

Run the complete capture from a clean or intentionally recorded working tree:

~~~sh
just capture-callgraphs
# Or retain a named capture:
scripts/callgraph_capture.sh target/callgraph-capture-20260711
~~~

The capture script refuses to write into a nonempty output directory. It
records the commit, branch, working-tree status, tool locations and versions,
source inventories, status codes, stderr, and generated evidence.

The graph uses complementary evidence layers. No layer alone supplies a valid
Rust call graph.

| Layer | Tool and artifact | What it establishes | What it does not establish |
|---|---|---|---|
| Source inventory | Git index plus nonignored worktree files and cflow input lists | Exact files considered by each narrow probe, including dirty implementation files | Rust name or type resolution |
| Lexical map | cflow, cscope/rust.out, and selected-paths.txt | Textual definitions and call-like references | Trait dispatch, macros, monomorphization, or runtime execution |
| Compiler map | MIR calls TSV from rustc -Zunpretty=mir | Compiler-resolved direct calls in selected binaries and libraries | Indirect calls, all generic instances, or external behavior |
| Structural map | cargo-modules artifacts when available | Crate and module dependencies | Dynamic call edges |
| Process capture | strace files | Executed process, file, loader, and syscall boundary | In-process Rust control flow |
| Dynamic call graph | callgrind files | Native functions reached by the fixture workload and their cost | Untaken branches, hardware fidelity, or representative full-session cost |
| Domain trace | RUST_LOG trace output | Completed bus phases with PC, bus, and I/O state | A substitute for an instruction oracle |

cflow and cscope are retained because they are useful fast lexical indexes.
Rust syntax, macros, traits, and generics make their output non-semantic. MIR
and runtime capture carry the semantic and execution burden.

## Load-bearing execution paths

The capture covers every path that presently carries a correctness, evidence,
or delivery claim. The named functions identify entry and destination
boundaries; MIR files provide exact direct-call edges for the selected build.

| Path | Entry | Core progression | Required evidence |
|---|---|---|---|
| MCS-4 fixture | fixture_runner main | fixture parse -> load_rom_hex_file -> run_cycles -> Mcs4System step -> I4004 tick -> selected I4001 and I4002 bus ticks | CLI integration test, MIR, strace, callgrind, trace lines |
| GUI MCS-4 fixture | mcs4-gui run_fixture | argument validation -> Mcs4System construction -> fixture load -> phase stepping -> final state | narrow cflow/cscope, GUI MIR, strace, trace lines |
| GUI MCS-40 fixture | mcs4-gui run_fixture | argument validation -> Mcs40System construction -> Mcs40System step -> I4040 tick and MCS-40 memory devices | GUI MIR, MCS-40 strace, integration test |
| Solver timing | solver_datasheet_timing test | circuit graph -> transient solver -> device equations -> matrix and convergence path -> timing assertion | test result, core MIR, strace envelope, coverage |
| Netlist ingestion | layout_netlist load_netlist_v1 | parsed netlist -> bridge -> circuit graph -> DC or transient solver | parser tests, bridge tests, provenance manifest, future end-to-end capture |
| Typed HDL export | mcs4-fpga-export main | CLI parse -> ExportRequest -> module selection -> atomic Verilog and manifest output | CLI MIR, strace, manifest hash, 37-module HDL gate, board result |
| Netlist publication | build_netlist_v1_v0 main | source inputs -> netlist payload -> transaction journal -> output and manifest publication | Python AST map, strace, transaction tests, output/input hashes |
| Timing trace and replay | mcs4-phase-trace main | fixture parse -> complete ordered ReplaySession input transcript -> phase step -> TraceFrame validation -> JSONL frame and no-replace checkpoint publication | trace CLI MIR, cscope lexical map, strace, checkpoint race tests, source-located timing ledger |
| GUI trace session | mcs4-gui app update | UI command -> one-owner simulation worker -> immutable TraceFrame event -> bounded JSONL import -> waveform and provenance panels | GUI worker tests, bounded-import tests, waveform logic tests, GUI MIR, source map |
| Virtual i4003 adapter | virtual FPGA headless scenario | scenario action -> Verilator i4003 model -> VCD, JSON summary, shared-schema JSONL frame | CMake scenario test, adapter fixture, non-comparison contract |
| Virtual MCS-4 system adapter | mcs4-virtual-system and mcs4-common-stimulus | bounded backend scenario or common ROM/reset/TEST/phase stimulus -> shared HDL simulation top or behavioral MCS-4 -> bounded VCD and streamed mapped JSONL frame -> per-signal comparison report | CMake monitor, budget-rejection, and common-stimulus tests; Rust contract tests; comparison report preserves matches and mismatches without asserting equivalence |
| Shared FPGA HDL system | mcs4_system_sim_top | explicit clock -> phase generator -> shared core -> generated 4004/4001/4002 and UART bridge | clean Verilator lint, Icarus system test, HDL export verifier |
| Extraction to gate HDL | gate_to_verilog_v0.py | retained gate input -> port derivation -> output-cone contract -> primitive lowering -> generated HDL | Python call map, input cardinality, contract report, HDL tests |
| FPGA delivery | crate Makefile and constraints | HDL -> synthesis -> place and route -> bitstream -> board probe | tool versions, utilization/timing, programmed-board capture, rollback artifact |

## Instrumented runtime capture

The new system trace event records each completed MCS-4 and MCS-40 bus phase:
phase, cycle count, program counter, resolved bus state, and I/O operation.
It adds observation without changing machine state or using a debug-only
execution path.

The capture runs these bounded workloads:

1. MCS-4 GUI fixture: src_wrm_rdm, 12 cycles, strict I/O phases.
2. MCS-40 GUI fixture: src_wrm_rdm, 12 cycles, strict I/O phases.
3. CLI fixture runner: the same fixture, 12 cycles.
4. MCS-4 legacy phase trace: 32-phase warmup and 24 retained records.
5. MCS-4 JSONL trace replay capture: the same warmup and records plus one
   transcript-backed checkpoint.
6. Typed i4003 FPGA exporter CLI: one output and provenance manifest.
7. Transactional 4003 netlist publication into the capture directory.
8. Solver datasheet timing integration test.
9. MCS-40 and 4308 integration test.
10. FPGA, trace-replay, and cross-fidelity focused tests.
11. Full-system Verilator monitor scenario with staged ROM, mapped JSONL frame,
    and VCD output.
12. MCS-4 fixture under callgrind for 64 cycles.

The short workloads answer control-flow and boundary questions. They do not
measure emulator throughput. Performance work requires a separate fixed ROM,
cycle count, host profile, warmup policy, and baseline artifact.

## Verified baseline

The following checks complete on the audited tree after the fixture-runner
boundary tests and the source-bound 4003 E correction:

| Check | Result | Interpretation |
|---|---|---|
| cargo test --workspace --locked | 1,159 passed, 0 failed, 0 ignored | Regression baseline; not a completeness proof |
| cargo fmt --all -- --check | passes | Rust formatting gate remains clean |
| cargo clippy --workspace --all-targets --all-features -- -D warnings | passes | Warning-free Rust analysis on the strict workspace surface |
| ruff check --no-cache scripts/ | passes | Python static style surface remains clean |
| shellcheck -S warning scripts/*.sh | passes | Shell warning surface remains clean |
| cargo llvm-cov --workspace --all-features --summary-only | 91.07% regions, 87.71% functions, 89.44% lines | Broad execution coverage; uncovered modules require explicit disposition |
| cargo audit --json | reports four registered advisories: two quick-xml vulnerabilities and two unmaintained transitive crates | `scripts/verify_advisory_exceptions.py` rejects an untracked, stale, mismatched, or expired exception; remediation remains open |

The coverage result exposes unexecuted or weakly executed surfaces:

- mcs4-chips/src/i4040/solver_bridge.rs has focused reference-graph, solver, and error-path tests. It does not represent an extracted 4040 netlist.
- mcs4-chips/src/simd.rs has feature-enabled scalar differential tests for its fetch, ADD, and program-counter subset. It has no performance benchmark contract.
- mcs4-gui/src/app.rs and mcs4-gui/src/main.rs have no measured execution.
- mcs4-system/src/bin/fixture_runner.rs has black-box success, missing-file,
  and malformed-input coverage; usage paths remain uncovered.
- mcs4-core/src/fidelity_manager.rs and several GUI panels remain materially
  below the workspace average.

## Debt taxonomy and evidence-backed findings

| Debt class | Current evidence | Required resolution |
|---|---|---|
| Implementation | The fixture runner returns path-bearing bounded-load errors. The 4003 Rust and Verilog models implement active-low E output masking and power-on clear. TimingIo records validated logical phase windows. Typed exporter requests render supported behavioral and FPGA modules with provenance manifests. | Add a target-specific 4003 initialization or reset synthesis proof and source-qualified analog timing comparison. |
| Structural | `docs/meta/capabilities.json` now records capability state, role owner, evidence, limitation, blocker, and next gate. `docs/BUILD_ENVIRONMENT.md` separates Rust, Python, HDL, fuzz, and delivery boundaries. | Keep each record synchronized with source and retained evidence. |
| Test | Solver bridge and feature-enabled SIMD paths have focused tests. GUI entry and host-specific unreadable-fixture behavior remain limited. Test totals remain owned by `mcs4-emu/CLAUDE.md`. | Add scenario tests by behavior, not line count, and retain a workload contract before performance claims. |
| Methodology | Existing callgraph material combines source traces and generated maps but lacks a declared semantic boundary. | Retain lexical maps, add compiler MIR and dynamic captures, and state which layer supports every conclusion. |
| Scientific and materials | Process parameters, geometry calibration, and extraction claims require traceable primary-source anchors and uncertainty bounds. | Record source locator, digitization method, nominal value, interval, model use, and falsifier for each parameter. |
| Evidence and provenance | Generated artifacts can outlive the input corpus or lose their transformation command. | Bind input hashes, generator version, command, output hash, and validation result in retained manifests. |
| Documentation | Deployment instructions used a nonexistent fixture name and claimed automatic CI triggers that the workflows do not provide. Completion language conflicts with retained placeholders and hardware blockers. | Correct commands, state manual dispatch accurately, and use capability states rather than aggregate completion percentages. |
| Dependency and security | `quick-xml` 0.38.4 has two registered vulnerability advisories through the GUI stack. `paste` and `ttf-parser` are separately registered unmaintained dependencies. The lockfile updates `anyhow` and both `rand` lines to remove superseded advisories. | Upgrade or reduce the GUI dependency graph and remove every exception before 2026-10-01. |
| Build environment | Rust is pinned but host packages, Python dependencies, HDL tools, and target hardware tools are not one hermetic environment. | Publish platform-specific prerequisites, lock Python dependencies, and capture exact tool versions in generated evidence. |
| Package and delivery | `just developer-bundle` creates a clean-revision virtual-board proof bundle with source archive, checksums, manifests, VCD, JSON scenario output, and validation logs. It is not a release artifact. | Define supported distributable deliverables, install, checksum, rollback, and hardware evidence before claiming deployment. |
| FPGA and hardware | The shared host HDL system passes deterministic lint and simulation through an explicit input clock. The programming contract now validates reviewed JSON route evidence, exact source and CST hashes, generated SDC, active timing output, and bitstream hashes. The gate export contract still accepts only the retained 4003 Q4 cone for structural resolution. No reviewed `sys_clk_in` route, target timing report, bitstream provenance, or attended board capture exists. | Record and constrain the board clock route, synthesize the exact source, retain timing and bitstream hashes, then run an attended waveform-backed probe. |
| Observability | The behavioral system emits stable versioned TraceFrame records with run identity, canonical external-input transcript hash and kind, JSONL capture, and no-replace replay checkpoints. The GUI consumes immutable frames from a single owner and bounds imported system-adapter frames. The full-system adapter exposes mapped paths and latches contention. | Run one common ROM and explicit input transcript through behavioral and full-system adapters before asserting cross-backend equivalence. |
| Performance | Callgrind startup cost dominates a short fixture capture. Existing benchmark claims do not define one shared workload contract. | Separate profiling capture from performance benchmarks and retain workload, host, flags, sample count, and variance. |
| Organizational | Canonical ownership differs between root documentation and mcs4-emu/CLAUDE.md; historical status text mixes completed and deferred work. | Declare owners for status, requirements, release, evidence, and generated artifacts; automate cross-document consistency checks. |
| Artifact lifecycle | Generated output, retained captures, caches, and historical documents coexist without a common retention class. | Classify each as source, reproducible derivative, evidence snapshot, or disposable cache; store regeneration and cleanup rules. |
| Interoperability | The 4003 behavioral boundary now has a source-bound CP, E, and serial-output contract, but behavioral Rust models, extracted gates, generated Verilog, and physical constraints still lack a complete equivalence ladder. | Add translation-boundary tests and compare stable vectors at every representation change. |

## Reconciliation decisions

| Conflict | Resolution |
|---|---|
| cflow and cscope look like call graphs but do not resolve Rust semantics | Preserve them as lexical evidence only. MIR and dynamic evidence support semantic and executed-edge claims. |
| docs/DEPLOYMENT.md said CI runs on push and pull requests | The workflows use workflow_dispatch only. Documentation now states manual dispatch under the quota hold. |
| The Pages deployment job required push while the workflow only accepted manual dispatch | The deployment condition must permit manual dispatch on main, or the deployment job remains unreachable. |
| Documentation reported 1,053 tests while the canonical ledger reported 1,124 | The verified live result is 1,159 after typed HDL, timing, fixture-boundary, replay, behavior-contract, bounded-import, atomic-checkpoint, and common-stimulus tests. mcs4-emu/CLAUDE.md remains the count owner. |
| Completion percentages describe broad capability while placeholders and unvalidated hardware remain | Replace broad percentages in future status reporting with per-capability evidence states: implemented, tested, reproduced, synthesized, hardware-probed, or blocked. |
| Historical material says gate-level Verilog is populated while scoped assessments describe empty or insufficient source gates | Treat HDL validity as an input-to-output chain. Require gate cardinality, port derivation, synthesis, simulation, and physical evidence before promoting the claim. |
| A syntactically valid gate module can still emit X values or have no observable output | The export contract checks each declared output cone, emits a failing bench for an incomplete graph, and blocks gate-mode simulation or synthesis before delivery. |
| The 4003 behavioral models treated E as a shift inhibit | The MCS-40 Users Manual states that E only gates parallel outputs and that power-on clear clears the shift register. Rust and generated behavioral Verilog now shift on CP regardless of E, mask parallel outputs when E is high, leave serial output exposed, and represent the zero power-on state. |
| SIMD is described as active in status material while source and coverage require a retention decision | The feature-enabled differential oracle now establishes its documented subset. Performance and full-ISA claims remain blocked on a workload benchmark and larger oracle. |
| Time-boxed advisory ignores suppress immediate policy failure | They do not resolve the advisories. Dependency update work remains a security item with a dated exit condition. |

## Ordered work program

The work items below are small, independently verifiable units. Their order
expresses dependencies, not a promise that every item fits one change set.

### Capture and evidence integrity

| Work item | Input | Completion evidence | Depends on |
|---|---|---|---|
| Record capture environment | clean or recorded tree | immutable environment record with commit and tools | none |
| Run lexical capture | selected Rust entry files | cflow status, output, stderr, and cscope database | environment record |
| Generate compiler call edges | selected binaries and libraries | MIR plus TSV direct-call edges | pinned nightly |
| Generate structural maps | Cargo workspace | cargo-modules artifacts or explicit unavailability record | Cargo metadata |
| Run syscall envelopes | six bounded workloads | exit status, stdout, stderr, and strace files | built binaries |
| Run dynamic profile | 64-cycle MCS-4 fixture | callgrind file and annotated function report | built GUI binary |
| Preserve trace sample | trace-enabled bounded fixture | phase, PC, bus, and I/O sequence attached to capture | trace instrumentation |
| Add capture manifest verifier (done 2026-07-11) | capture directory | checksummed manifest rejects missing required probe status, source-byte archive, path artifact, hash, or required capture surface | stable capture layout |
| Archive selected capture | validated capture | provenance record and retention class | capture manifest |

### Command and system correctness

| Work item | Input | Completion evidence | Depends on |
|---|---|---|---|
| Keep fixture-load error boundary | missing fixture path | nonzero result, path-bearing error, no panic | current CLI tests |
| Add malformed-hex CLI case (done 2026-07-11) | malformed fixture | exit code 1, parse location, no stdout, and no panic | fixture runner |
| Correct 4003 active-low E and power-on-clear behavior (done 2026-07-11) | MCS-40 Users Manual OCR CP, E, serial-output, and power-on-clear clauses | Rust and generated behavioral Verilog tests show E high masks parallel outputs without blocking CP shifting or serial output, and retain zero-state POC representation | primary-source pinout note |
| Add unreadable-fixture CLI case | permission-controlled fixture | deterministic error behavior on supported host | platform test guard |
| Assert MCS-4 trace sequence | fixed ROM and cycle count | expected phase/PC/bus trace fixture | trace capture |
| Assert MCS-40 trace sequence | fixed ROM and cycle count | expected phase/PC/bus trace fixture | MCS-40 integration |
| Define TimingIo role (done 2026-07-11) | caller inventory and timing model | CPU-owned validated phase snapshot, deterministic trace fixture, and source-located parameter ledger | current source trace |
| Characterize system-step branch partitions | Mcs4System and Mcs40System lizard warnings | phase-handler boundary proposal with unchanged trace output | fixed trace fixtures |
| Extract one tested system-step partition | selected phase handler | unit and trace equivalence before and after extraction | branch partition proposal |
| Exercise I4040 solver bridge (done 2026-07-11) | named reference graph | success, unknown-name, convergence, fidelity, and no-physical-pin-map tests | solver contract |
| Decide SIMD lifecycle (partially done 2026-07-11) | current consumers and feature build | feature-enabled scalar differential test for fetch, ADD, and PC subset; benchmark remains open | dependency inventory |

### Solver, extraction, and scientific fidelity

| Work item | Input | Completion evidence | Depends on |
|---|---|---|---|
| Build logical timing parameter provenance table (done 2026-07-11) | retained 4004 OCR timing rows | source locator, picosecond interval, code use site, limitation, and falsifier in a checked ledger | primary sources |
| Build process and geometry parameter provenance table | process and geometry constants | source locator, units, interval, use site, and confidence | primary sources |
| Add parameter uncertainty propagation | selected timing or power result | output interval with sensitivity ranking | provenance table |
| Reproduce one datasheet timing point | source timing row and circuit fixture | measured model output, tolerance, and failure explanation | stable transient solver |
| Bound 4003 CP-to-serial-output timing | MCS-40 Users Manual timing row | explicit timing-model contract or measured delay model with load assumptions | source-bound 4003 behavior |
| Record solver convergence corpus | representative graphs | iteration count, gmin path, convergence result, and seed | solver tests |
| Partition transient solver run | lizard complexity report and waveform tests | named setup, step, convergence, and record helpers with unchanged waveforms | waveform oracle |
| Make netlist conversion transactional (done 2026-07-11) | malformed and valid netlists | crash-recovery journal, no partial artifact on injected failure, input hashes, and output manifest hashes | parser and output contract |
| Cross-check netlist bridge | one retained netlist | graph counts, named anchors, and solver result tied to input hash | netlist provenance |
| Measure gate-input cardinality | gates_v0 source per chip | explicit nonzero or zero result before HDL generation | retained extraction source |
| Reject empty semantic gate export | empty gate input | failure or explicit non-semantic artifact classification | gate exporter contract |
| Verify gate port derivation | retained pad and subcircuit source | port list matches manifest and constraint names | gate input completeness |
| Replace generic exporter shell (done 2026-07-11) | public export callers | typed behavioral and FPGA request matrix, provenance manifests, Icarus, and exact Verilator warning contract | API inventory |
| Compare behavioral and generated HDL vectors | fixed instruction and I/O vectors | simulator equivalence report and seed | exporter semantics |

### Test, quality, and security gates

| Work item | Input | Completion evidence | Depends on |
|---|---|---|---|
| Re-run coverage after CLI tests | workspace test suite | changed coverage summary retained with command | current baseline |
| Add solver bridge coverage target (done 2026-07-11) | named reference graph | exercised success, unknown, convergence, and fidelity behavior | solver fixture |
| Add GUI entry harness | headless or attended test strategy | documented supported test mode and captured output | GUI platform boundary |
| Add Python exception-path tests | each broad exception site | behavior-specific tests or narrowed exception type | exception inventory |
| Establish Python formatter baseline | Ruff formatter reports 92 unformatted scripts while the verifier runs only Ruff checks | approved formatting policy, bounded baseline change, and a gate decision | documentation-owner decision |
| Enforce tracked local Markdown links (done 2026-07-11) | tracked Markdown and YAML paths, Markdown anchors, and reference-style links | offline verifier runs in `just verify` and Docs CI before Pages deployment | link_check.py, link_check.sh entrypoint, and isolated fixture tests |
| Characterize one high-complexity Python workflow | lizard report and script fixtures | branch table, input-output contract, and current failure behavior | retained script input |
| Decompose one high-complexity Python workflow | behavior-specific tests | smaller named functions with byte-for-byte or schema-equivalent output | workflow characterization |
| Add Verilog syntax and lint gate (done 2026-07-11) | 37 typed generated modules | Icarus syntax result and exact Verilator warning contract per module | reproducible export |
| Extend gate HDL behavior oracles | contract-passing gate module and independent vector source | named functional vectors compare behavioral RTL and gate HDL after structural X/Z checks | gate export contract |
| Add HDL synthesis smoke gate | one selected target | tool version, timing, utilization, and generated netlist | syntax-valid HDL |
| Audit quick-xml dependency path (done 2026-07-11) | lockfile and features | compatible-update trial, dependency-path record, exposure boundary, and exception registry | cargo metadata |
| Remove advisory ignore or set a bounded revisit artifact (done 2026-07-11) | live audit report | registry-checked exceptions expire on 2026-10-01 and fail when stale or untracked | dependency update |
| Separate benchmark contract from profiler capture | chosen workload | host metadata, repetitions, variance, and baseline comparison | fixed workload |

### Delivery, documentation, and governance

| Work item | Input | Completion evidence | Depends on |
|---|---|---|---|
| Publish capability matrix (done 2026-07-11) | crate, script, and FPGA inventory | checked machine-readable records with role owner, evidence state, limitation, blocker, and next gate | debt taxonomy |
| Correct operational command examples | executable CLI and fixture inventory | docs commands run with --locked | command tests |
| Keep workflow documentation truthful | workflow YAML | docs state manual dispatch until trigger policy changes | workflow review |
| Make Pages deployment reachable | docs workflow on main | manual dispatch shows deploy job eligibility | workflow policy |
| Define developer proof-bundle contract (done 2026-07-11) | clean virtual-board revision | source archive, provenance, checksums, VCD, JSON scenario output, validation logs, and explicit non-release boundary | delivery scope decision |
| Define release artifact contract | intended consumer and platform | version, checksum, provenance, install, and rollback specification | delivery scope decision |
| Add Python environment lock | supported Python tools | reproducible installation and validation command | dependency inventory |
| Define artifact retention classes | target, coverage, generated HDL, evidence bundles | owner, regeneration, retention, and deletion policy | artifact inventory |
| Reconcile canonical document owners | root and emulator documentation | explicit source-of-truth map enforced by checker | owner decision |
| Replace aggregate completion language | status and roadmap prose | capability evidence states with blockers | capability matrix |

### Hardware validation

| Work item | Input | Completion evidence | Depends on |
|---|---|---|---|
| Select one FPGA target | board and toolchain availability | target declaration, pin source, voltage assumptions, rollback method | user hardware scope |
| Compile behavioral HDL for target | chosen module and constraints | tool log, timing, utilization, bitstream hash | synthesis gate |
| Perform attended board probe | isolated board and serial capture | boot/configuration log, observed I/O vector, recovery procedure | board access |
| Record physical mismatch | failing hardware observation | minimized vector, hypothesis, source location, and retest plan | probe artifact |
| Promote hardware capability | repeated board evidence | versioned evidence bundle and explicit supported boundary | no unresolved mismatch |

## Falsification rules

- A lexical cscope or cflow edge does not establish a Rust semantic edge.
- A compiler MIR edge does not establish that a workload executes it.
- A callgrind edge does not establish correctness or physical fidelity.
- A passing unit test does not establish a source parameter, die geometry, or
  electrical model is historically accurate.
- A Verilog parser or synthesis pass does not establish board correctness.
- A nonzero coverage percentage does not establish that an unmeasured
  representation boundary is sound.
- An ignored dependency advisory does not establish the dependency is safe.
- An aggregate completion percentage does not establish the underlying
  capability is present, reproduced, or validated.

## Immediate next gate

The source-bound 4003 behavioral correction does not establish the partial
gate artifact as a chip model. Bind an independently sourced 4003
behavior-vector corpus to the structurally resolved Q4 cone. Record source
locator, input names, expected outputs, generator revision, simulator command,
and output hash. Compare the same vectors against the behavioral model before
extending the gate-delivery surface. Do not promote the current X/Z-resolution
result into a chip, electrical, synthesis, or board claim.
