# Call Graph and Runtime Capture Evidence

## Capture identity

This report records the complete capture executed on 2026-07-12 from the
repository working tree. The generated bundle is intentionally untracked under
target/callgraph-capture-20260712-common-stimulus because it contains MIR, strace logs,
profiler data, VCD output, and generated runtime evidence. Regenerate it with:

~~~sh
scripts/callgraph_capture.sh target/callgraph-capture-YYYYMMDD
~~~

The script rejects a nonempty destination. It records commit, branch,
source date, tool paths and versions, the Cargo target directory, and then
working-tree status in environment.txt before analysis starts. v17 records
commit `eccb68a9cd4f1fe2e628006835ed1cc0c7504f72`, 143 Git status entries,
1,241 checksummed artifacts, and 67 status records. It inventories 169 Rust,
124 Python, 19 Verilog, and 3 C++ or header files from both the index and
nonignored worktree. It archives 336 exact source inputs with a per-file
SHA-256 manifest and the tracked working-tree binary diff. Every recorded
status exits zero. The working tree is intentionally dirty, so v17 proves the
captured source state and does not serve as a clean-revision release artifact.

The preceding v5 attempt remains incomplete after its launch snapshot reports
a shell parser error. v6 remains a complete prior bundle. v7 is intentionally
incomplete because a formatter repair landed after its environment snapshot.
v8 records the active-low E repair. v9 reruns after the generic 4003
power-on-clear repair. v10 adds the typed exporter CLI, TimingIo trace, and
transactional netlist-publication paths. v11 makes source inventory complete
for the dirty worktree. v12 and v13 remain retained diagnostics for the
isolated-target and environment-parser failures. v14 repairs both capture
mechanisms but lacks source-byte archival and an exact required probe profile.
v15 adds both mechanisms. v16 reruns the complete profile after the final
virtual-board and capture-contract hardening. v17 adds the common-stimulus
MIR, Python, C++ lexical, and runtime paths and is the authoritative capture
for this implementation.

## Result matrix

All 67 recorded capture statuses begin with `exit=0`. The two Python cflow
probes record `usable=0`; the Python and C++ cscope probes record `semantic=0`.
Those fields prevent C-oriented lexical tooling from becoming a false Python,
C++, or Rust semantic-call-graph claim.

| Evidence surface | Output | Result |
|---|---|---|
| cflow lexical map | nine Rust and one C++ cflow text maps and stderr files | 10 of 10 exit zero; Rust and C++ maps are lexical only |
| Python cflow probes | gate exporter and netlist publisher cflow text plus parser diagnostics | both exit zero, explicitly unusable for Python semantics |
| cscope lexical index | Rust database plus 716 selected symbol-query lines | built successfully; lexical only |
| C++ cscope probe | both virtual-board sources plus 376 selected-query lines | exit zero, explicitly non-semantic |
| Python cscope probe | gate exporter token index and selected queries | exit zero, explicitly non-semantic |
| Python AST maps | gate exporter and netlist publisher direct-call maps | 26 functions and 29 edges; 7 functions and 7 edges |
| compiler MIR | fixture runner, phase trace, GUI fixture, typed exporter CLI, system, chips, core, FPGA, and focused I4003 plus replay TSV maps | 8 full maps and 3 derived focus maps exit zero |
| process envelopes | MCS-4 fixture, MCS-40 fixture, fixture runner, legacy and frame trace capture, typed exporter, netlist publisher, focused tests, and full-system Verilator | 21 of 21 exit zero |
| dynamic profile | 64-cycle MCS-4 callgrind run and annotation | exit zero |
| dynamic graph render | gprof2dot DOT plus Graphviz SVG | exit zero |

The capture is reproducible evidence, not a claim that every static or
dynamic edge represents hardware behavior.

The post-fix workspace coverage run reports 91.07% regions, 87.71% functions,
and 89.44% lines. Fixture-runner line coverage remains 75.86% after success,
missing-file, and malformed-input black-box tests because both error cases
share the same runner error boundary. The usage branch remains explicit test
work.

## Static map measurements

