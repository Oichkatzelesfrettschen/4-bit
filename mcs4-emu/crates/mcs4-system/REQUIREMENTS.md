# mcs4-system -- Requirements

> System assembly for the MCS-4 (4004 + 4001 + 4002 + 4003) and MCS-40 (4040 +
> 4308 + 4101 + support) configurations. Test fixtures, hex loader,
> single-CPU and multi-CPU clusters. The crate also defines versioned phase
> frames and transcript-backed behavioral replay checkpoints.

## Build

```sh
cargo build --locked -p mcs4-system
cargo test --locked -p mcs4-system
cargo clippy --locked -p mcs4-system --all-targets -- -D warnings
```

Capture versioned frames and a replay checkpoint:

```sh
mkdir -p target/trace-capture
cargo run --locked -p mcs4-system --bin mcs4-phase-trace -- \
  --architecture mcs4 \
  --fixture mcs4-emu/crates/mcs4-system/fixtures/src_wrm_rdm.hex \
  --warmup 32 --phases 24 --format frame-jsonl \
  --checkpoint target/trace-capture/mcs4.checkpoint.json \
  > target/trace-capture/mcs4.frames.jsonl
```

## Toolchain and lints

- Workspace nightly pin is used by the full workspace verification surface.
- MSRV stable 1.92.0 applies to this crate.
- Workspace `-D warnings`; the crate root currently uses
  `#![allow(missing_docs)]`. Per debt phase D1.4.3, the blanket
  `#![allow(dead_code, unused_variables)]` in `src/simd_cluster.rs` is being
  narrowed.

## Rust dependencies (workspace-pinned)

- `mcs4-core`, `mcs4-bus`, `mcs4-chips` (path).
- `rayon` -- multi-CPU cluster work-stealing.
- `memmap2` -- ROM mmap loading (three independent unsafe sites in
  `lib.rs:78`, `mcs4.rs:160`, `mcs40.rs:205`; consolidation tracked under
  D1.2.1 + D10.3.1 SAFETY annotation work).
- `bumpalo` -- transient arena allocations.
- `tracing`, `serde`, `serde_json`, `sha2` -- diagnostics, trace frames, and
  replay stimulus hashes.
- `tempfile` (dev) -- fixture round-trip tests.
- `proptest` (dev) -- planned cluster differential proptests (D4.1.2).

## System packages

None.

## Known gotchas

- ROM hex loader (`fixture.rs`) does not currently bounds-check against ROM
  size at the parse step (tracked under D10.3.3).
- `cargo run -p mcs4-system -- --mode fixture` invokes the canonical fixture
  runner; command-boundary tests cover valid, missing, and malformed fixtures.
- The default `mcs4-phase-trace` format remains a legacy `PhaseTrace` JSON
  array. `--format frame-jsonl` records `TraceFrame` objects and permits
  `--checkpoint`; the checkpoint reconstructs behavioral state from its
  transcript and does not snapshot analog solver state. The transcript digest
  covers every ordered external input, not only the ROM image.
- Checkpoint publication writes and syncs a temporary file, creates the final
  path without replacement, and syncs the parent directory. Existing or
  concurrently created checkpoints remain intact.
- A shared JSON schema does not prove cross-fidelity equivalence. Frame
  comparison requires equal stimulus representation and digest plus mapped
  signal paths; the i4003-only Verilator adapter intentionally fails that
  comparison today.
