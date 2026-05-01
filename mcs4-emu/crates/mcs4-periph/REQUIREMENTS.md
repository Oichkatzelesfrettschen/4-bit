# mcs4-periph -- Requirements

> Peripheral device emulation: 7-segment display (BCD decode, ASCII render),
> 4x4 matrix keyboard scanner with debounce, UART (TX/RX FIFOs, ASR-33
> compatible bit-bang). 30 unit tests.

## Build

```sh
cargo build -p mcs4-periph
cargo test  -p mcs4-periph
cargo clippy -p mcs4-periph --all-targets -- -D warnings
```

## Toolchain and lints

- Workspace nightly pin; MSRV stable 1.92.0.
- Workspace `-D warnings`.

## Features

None.

## Rust dependencies (workspace-pinned)

- `mcs4-core`, `mcs4-bus`, `mcs4-chips` (path).
- `tracing`, `serde` -- diagnostics and snapshots.

## System packages

None.

## Known gotchas

- The matrix keyboard exposes raw row drives plus debounced events; tests
  drive both APIs.
- UART bit-bang timing assumes a 110 baud ASR-33; configurable at construction.