| Source | Direct-call rows from MIR | Interpretation |
|---|---:|---|
| fixture runner binary | 50 | command parsing, fixture load, machine run, and result printing |
| phase trace binary | 396 | fixture selection, replay, bounded phase execution, checkpointing, and JSON serialization |
| GUI fixture binary | 452 | command mode selection, MCS-4 and MCS-40 fixture branches |
| typed exporter CLI | 297 | typed request parsing, export, atomic output, and manifest writing |
| system library | 2,503 | system wiring, phase tracing, replay, shared-frame comparison, stepping, and support surfaces |
| chips library | 1,320 | chip, timing, and instruction implementation surface |
| core library | 4,153 | solver, device, TCAD, and model surface |
| FPGA library | 3,415 | module construction, typed request selection, debug exports, and Verilog export surface |
| trace-replay focus | 427 | replay input, phase stepping, checkpoint, restore, frame validation, and comparison paths |

The Rust and C++ cflow maps and all cscope maps return textual references only.
They do not resolve Rust traits, macros, generic monomorphization, C++ Qt
dispatch, or HDL execution. MIR and bounded runtime capture carry the
compiler-aware and executed-path evidence.

## Elucidated control paths

### Fixture-runner command boundary

The compiler map resolves this direct progression:

~~~text
fixture_runner main
  -> Mcs4System::minimal
  -> Mcs4System::load_rom_hex_file
  -> Mcs4System::run_cycles
  -> Mcs4System::pc
  -> Mcs4System::accumulator
  -> Mcs4System::carry
~~~

The focused runtime workload exits zero and reports 12 cycles, PC 0x000c,
accumulator 0xa, and carry false. The missing-file integration test verifies
the alternate boundary: it returns failure with a path-bearing error instead
of panicking.

### GUI fixture branch

The GUI MIR map resolves both system branches in run_fixture:

~~~text
run_fixture
  -> Mcs4System::minimal
  -> Mcs4System::load_rom_hex_file
  -> Mcs4System::phase, pc, step, accumulator
  -> validate_io_phase

run_fixture
  -> Mcs40System::new
  -> Mcs40System::load_rom_hex_file
  -> Mcs40System::phase, pc, step
  -> validate_io_phase
~~~

The MCS-4 runtime trace shows the first cycle in order:

~~~text
A1 -> A2 -> A3 -> M1 -> M2 -> X1 -> X2 -> X3
~~~

The trace records PC, bus value, cycle count, and I/O operation after every
phase. The MCS-40 runtime trace shows the same eight-phase progression.
This proves the bounded fixtures invoke the phase loop. It does not prove
instruction-set completeness or electrical timing accuracy.

### Solver timing branch

The focused solver test executes its transient timing assertion and reports:

~~~text
stage delay = 22.57 ns
t0D2 window = 150 ns
margin = 127.43 ns
~~~

The result validates that one retained inverter fixture satisfies its asserted
window under the current model. It does not calibrate the complete process
model or prove all extracted paths meet the historical datasheet.

### FPGA export branch

The typed FPGA exporter path now runs as a command boundary. Its MIR map has
297 direct-call rows, and the captured CLI request selects `i4003` with the
`fpga` flavor, writes `i4003_fpga.v`, and writes a schema-1 provenance manifest.
The v17 manifest records the source revision, dirty-tree provenance, the typed
request, and the generated-output SHA-256.

This establishes the captured typed selection and atomic-output boundary. The
separate `just hdl-validate` gate executes Icarus and an exact Verilator warning
contract for all 37 typed exports. Neither capture nor static validation proves
target synthesis, complete gate input, constraint validity, electrical timing,
or board behavior.

The gate export contract now traces each declared output cone before delivery.
The 4003 Q4 cone resolves through declared inputs and its generated bench
checks all eight binary input vectors for X or Z outputs. The 4001, 4002, and
4004 exports fail preflight because their retained evidence respectively has
undriven dependencies, no output anchor, and multiple output drivers. Their
generated benches call `$fatal` instead of printing X values and returning
success. This proves structural output resolution only; independently sourced
behavior vectors remain necessary before a functional claim.

