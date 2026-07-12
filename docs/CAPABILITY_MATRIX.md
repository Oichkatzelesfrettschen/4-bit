# Capability Matrix

This matrix separates implemented code from tested behavior, reproduced host
workflows, synthesis, hardware probes, and blocked work. The machine-checkable
records live in `docs/meta/capabilities.json` and the verifier rejects missing
evidence paths, undeclared maintenance roles, or unsupported state values.

Maintenance roles own evidence upkeep and the next gate. They do not assert a
person, funding commitment, or hardware availability.

| Capability | State | Owner | Evidence boundary | Next gate |
|---|---|---|---|---|
| 4003 behavioral boundary | tested | emulator-maintainers | Rust and behavioral HDL vectors | Gate-stage and timing evidence |
| Typed HDL export | tested | emulator-maintainers | Typed request, checksum manifest, HDL parser and linter checks | Target synthesis report |
| Shared MCS-4 FPGA HDL system | tested | emulator-maintainers | Deterministic Icarus system simulation and clean Verilator lint | Constrained target synthesis report |
| MCS-4 and MCS-40 phase timing traces | tested | emulator-maintainers | Deterministic fixture traces | Calibrated transient comparison |
| Cross-fidelity trace and replay | tested | emulator-maintainers | Versioned JSONL frames, stimulus hashes, transcript checkpoints, and read-only GUI import | Common behavioral ROM and input transcript through both adapters |
| Timing parameter provenance | tested | evidence-maintainers | OCR locator, bounds, code use-site, and falsifier ledger | Calibrated transient or waveform comparison |
| 4040 solver bridge reference | tested | evidence-maintainers | Explicit 4004 reference graph only | Provenance-backed 4040 netlist |
| SIMD ADD subset | tested | emulator-maintainers | Deterministic scalar differential oracle | Measured workload contract |
| Netlist v1 publication | reproduced | evidence-maintainers | Default byte-identical regeneration and hash manifest | Repository-wide provenance gate |
| 4003 virtual FPGA board | reproduced | emulator-maintainers | Qt6 and Verilator headless VCD, JSON summary, and JSONL adapter frames | Synthesis and attended board probe |
| MCS-4 system Verilator adapter | reproduced | emulator-maintainers | Generated 4004/4001/4002 HDL, VCD, mapped JSONL frames, and monitor invariants | Shared behavioral ROM and input transcript |
| Developer proof bundle | implemented | evidence-maintainers | Clean-tree builder and focused contract tests | Clean-revision retained bundle |
| Physical FPGA conformance | blocked | hardware-maintainers | Host HDL exists; clock-route, target timing, and attended-board evidence do not | Target bitstream and waveform-backed probe |

`tested` means a scoped oracle passes. `reproduced` means a deterministic
workflow recreates the scoped artifact or result. Neither state implies
synthesis or physical conformance.

The shared HDL system remains a host-only boundary. `mcs4_top` requires a
reviewed `sys_clk_in` route before programming. The physical promotion criteria
and transistor-extraction blockers live in
`docs/evidence/fpga-board-clock-and-conformance-blockers.md`.
