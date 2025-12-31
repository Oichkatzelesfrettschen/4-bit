# Roadmap

- Tier 1: 4040 CPU, Disassembler, Waveform Viewer. [In Progress]
- Nightly toolchain enabled; clippy warnings-as-errors gate.
- Dependencies: modular-bitfield, bitflags, tinyvec/smallvec; portable-simd for cluster simulation.
- Dual UI System [PLANNED]
  - Features: cli, tui(ratatui+crossterm), gui(pixels+winit); build-time selectable, run-time `--mode` flag.
  - CLI: batch, fuzzing, scripting; TUI: era-accurate terminal UI; GUI: framebuffer graphics and waveform.
  - Shared debugger controller and panels; consistent shortcuts.
- Sync with mcs4-emu/STATUS.md.