See `docs/evidence/gate-hdl-export-contract.md` for the contract, current
per-chip result, and delivery commands.

### 4003 active-low E and power-on-clear boundary

The v17 capture retains a bounded map for the corrected 4003 path. Its cflow
slices name set_enable_pin and chip_i4003 as lexical roots and retain the
Rust parser diagnostics. Its cscope report locates set_enable_pin,
parallel_outputs_enabled, parallel_out, serial_out, chip_i4003, and
chip_i4003_fpga in the 716-line selected query report. These two tools remain
lexical evidence only.

The compiler-aware slices contain 51 I4003 rows and 117 emitter rows. They
resolve the intended behavior boundary:

~~~text
shift_in -> set_data_in -> set_clock(false) -> set_clock(true)
set_enable -> set_enable_pin
is_enabled -> parallel_outputs_enabled
parallel_out -> parallel_outputs_enabled
output_bit -> parallel_out
all_chip_modules -> chip_i4003
fpga_chip_modules -> chip_i4003_fpga
~~~

The emitted-module MIR rows retain enable_n, the parallel-output mask to
10'd0, an ungated serial_out assignment, an unconditional generic shift, and
the generic power-on-clear declaration reg [9:0] shift_reg = 10'd0. The
FPGA-safe module retains the same output mask, clears shift_reg through rst,
and shifts only on clk_in_rise, not on E.

The runtime capture records three passing focused commands: the behavioral
test that shifts ten ones while E is high and then verifies masked parallel
output plus live serial output; the MCS-4 system-wiring test that still
observes 0x2AA through the standalone E-low convention; and the two FPGA
emitter tests that assert the generic power-on-clear declaration and the
FPGA reset assignment. These executions establish the coded behavior
boundary. The capture invokes the FPGA-safe exporter CLI. The separate HDL
gate simulates and lints the generated behavioral and FPGA modules. Neither
surface establishes stage ordering, physical delay, gate equivalence, target
synthesis, or board behavior.

### Timing trace, replay, and full-system Verilator boundary

The phase-trace CLI has a 396-row MIR direct-call map and a 427-row derived
replay focus map. Its cscope slice locates `apply_input`, `step_phase`,
`checkpoint`, `restore_from_checkpoint`, frame validation, and
`compare_trace_frames`. The frame-capture workload runs after a 32-phase
warmup, emits 24 deterministic MCS-4 JSONL frames from A1 through X3 over
machine cycles 4 through 7, and writes a transcript-backed checkpoint.

The full-system C++ adapter has a lexical C++ cflow entry from `main` to
`runHeadless` and a 376-line cscope slice for scenario parsing, `SystemBoard`,
phase advancement, streamed frame emission, expectation checks, and atomic
summary publication. The bounded system scenario runs 5,000 system cycles,
emits 628 mapped frames, records 16 WMP strobes, observes a non-idle bus,
observes no phi overlap or bus contention, and reports one active bus producer
at completion. The adapter rejects an over-budget cumulative scenario. Its VCD,
JSON summary, JSONL frames, and strace logs are retained in v17.

The v17 capture adds the common-stimulus path: a 136-row behavioral MIR map,
a 59-line Python AST map, a 449-line C++ cscope report, and strace envelopes
for the behavioral and FPGA consumers. The common CTest runs one exact 256-byte
ROM, reset, TEST, and phase-boundary JSON document through behavioral replay
and the system adapter. Its comparison report records 87 matching and 57
mismatching observations across 16 phase frames. It does not assert equivalence:
phase, ROM/RAM-control representation, and nine bus samples remain explicit
reconciliation targets.

These observations establish that the bounded workloads execute the trace,
replay, and host HDL observation paths. They do not calibrate transistor
propagation, clock slew, package delay, a physical clock source, or a board.

### Transactional netlist-publication boundary

The v17 capture retains a Python AST map for `build_netlist_v1_v0.py`: seven
defined functions and seven same-file direct-call edges. Its syscall envelope
builds the 4003 v1 output in the capture directory and produces a schema-1
manifest whose output SHA-256 is
`6d60922a52342158056188cbf97e5f3e2618a8d7a4fb3915c9031468555d3c30`.

