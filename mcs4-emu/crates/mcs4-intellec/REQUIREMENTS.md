# mcs4-intellec -- Requirements

> Intellec-4 development system emulation: front panel (switches, LEDs),
> Monitor ROM (examine/deposit/go/halt), 4702 PROM programmer, and system
> integration glue. 44 unit tests + 6 full-system integration tests.

## Build

```sh
cargo build -p mcs4-intellec
cargo test  -p mcs4-intellec
cargo clippy -p mcs4-intellec --all-targets -- -D warnings
```

## Toolchain and lints

- Workspace nightly pin; MSRV stable 1.92.0.
- Debt note: `monitor.rs:{332,350,393}` and `system.rs:417` contain four
  `panic!()` sites slated for `Result` migration (D1.1.2). After that lands,
  `clippy::panic = "deny"` will be elevated for this crate.

## Features

None.

## Rust dependencies (workspace-pinned)

- `mcs4-core`, `mcs4-bus`, `mcs4-chips`, `mcs4-system`, `mcs4-periph` (path).
- `tracing`, `serde` -- diagnostics and snapshots.

## System packages

None.

## Known gotchas

- Monitor command dispatch panics on unexpected enum variants; the upcoming
  property-test harness `tests/proptest_monitor.rs` (D4.1.3) will cover this
  path before deprecation of the panics.
- PROM programmer interface assumes a live 4702 (`mcs4-chips/src/i4702.rs`);
  tests stub the 4702 with the chip's own state machine.
