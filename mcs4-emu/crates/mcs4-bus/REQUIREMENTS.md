# mcs4-bus -- Requirements

> Bus protocol abstraction for the MCS-4 / MCS-40 family. 4-bit data bus,
> non-overlapping two-phase clock, control signals (SYNC, CM-ROM, CM-RAM, IoOp).
> 17 unit tests, no integration tests; pure data structures, no I/O.

## Build

```sh
cargo build -p mcs4-bus
cargo test -p mcs4-bus
cargo clippy -p mcs4-bus --all-targets -- -D warnings
```

## Toolchain and lints

- Rust toolchain: `nightly-2026-04-05` (workspace pin in `rust-toolchain.toml`).
- MSRV: 1.92.0 (stable). The crate itself does not require nightly features.
- Edition: 2021 (workspace).
- Workspace lints apply (`-D warnings` via `.cargo/config.toml`); see
  `[workspace.lints]` in the root `Cargo.toml`.

## Features

None. The crate has no Cargo features.

## Rust dependencies (workspace-pinned)

- `mcs4-core` (path) -- shared simulation primitives.
- `tracing` -- structured logging spans on bus operations.
- `serde`, `rkyv`, `bytemuck`, `zerocopy` -- serialization for bus snapshots.

## System packages

None. Pure Rust.

## Known gotchas

- The bus models `phi1`/`phi2` as non-overlapping; consumers must respect the
  guard interval (see `clock.rs`).
- `data_bus.rs` returns `Z` (high-impedance) when no driver is present; do not
  treat `Z` as logic 0.
