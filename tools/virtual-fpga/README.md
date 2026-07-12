# MCS-4 Virtual FPGA Board

This optional Qt6 and Verilator toolset has two host-only targets:

- `mcs4-virtual-fpga` provides the Qt6 Intel 4003 board and its chip-level
  headless scenario.
- `mcs4-virtual-system` executes the shared generated 4004, 4001, and 4002
  system HDL through `mcs4_system_sim_top`.

Both targets are host simulations. Neither claims synthesis, timing closure,
pin assignment, or physical-board validation.

The board uses `i4003_fpga`, which samples CP through `sys_clk`. One board CP
pulse holds CP low, samples it high, and returns it low across three system
cycles. Reset holds the model reset for two system-clock cycles before release.

## Build and test

```sh
cmake -S tools/virtual-fpga -B build/virtual-fpga -G Ninja
cmake --build build/virtual-fpga
ctest --test-dir build/virtual-fpga --output-on-failure
```

## Run the board

```sh
build/virtual-fpga/mcs4-virtual-fpga
```

## Run the shared MCS-4 system scenario

The CMake configuration stages `monitor_rom.hex` in its build directory so the
system model has a deterministic ROM input.

```sh
build/virtual-fpga/mcs4-virtual-system \
  --headless \
  --scenario tools/virtual-fpga/scenarios/mcs4-system-monitor.json \
  --vcd build/virtual-fpga/mcs4-system-monitor.vcd \
  --trace-frames build/virtual-fpga/mcs4-system-monitor.trace.jsonl \
  --summary build/virtual-fpga/mcs4-system-summary.json
```

The system scenario drives reset, TEST, UART receive idle, and a bounded
5,000-system-cycle monitor run. Its oracle requires at least two WMP strobes,
a non-idle bus value, non-overlapping phi phases, and no observed bus
contention. The system trace exposes
stable phase, bus, CPU, control, bus-producer, and WMP signals. It is an FPGA
adapter trace, not a target-board or transistor-level trace.

## Run one common behavioral and FPGA stimulus

`mcs4-common-stimulus` is the exact input intersection shared by the behavioral
MCS-4 system and the full-system Verilator adapter. It contains exactly 256 ROM
bytes as `rom_hex`, begins with `reset`, and accepts only `set_test` and positive
`run_phases` actions. It rejects more than 100,000 actions or 1,000,000 requested
phase boundaries. UART input, ROM-port input, program-counter forcing, and raw
system-cycle stepping remain backend-specific and do not enter this contract.

```sh
cargo run --locked -p mcs4-system --bin mcs4-common-stimulus -- \
  --stimulus tools/virtual-fpga/scenarios/mcs4-common-nop.json \
  > build/virtual-fpga/mcs4-common-behavioral.trace.jsonl

build/virtual-fpga/mcs4-virtual-system \
  --headless \
  --scenario tools/virtual-fpga/scenarios/mcs4-common-nop.json \
  --vcd build/virtual-fpga/mcs4-common.vcd \
  --trace-frames build/virtual-fpga/mcs4-common-fpga.trace.jsonl \
  --summary build/virtual-fpga/mcs4-common-summary.json
```

The Verilator adapter writes the exact `rom_hex` bytes to a temporary file and
passes that file to the BSRAM only through its simulation-specific plusarg. The
Gowin and Icarus paths continue to use the reviewed `INIT_FILE` parameter. Both
traces hash the exact scenario JSON bytes and identify them as `scenario-json`.
The CTest comparison records shared-signal matches and mismatches in
`mcs4-common-comparison.json`. It normalizes behavioral control-bank identifiers
to FPGA control-line booleans and captures the FPGA observation before the HDL
phase register advances, matching the behavioral post-phase observation
contract. The report distinguishes `exact_equivalence` from
`within_mismatch_budget`; a nonzero budget bounds known behavioral-to-HDL debt
but never claims exact equivalence. The control scenario requires both adapters
to activate the RAM control surface and retains the raw and normalized values
for every recorded mismatch. This validates host-model agreement only; it does
not establish synthesis, timing, or physical conformance.

The checked-in NOP scenario permits at most 18 mismatching observations. The
checked-in ROM/RAM control scenario permits at most 89 and requires four active
RAM-control observations from each adapter. Any increase fails CTest and the
per-path report identifies the owning observable for the next HDL parity fix.

## Run a deterministic scenario

```sh
build/virtual-fpga/mcs4-virtual-fpga \
  --headless \
  --scenario tools/virtual-fpga/scenarios/i4003-shift-gate.json \
  --vcd build/virtual-fpga/i4003-shift-gate.vcd \
  --trace-frames build/virtual-fpga/i4003-shift-gate.trace.jsonl \
  --summary build/virtual-fpga/i4003-summary.json
```

Scenario actions are `reset`, `set_data`, `set_enable_n`, `set_e`, `pulse_cp`,
and `run_sys_cycles`. `set_e` is a compatibility spelling for the physical,
active-low E input. Every scenario has schema version 1 and can declare final
state assertions in `expect`.

Both headless adapters reject scenario JSON larger than 8 MiB, more than
100,000 actions, more than 10,000,000 cumulative system cycles, more than
100,000 trace frames, more than 64 MiB of JSONL trace output, and VCD execution
beyond 1,000,000 simulated system cycles. The full-system adapter also rejects
a scenario that requests its cycle limit after reset work. These limits bound
host resource use; they do not model FPGA resource limits.

`--trace-frames` writes one JSONL record per completed scenario action. Each
record uses the shared trace-frame schema carried by `mcs4-system`, declares
the `i4003-fpga-verilator` adapter provenance, and exposes CP, serial input,
active-low E, parallel output, serial output, and system-cycle signals. These
are action-boundary chip-adapter observations, not MCS-4 CPU bus phases. They
cannot establish whole-system equivalence with behavioral MCS-4 frames until a
shared system wrapper exposes matching signals and stimulus.

The full-system target emits a frame on reset, input actions, and every observed
CPU phase transition during `run_sys_cycles`. Its `mcs4.*` signal paths map to
the behavioral frame names. The monitor scenario remains backend-specific. The
common stimulus emits frames only at shared phase boundaries and records both
adapters with the same scenario JSON digest. The comparison requires matching
stimulus identity before it evaluates signal equality.

The shared `mcs4.control.rom` path reports the ROM selection latch. The shared
`mcs4.control.ram` path reports the completed CM-RAM transfer selection, which
can clear between phases. FPGA-only `mcs4.fpga.rom_selected` and
`mcs4.fpga.ram_selected` paths expose the corresponding wrapper latches. The
bus trace retains the most recently driven nibble during idle intervals and
exports `mcs4.fpga.bus_driven` so an idle retained zero remains distinct from a
driven zero.
