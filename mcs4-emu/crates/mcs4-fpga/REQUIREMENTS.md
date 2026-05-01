# mcs4-fpga -- Requirements

> Verilog export and FPGA constraint generation. Produces synthesizable
> behavioral Verilog for all 22 chips and gate-level Verilog for the 4 core
> chips (7,525 lines emitted from extracted netlists). Ships iCE40 (.pcf) and
> Spartan-7 (.xdc) constraint files plus a synthesis Makefile. 24 unit tests.

## Build

```sh
cargo build -p mcs4-fpga
cargo test  -p mcs4-fpga
cargo clippy -p mcs4-fpga --all-targets -- -D warnings
```

## Toolchain and lints

- Workspace nightly pin (`nightly-2026-04-05`); MSRV stable 1.92.0.
- Workspace `-D warnings`. Note: `clippy::panic` should be elevated to deny in
  this crate (debt item D1.1.3); currently a `panic!()` in `verilog.rs:2479`
  is tracked for migration to `Result`.

## Features

None.

## Rust dependencies (workspace-pinned)

- `mcs4-chips`, `mcs4-bus`, `mcs4-core` (path) -- chip metadata for module
  emission.
- `serde`, `serde_json` -- gate-level netlist input.

## System packages (consumer side, optional)

For actually running the synthesis step (outside this Rust crate):

- iCE40 path: `yosys`, `nextpnr-ice40`, `icestorm` (icepack/iceprog).
- Spartan-7 path: Xilinx Vivado (proprietary).
- Linting: `verilator` (planned in D10.4.2).

These tools are NOT required to build or test the Rust crate; they are only
needed when invoking `make -C constraints synth-*` against the emitted Verilog.

## Known gotchas

- `verilog.rs` is currently a 3,053-line monolith mixing exporter, per-chip
  generators, and netlist parsing. A decomposition to
  `verilog/{port,module,exporter,chips/{behavioral,fpga}}` is tracked under
  debt phase D2.1 (deferred to a focused human-reviewed PR).
- The single `#[ignore]` test in this crate is intentional (long-running
  Verilog regeneration).
- The `impl/` subtree contains a baseline Gowin Synthesis + PnR run
  (March 2026) for the `mcs4_estimate` and `mcs4_full` projects. Files
  include `.fs` bitstreams (~3.4 MB), `.vg` synthesized netlists, and HTML
  reports. They are committed as a reproducibility checkpoint, not as
  build artifacts to regenerate every CI run. Future regeneration should
  go through Gowin EDA tooling per `docs/evidence/FPGA_SYNTHESIS_WORKFLOW_V0.md`.
