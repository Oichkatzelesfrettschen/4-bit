# mcs4-gui -- Requirements

> egui / eframe based debugger and programmer GUI. Panels for disassembly,
> registers, memory, stack, breakpoints, run/stop/step controls, waveform
> view, signal trace, provenance, and die viewer. Provides the `mcs4-emu`
> binary. Panel logic and worker/session boundaries run in headless tests.

## Build

```sh
cargo build -p mcs4-gui
cargo run   -p mcs4-gui --bin mcs4-emu -- --help
cargo test  -p mcs4-gui
cargo clippy -p mcs4-gui --all-targets -- -D warnings
```

## Toolchain and lints

- Workspace nightly pin; MSRV stable 1.92.0.
- Workspace `-D warnings`; `#![allow(missing_docs)]` is applied at the crate
  root and is being narrowed under debt phase D5.2.1.

## Features

None today.

## Rust dependencies (workspace-pinned)

- `mcs4-core`, `mcs4-bus`, `mcs4-chips`, `mcs4-system` (path).
- `egui`, `eframe` (0.33) -- immediate-mode UI.
- `clap` -- CLI entrypoint.
- `serde_json` -- validated shared trace-frame JSONL import.
- `tracing`, `tracing-subscriber` -- logging.

## System packages (Linux)

eframe needs the standard X11/Wayland and graphics deps. On Debian/Ubuntu:

```sh
sudo apt-get install -y \
    libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libxkbcommon-dev libssl-dev libfontconfig1-dev \
    libgl1-mesa-dev
```

CachyOS / Arch:

```sh
sudo pacman -S libxcb libxkbcommon openssl fontconfig mesa
```

Without these, `cargo build -p mcs4-gui` will fail at link time on Linux.

## Known gotchas

- `session.rs` owns the only mutable behavioral simulation state in a worker.
  UI callbacks send bounded commands and consume immutable `TraceFrame` events.
  Panels must not obtain or mutate `Mcs4System` directly.
- `--trace-frames <path>` streams and validates shared JSONL frames at GUI
  startup. The importer caps the file at 64 MiB, each line at 64 KiB, and the
  frame count at 100,000. It rejects run-ID regression and non-increasing
  sequences within a run before atomically replacing the visible trace.
  Imported mode disables Run and Step; Reset clears the imported trace and
  returns control to the behavioral worker.
- `signal_trace.rs` retains frame IDs, run boundaries, and explicit eviction
  counts. A missing retained prefix is a discontinuity, not an implicit
  continuous waveform.
- `panels/provenance.rs` reports model, backend, fidelity, evidence state, and
  declared hashes. `panels/die_viewer.rs` stays evidence-empty until a
  coordinate-bearing physical netlist supplies device observations.
- `panels/waveform.rs` is the production waveform viewer (cursors,
  measurement markers, signal grouping, and trace-frame selection). The earlier audit
  reference to a "legacy `src/waveform.rs` scaffolding" was incorrect; no
  such file exists in the tree. Pointer selection starts after the fixed label
  column, so labels cannot select a waveform frame.
- `signal_trace.rs` and `panels/waveform.rs` contain UI-bounded `unwrap()`
  sites inside `#[cfg(test)] mod tests` blocks; production code paths in
  these modules are unwrap-free.
- Headless test mode requires no display server.
