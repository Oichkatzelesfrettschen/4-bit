# Roadmap

- Tier 1: 4040 CPU, Disassembler, Waveform Viewer. [In Progress]
- Nightly toolchain enabled; clippy warnings-as-errors gate.
- Dependencies: modular-bitfield, bitflags, tinyvec/smallvec; portable-simd for cluster simulation.
- SIMD Cluster Execution: implement CpuStateSimd (SoA), vectorized fetch/decode/execute, fuzz harness, and property tests (scalar vs SIMD) [PLANNED]
- Sync with mcs4-emu/STATUS.md.