The AST map does not resolve the imported transaction helper. The runtime
capture shows that the publication path completes; focused unit tests inject
failure and verify journal-based rollback and recovery. This evidence does not
infer missing physical connectivity from a partial netlist.

## Dynamic profile result

The 64-cycle MCS-4 fixture records 2,324,089 instructions under callgrind.
Startup dominates this deliberately short run:

| Function or category | Inclusive instruction share |
|---|---:|
| dynamic-loader relocation | 12.79% |
| mimalloc string comparison helper | 11.98% |
| Mcs4System step_traced | 2.66% |
| TimingIo record_completed_phase | 2.58% |
| I4004 tick | 1.00% |
| I4002 tick_bus | 0.46% |
| I4001 tick_bus | 0.44% |

The profile establishes that the fixture reaches the expected system and chip
functions. It does not support throughput conclusions because loader and
initialization work dominate the short workload. A separate benchmark needs a
fixed ROM, warmup, iteration policy, host record, and variance report.

## Tool boundary findings

The default capture uses cflow, cscope, cargo-modules, compiler MIR, strace,
callgrind, gprof2dot, Graphviz, CMake, Ninja, and Verilator. It records each
required probe status and tool availability state. The verifier rejects a
bundle that omits any required status, source-byte archive member, manifest
hash, or capture-profile version.

The capture defaults to the already validated workspace Cargo target directory
instead of a fresh capture-local target. This avoids an unbounded dependency
rebuild during MIR collection. The environment record names that target path;
the bundle retains the compiler-produced MIR, not the mutable target cache.
Its source archive binds the input bytes independently of that cache.

rust-analyzer analysis-stats is available but remains opt-in through
CALLGRAPH_CAPTURE_RUST_ANALYZER=1. A full workspace trial reports 4,867 MB in
the tool summary and terminates before the capture status file is written.
This makes it valuable for a deliberate semantic investigation but unsuitable
as an unconditional capture prerequisite on this host.

Additional host-only tools selected from the local tool catalog are:

| Tool | Use when the question is concrete |
|---|---|
| rust-analyzer | Resolve definitions, references, diagnostics, and semantic regions around a bounded Rust path |
| semgrep | Enforce a reviewed rule for production panic boundaries, broad exceptions, or incomplete export shapes |
| CodeQL | Trace attacker-controlled fixture or netlist data across crate boundaries after a database build is justified |
| pylint and lizard | Audit Python exception contracts and complexity after the existing Ruff surface |
| uftrace and perf | Investigate one reproducible Linux runtime path after a workload contract exists |
| rr | Replay a deterministic native divergence or crash |
| radamsa | Mutate inputs only after an oracle defines the expected rejection behavior |
| diffoscope | Compare supposedly reproducible host or cross-compiled artifacts |
| sigrok-cli | Capture an actual FPGA or bus waveform during attended hardware validation |

Python complexity triage uses lizard with cyclomatic-complexity threshold 15
and length threshold 120. It reports 53 warnings. The highest-impact
candidates include ocr_signal_labels main at CCN 110, detect_layout_edge_labels
main at CCN 69, extract_netlist_v0 main at CCN 63, and build_netlist_v1_v0
main at CCN 63. Rust triage at CCN 20 and length 150 reports 11 warnings,
including Mcs4System step at CCN 28, Mcs40System step at CCN 23, transient
solver run at CCN 30, DC solver solve at CCN 22, and the large FPGA module
renderers. These measurements identify decomposition candidates; they do not
establish defects without behavior-preserving tests.

## Falsification conditions

- A cflow or cscope reference does not establish a Rust semantic edge.
- A MIR edge does not establish that a fixture executes it.
- A callgrind edge does not establish functional correctness.
- A fixture trace does not establish analog timing or silicon conformance.
- A full-system Verilator frame map does not establish a common behavioral
  transcript, target synthesis, or a board result.
- A generated SVG does not establish HDL synthesis or board behavior.

The next investigation selects one unresolved task from
docs/repository-debt-callgraph-capture-roadmap.md and adds its own input,
oracle, capture, and falsification rule.
