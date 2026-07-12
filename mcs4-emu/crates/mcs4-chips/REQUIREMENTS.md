# mcs4-chips -- Requirements

> Behavioral and cycle-accurate implementations of every MCS-4 / MCS-40 chip
> tracked by this project. Includes the 4004 (46 instructions, full ALU,
> 3-level stack), 4040 (60 instructions, interrupts, 7-level stack), all
> support chips (4001/4002/4003/4008/4009/3216/3226/3205/3404/2101/4101/
> 4201/4207/4209/4211/4265/4289/4308/4316/4702), the disassembler, and the
> SIMD cluster decode helpers. 252 unit + 4 circuit-sim + 1 fuzz + 11 proptest
> tests.

## Build

```sh
cargo build -p mcs4-chips
cargo test  -p mcs4-chips
cargo clippy -p mcs4-chips --all-targets -- -D warnings
```

For the SIMD path (nightly `portable_simd`):
```sh
cargo +nightly-2026-04-05 build -p mcs4-chips --features simd
```

## Toolchain and lints

- Rust nightly pin: `nightly-2026-04-05` (required for `simd` feature).
- MSRV stable: 1.92.0 (when `simd` feature is OFF).
- Workspace `-D warnings`; the crate uses `clippy::unwrap_used = "warn"` per
  workspace lint policy.

## Features

- `simd` (off by default) -- enables the `std::simd` parallel decode helpers
  used by `mcs4-system`'s SIMD cluster.
- `mimalloc` (off by default) -- swap to mimalloc allocator for benchmark runs.

## Rust dependencies (workspace-pinned)

- `mcs4-core`, `mcs4-bus` (path) -- chip lifecycle traits and bus primitives.
- `bitflags`, `smallvec`, `num-traits`, `bytemuck`, `zerocopy` -- bit-packed
  state and zero-copy helpers.
- `tracing`, `serde`, `serde_json` -- diagnostics and netlist JSON.
- `proptest` (dev) -- property-based testing.
- `criterion` (dev) -- microbenchmarks (`benches/cpu_bench.rs`).
- `mimalloc` (optional) -- allocator swap.

## System packages

None for the default build. Benchmark runs with `mimalloc` need a working
C toolchain (typically already present).

## Known gotchas

- `ChipSolverBridge` impls for the four core chips load subcircuit JSON from
  `docs/evidence/subcircuits_v0/<chip>/`; tests must run from the workspace
  root or via Cargo so `CARGO_MANIFEST_DIR.ancestors().nth(3)` resolves.
- The 4004 has no `DEC` instruction (opcode 0x7 is `ISZ`, a 2-byte instruction).
- `ScalarI4004::execute_single` returns `bool` to signal whether the PC was set
  by the instruction (`BBL`, `JIN`, etc.).
- 4040-specific opcodes (0x01-0x0E) are dispatched in `execute_4004()`
  through 4040 handler methods; do not duplicate them in 4004 paths.
