# Deployment and Operations

## Build

```sh
# Debug build (faster compilation, debug info)
cargo build --workspace --locked

# Release build (optimized, stripped)
cargo build --workspace --release --locked

# Zen3-optimized build (AMD Zen 3 CPUs)
RUSTFLAGS="-C target-cpu=znver3" cargo build --workspace --profile zen3-release --locked

# Clean all build artifacts
cargo clean
```

## Run

### GUI emulator
```sh
cargo run --locked -p mcs4-gui -- --mode gui
```

Read a validated shared trace frame stream without mixing it with live worker
commands:

```sh
cargo run --locked -p mcs4-gui -- --mode gui \
  --trace-frames target/trace-capture/mcs4.frames.jsonl
```

Run and Step remain disabled for an imported trace. The importer bounds file,
line, and frame retention before it atomically replaces the visible trace.
Reset clears the import and returns control to the behavioral worker.

### Fixture runner (no GUI)
```sh
cargo run --locked -p mcs4-gui -- --mode fixture --system mcs4 \
  --fixture src_wrm_rdm --cycles 12 --strict-io-phases
```

With ROM port input for RDR fixtures:
```sh
cargo run --locked -p mcs4-gui -- --mode fixture --system mcs4 \
  --fixture rom_port_wrr_rdr --rom-io-input 12 --rom-io-chip 0 \
  --strict-io-phases
```

Direct command-boundary fixture runner:
```sh
cargo run --locked -p mcs4-system --bin fixture_runner -- \
  mcs4-emu/crates/mcs4-system/fixtures/src_wrm_rdm.hex 12
```

Versioned behavioral frame capture and replay checkpoint:
```sh
mkdir -p target/trace-capture
cargo run --locked -p mcs4-system --bin mcs4-phase-trace -- \
  --architecture mcs4 \
  --fixture mcs4-emu/crates/mcs4-system/fixtures/src_wrm_rdm.hex \
  --warmup 32 --phases 24 --format frame-jsonl \
  --checkpoint target/trace-capture/mcs4.checkpoint.json \
  > target/trace-capture/mcs4.frames.jsonl
```

`frame-jsonl` records completed behavioral phases with provenance and a digest
of the complete ordered external-input transcript. The checkpoint replays the
recorded behavioral transcript; it does not snapshot analog or hardware state.
The default output remains the legacy
pretty `phase-json` array for existing fixture consumers.

## Testing

```sh
# Full test suite
cargo test --workspace --locked

# Single crate
cargo test --locked -p mcs4-chips

# With SIMD feature (requires nightly)
cargo test --locked -p mcs4-chips --features simd

# Coverage report (requires cargo-llvm-cov)
cargo llvm-cov --workspace --all-features --lcov --output-path coverage/lcov.info
```

### Gate HDL delivery check

```sh
# Check the currently resolved retained gate-HDL surface.
python3 scripts/gate_to_verilog_v0.py --chips 4003 \
  --check-export-contract --check-generated

# Run its exhaustive X/Z-resolution testbench.
make -C mcs4-emu/crates/mcs4-fpga sim CHIP=4003 MODE=gate
```

Gate-mode operations for 4001, 4002, and 4004 stop at preflight because the
retained extraction does not expose a deterministic output cone. The gate
contract establishes structural resolution only; it does not establish chip
behavior, timing, pin mapping, or board readiness. See
`docs/evidence/gate-hdl-export-contract.md`.

### Optional Qt6 and Verilator virtual FPGA board

The optional toolset builds a generated `i4003_fpga` Qt6 board plus a headless
full MCS-4 Verilator system adapter. The system adapter composes generated
4004, 4001, and 4002 HDL with the shared simulation wrapper, monitor ROM, VCD,
JSON summary, and mapped JSONL trace frames. It requires `cmake`, `ninja`, Qt6
Core and Widgets development packages, and Verilator. It does not establish
synthesis, timing closure, pin assignment, or physical-board behavior.

```sh
just virtual-fpga-build
just virtual-fpga-test
just virtual-fpga-run
just virtual-fpga-release-check
```

