# mcs4-fpga -- Requirements

> Verilog export and FPGA constraint generation. Produces behavioral Verilog
> for all 22 chips and retained gate-level Verilog for the
> 4 core chips. The gate export contract currently permits delivery only for
> the structural 4003 Q4 cone; 4001, 4002, and 4004 remain incomplete evidence
> artifacts and fail before gate-mode simulation or synthesis. Ships iCE40
> (.pcf) and Spartan-7 (.xdc) constraint files plus a synthesis Makefile.
> `just hdl-validate` checks all typed behavioral and FPGA exports plus the
> shared host HDL system.

## Build

```sh
cargo build --locked -p mcs4-fpga
cargo test --locked -p mcs4-fpga
cargo clippy --locked -p mcs4-fpga --all-targets -- -D warnings
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
- Gate-HDL simulation: `iverilog` and `vvp`.
- Linting: `verilator`; `just hdl-validate` runs Icarus syntax checks and a
  module-scoped Verilator warning contract across every typed export.

For the optional virtual board under `tools/virtual-fpga/`:

- `cmake`, `ninja`, Qt6 Core and Widgets development packages, and Verilator.
- `just virtual-fpga-test` generates FPGA-safe 4003, 4004, 4001, and 4002
  models; builds the Qt6 4003 host and the headless MCS-4 system adapter; and
  runs VCD, JSON-oracle, and JSONL-trace scenarios.

These tools are NOT required to build or test the Rust crate. They are needed
for `just hdl-validate`, `make -C mcs4-emu/crates/mcs4-fpga gowin_lint`, or
the target-specific `ice40_*`, `xilinx_*`, and `gowin_*` Makefile targets.

## Known gotchas

- `make sim CHIP=4003 MODE=gate` checks the gate export contract and runs the
  Q4 structural-resolution bench. Gate-mode operations for 4001, 4002, and
  4004 stop at the same preflight because their retained gate graphs do not
  provide a deterministic exported surface. See
  `docs/evidence/gate-hdl-export-contract.md`.

- The generated behavioral i4003 and i4003_fpga modules expose enable_n as
  the physical active-low E input. E high masks parallel outputs to zero. It
  does not stop CP shifting or serial output. External behavioral-module
  instantiations must rename the former ambiguous enable port and drive
  enable_n low to expose parallel data.

- The generic behavioral i4003 initializes shift_reg to zero to represent
  power-on clear. The i4003_fpga module clears shift_reg through rst. Target
  synthesis must verify the selected initialization or reset mapping.

- `mcs4_system_core.v` is the shared host HDL system. `mcs4_system_sim_top.v`
  supplies deterministic external clocking for `gowin_lint` and `gowin_sim`.
  The core instantiates one 4001 and one 4002 model and therefore does not
  establish a complete multi-chip board memory map.

- `mcs4_top.v` requires `sys_clk_in`. `gowin_prog` and `gowin_flash` first
  require a reviewed schema-1 JSON `GOWIN_CLOCK_EVIDENCE` record. The contract
  validates the exact `sys_clk_in` `IO_LOC`, `IO_TYPE`, static and generated
  source hashes, constraint hash, generated SDC, active timing report, timing
  paths, and bitstream hash before programming. The repository lacks that
  clock evidence now, so these targets intentionally fail before board
  mutation. See
  `docs/evidence/fpga-board-clock-and-conformance-blockers.md`.

- `verilog.rs` remains a large monolith mixing exporter, per-chip
  generators, and netlist parsing. A decomposition to
  `verilog/{port,module,exporter,chips/{behavioral,fpga}}` is tracked under
  debt phase D2.1 (deferred to a focused human-reviewed PR).
- The `impl/` subtree contains a baseline Gowin Synthesis + PnR run
  (March 2026) for the `mcs4_estimate` and `mcs4_full` projects. Files
  include `.fs` bitstreams (~3.4 MB), `.vg` synthesized netlists, and HTML
  reports. They are committed as a reproducibility checkpoint, not as
  build artifacts to regenerate every CI run. Future regeneration should
  go through Gowin EDA tooling per `docs/evidence/FPGA_SYNTHESIS_WORKFLOW_V0.md`.
