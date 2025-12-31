# Roadmap

- Tier 1: 4040 CPU, Disassembler, Waveform Viewer. [In Progress]
- Nightly toolchain enabled; clippy warnings-as-errors gate.
- Dependencies: modular-bitfield, bitflags, tinyvec/smallvec; portable-simd for cluster simulation.
- Dual UI System [PLANNED]
  - Features: cli, tui(ratatui+crossterm), gui(pixels+winit); build-time selectable, run-time `--mode` flag.
  - CLI: batch, fuzzing, scripting; TUI: era-accurate terminal UI; GUI: framebuffer graphics and waveform.
  - Shared debugger controller and panels; consistent shortcuts.
- Next 5 Steps (2025-12-31T06:46:38Z)
  1) Add Disassembler section and rkyv snapshot export/import to API.md (zero-copy, CLI/TUI integration) [Done]
  2) Document I/O ports mapping and SRC/DCL timing in Architecture [Done]
  3) Add ROM loading CLI (clap) plan in Deployment; examples for segmented 4001 chips [Planned]
  4) Wire docs CI: nightly cargo doc, link/md lint; status badge in README [Planned]
  5) TUI module plan: registers/code/system panes, controls, snapshot schema; bind to ringbuf [Planned]