The test suite verifies the 4003 CP shift and active-low parallel-output gate
contract plus the MCS-4 monitor scenario: WMP activity, a non-idle bus,
non-overlapping phi phases, no observed bus contention, and cumulative-budget
rejection. Both headless paths stream bounded shared-schema JSONL frames. The
common MCS-4 stimulus path runs one exact 256-byte ROM, reset, TEST, and
phase-boundary transcript through behavioral replay and the full-system adapter,
then records per-signal matches and mismatches. It proves input identity and
comparison coverage, not cross-backend or board equivalence. See
`tools/virtual-fpga/README.md` for the scenario format.

### Host-validated Gowin system HDL

The shared MCS-4 system uses an explicit host clock for lint and simulation:

```sh
just hdl-validate
make -C mcs4-emu/crates/mcs4-fpga gowin_lint
make -C mcs4-emu/crates/mcs4-fpga gowin_sim
```

`gowin_prog` and `gowin_flash` require a reviewed JSON
`GOWIN_CLOCK_EVIDENCE=<path>` record. The preflight validates the exact
`sys_clk_in` pin, I/O type, source hashes, constraint hash, and generated SDC.
The programming guard then validates the source contract, active timing report,
timing paths, and generated bitstream hashes before it invokes the programmer.
The repository has no such reviewed record or constraint assignment now, so
programming is intentionally blocked. See
`docs/evidence/fpga-board-clock-and-conformance-blockers.md`.

## Linting and Formatting

```sh
# Format check
cargo fmt --all -- --check

# Clippy (warnings as errors)
cargo clippy --workspace --locked --all-targets --all-features -- -D warnings

# Full lint pass (alias)
cargo clippy-all

# Check tracked local Markdown and YAML targets, Markdown anchors, and
# reference-style links without network access
just link-check

# Verify typed HDL, netlist provenance, timing provenance, capabilities, and
# time-bounded security exceptions in addition to the standard Rust and Python gates.
just verify
```

## Benchmarks

```sh
# Standard benchmarks
cargo bench --workspace --locked

# With AVX2 optimization
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo bench --workspace --locked

# Performance regression check (Python)
python3 scripts/benchmark_emulator_v0.py \
  --output docs/evidence/benchmarks_ci.json \
  --baseline docs/evidence/benchmarks_baseline_v0.json \
  --fail-on-regression
```

## Logs

Set `RUST_LOG` for tracing output:
```sh
RUST_LOG=info cargo run --locked -p mcs4-gui
RUST_LOG=mcs4_core=debug cargo test --locked -p mcs4-core
```

## Artifacts

| Path | Description |
|------|-------------|
| `target/` | Build output (workspace root) |
| `coverage/` | Coverage reports (lcov) |
| `docs/evidence/` | OCR, benchmarks, extraction artifacts |
| `mcs4-emu/crates/mcs4-system/fixtures/` | ROM test fixtures (.hex) |

## Call graph and runtime evidence

```sh
just capture-callgraphs
```

The capture records cflow and cscope lexical maps, compiler MIR direct-call
edges, syscall envelopes, a bounded callgrind profile, and the frame/replay
checkpoint command boundary. See
`docs/repository-debt-callgraph-capture-roadmap.md` for interpretation limits
and retention rules.

## Developer proof bundle

Run `just developer-bundle` from a clean working tree to retain a source archive,
typed i4003 FPGA HDL, VCD and JSON scenario evidence, checksums, provenance,
and validation logs under `target/developer-bundle/`. This artifact is a scoped
developer evidence bundle, not a release package or a hardware-delivery claim.
See `docs/DEVELOPER_BUNDLE.md`.

## CI

Both GitHub workflows accept manual dispatch only during the quota hold. Run
the `CI` or `Docs CI` workflow from the Actions page before a merge when
the relevant hosted gate is required. The current jobs include:
- **build-test**: format check, clippy, full test suite
- **coverage**: generates lcov report
- **bench**: runs benchmarks with regression detection
