# Build Environment and Delivery Boundaries

The repository has four build surfaces. Each surface has its own lock or
provenance boundary. No document treats them as one hermetic environment.

| Surface | Root and package mechanism | Reproducibility control | Validation entry point |
|---|---|---|---|
| Rust emulator and exporter | Root `Cargo.toml` workspace and `Cargo.lock` | `rust-toolchain.toml` pins nightly; Cargo uses `--locked` | `just verify` |
| Rust fuzz targets | `mcs4-emu/fuzz` cargo-fuzz workspace | Fuzz toolchain and corpus are separate from the root workspace | `cargo fuzz` from `mcs4-emu` |
| Python evidence pipeline | `scripts/pyproject.toml` | Direct runtime packages use exact versions; host OCR binaries remain external | `python3 -m pytest scripts/tests` and `ruff check scripts/` |
| HDL and virtual board | Generated Verilog, hand-maintained Gowin system HDL, Icarus, Verilator, CMake, Qt6, and Ninja | Typed exporter manifests, host HDL simulation, VCD and JSONL scenarios, and tool versions in capture/bundle evidence | `just hdl-validate`, `make -C mcs4-emu/crates/mcs4-fpga gowin_sim`, and `just virtual-fpga-release-check` |

## Supported local validation

Run the normal repository gate from the root:

~~~sh
just verify
~~~

It runs Rust format, lint, and tests; Python tests and lint; local-link checks;
typed HDL validation; gate-HDL structural validation; netlist hashes and input
provenance; timing provenance; capability evidence; RustSec exception checks;
and Rust documentation generation.

The optional virtual-board gate needs CMake, Ninja, Qt6 Core and Widgets
development files, and Verilator:

~~~sh
just virtual-fpga-release-check
~~~

The shared Gowin HDL system has deterministic host gates that do not require
vendor tools:

~~~sh
make -C mcs4-emu/crates/mcs4-fpga gowin_lint
make -C mcs4-emu/crates/mcs4-fpga gowin_sim
~~~

Use the system package manager to install host tools. The project does not
claim that an arbitrary package-manager snapshot is reproducible. The retained
capture and developer bundle record actual tool versions and commands for a
specific result.

## Determinism and output locations

CI sets `SOURCE_DATE_EPOCH` to the source commit time for Rust documentation
and mdBook output. The developer bundle does the same for a clean revision.
Generated work belongs under `target/` or `build/`, both ignored by the
repository. Retained evidence belongs under `docs/evidence/` only when its
source, generator, validation rule, and retention purpose are explicit.

## Delivery boundary

`just developer-bundle` creates a clean-revision developer proof bundle for
the virtual i4003 board. It is evidence for the scoped host workflow only. It
does not publish a release, install software, generate a target bitstream, or
program hardware. A release or hardware delivery requires a separately scoped
target, reviewed `sys_clk_in` route, synthesis report, constraints, checksum,
rollback path, and attended probe. The current Gowin programming targets reject
missing reviewed JSON clock evidence, mismatched `sys_clk_in` constraints,
source hashes, generated SDC, active timing report, timing paths, or bitstream
hashes. The exact entry criteria live in
`docs/evidence/fpga-board-clock-and-conformance-blockers.md`.

## Known environment debt

- The Python project pins direct packages but has no lockfile or isolated
  environment bootstrap. A future reproducible Python delivery needs a locked
  resolver output and a supported installer.
- Host packages for OCR, HDL, Qt, and FPGA vendor tools remain distribution or
  vendor supplied. Capture manifests record observed versions but do not pin a
  complete operating-system image.
- FPGA synthesis and board programming stay target-specific and blocked until
  a board clock route, electrical constraint, timing report, and attended board
  probe enter scope.
