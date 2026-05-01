# mcs4-system -- Requirements

> System assembly for the MCS-4 (4004 + 4001 + 4002 + 4003) and MCS-40 (4040 +
> 4308 + 4101 + support) configurations. Test fixtures, hex loader,
> single-CPU and multi-CPU clusters, and the SIMD cluster (16-lane parallel
> 4004 execution behind a feature gate). 45 unit + 9 integration tests.

## Build

```sh
cargo build -p mcs4-system
cargo test  -p mcs4-system
cargo clippy -p mcs4-system --all-targets -- -D warnings
```

For the SIMD cluster (nightly required):

```sh
cargo +nightly-2026-04-05 build -p mcs4-system --features simd_cluster
cargo +nightly-2026-04-05 test  -p mcs4-system --features simd_cluster
```

## Toolchain and lints

- Workspace nightly pin: required when `simd_cluster` is enabled (uses
  `std::simd` / `portable_simd`).
- MSRV stable 1.92.0 when `simd_cluster` is OFF.
- Workspace `-D warnings`; the crate root currently uses
  `#![allow(missing_docs)]`. Per debt phase D1.4.3, the blanket
  `#![allow(dead_code, unused_variables)]` in `src/simd_cluster.rs` is being
  narrowed.

## Features

- `simd_cluster` (off by default, currently INERT) -- intended 16-lane SIMD
  execution path against `std::simd` / `portable_simd`. As of 2026-04-30 the
  underlying `src/simd_cluster.rs` file has bitrotted: it is not declared as
  a module in `src/lib.rs` and would not compile under the feature today
  (missing `#![feature(portable_simd)]` crate attribute among other drift).
  Resurrection is tracked under debt phases D1.4.3 and D2.2.

## Rust dependencies (workspace-pinned)

- `mcs4-core`, `mcs4-bus`, `mcs4-chips` (path).
- `rayon` -- multi-CPU cluster work-stealing.
- `memmap2` -- ROM mmap loading (three independent unsafe sites in
  `lib.rs:78`, `mcs4.rs:160`, `mcs40.rs:205`; consolidation tracked under
  D1.2.1 + D10.3.1 SAFETY annotation work).
- `bumpalo` -- transient arena allocations.
- `tracing`, `serde` -- diagnostics and snapshots.
- `tempfile` (dev) -- fixture round-trip tests.
- `proptest` (dev) -- planned cluster differential proptests (D4.1.2).

## System packages

None.

## Known gotchas

- The two-phase fetch path for 2-byte instructions (JUN/JMS/JCN/ISZ/FIM) is
  per-lane in the SIMD cluster; do not interleave fetch with execute across
  lanes.
- ROM hex loader (`fixture.rs`) does not currently bounds-check against ROM
  size at the parse step (tracked under D10.3.3).
- `cargo run -p mcs4-system -- --mode fixture` invokes the canonical fixture
  runner; integration tests use the same path.
