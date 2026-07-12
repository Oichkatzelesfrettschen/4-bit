# FPGA Board Clock and Physical Conformance Blockers

## Current boundary

The repository validates a host HDL system, not a deployed FPGA system.
`mcs4_system_core.v` integrates generated 4004, 4001, and 4002 models with
the monitor ROM, RAM, two-phase generator, and UART bridge.
`mcs4_system_sim_top.v` drives that core from an explicit host clock and
passes Icarus simulation and clean Verilator lint through
`scripts/verify_hdl_exports.py`.

`mcs4_top.v` requires `sys_clk_in`. The Gowin constraint file intentionally
does not assign that input because no board clock route, pin, electrical
standard, or measured frequency is retained. `gowin_prog` and `gowin_flash`
therefore refuse to program hardware until a versioned JSON clock contract
matches the exact route, source bytes, constraints, generated SDC, timing
report, timing paths, and bitstream. A synthesis artifact created without that
evidence is useful only for host-side resource inspection; it is not a
deployable result.

The virtual Qt6 board remains an i4003-only host adapter. The headless
Verilator system adapter executes the shared generated 4004, 4001, and 4002
HDL system, emits mapped JSONL frames, VCD, and monitor invariants. Both are
host simulations. Neither executes a common behavioral transcript, proves
target synthesis, or establishes FPGA board equivalence.

## Promotion order

The promotion order is strict. Later evidence does not repair a missing earlier
boundary.

1. Record the board clock route and electrical contract.
2. Apply the reviewed route to the constraint file.
3. Synthesize the exact source and constraint set for the named target.
4. Retain timing, utilization, bitstream, and tool-version evidence.
5. Program an attended board and capture the declared probe vectors.
6. Repeat the probe after a cold power cycle and rollback test.

## Blocker records

### gowin-sys-clk-route-evidence

Status: blocked.

Required input:

- Board model, revision, FPGA part, package, and power configuration.
- Schematic or vendor board document that identifies the physical clock source
  and route to the FPGA.
- `sys_clk_in` pin, bank, `IO_LOC`, `IO_TYPE`, nominal frequency, measured
  frequency, conservative timing frequency, duty cycle, and measurement
  method.
- Source URL or retained document hash, observer, date, and tool or instrument
  identity.

Acceptance condition:

- A reviewed schema-1 JSON evidence record names the exact target device and
  top module, board revision, reviewer, route record, `sys_clk_in`, nominal
  frequency, measured frequency, conservative timing frequency, `IO_LOC`, and
  `IO_TYPE`.
- `constraints/mcs4_gowin.cst` assigns and electrically declares the same
  `sys_clk_in` route with an exact constraint SHA-256.
- The preflight hashes every named static source input and generates an exact
  `create_clock` SDC from the reviewed timing frequency and duty cycle.

Falsifiers:

- A measured frequency exceeds the timing frequency, differs from the UART or
  phase-generator assumption, or changes the generated SDC period.
- The board document names a different pin, bank, voltage, or clock source.
- The constraint has an unassigned, incompatible, or duplicate clock port.

### target-synthesis-timing-evidence

Status: blocked on `gowin-sys-clk-route-evidence`.

Required input:

- Exact source revision or source archive hash.
- Generated 4004, 4001, and 4002 HDL hashes.
- Target device, vendor-tool version, command log, constraint hash, generated
  SDC hash, source-contract hash, and monitor-ROM hash.
- Synthesis, place-and-route, timing, utilization, and timing-path reports.
- Bitstream hash, output filename, and generated build-evidence manifest.

Acceptance condition:

- The vendor flow completes for the exact constrained `mcs4_top` source.
- The report names the generated SDC rather than an unconstrained timing file,
  contains a `sys_clk_in` pinout row with the reviewed location and I/O type,
  and retains a setup timing-path section without a timing failure record.
- The generated build-evidence manifest hashes the source contract, PnR report,
  timing paths, and bitstream before programming begins.

Falsifiers:

- A source, generated HDL, constraint, SDC, report, timing-path, or bitstream
  hash changes after the contract is generated.
- Timing is unconstrained, names a different SDC, fails, or omits the board
  clock pinout.

### attended-board-probe

Status: blocked on `target-synthesis-timing-evidence` and physical board access.

Required input:

- Board and programmer identity, programming command, and programmed bitstream
  hash.
- Serial transcript and logic-analyzer or oscilloscope capture with timebase.
- Explicit reset, TEST, UART transmit, UART receive, and heartbeat vectors.
- Cold-power-cycle result, recovery procedure, and rollback artifact.

Acceptance condition:

- The probe replays the declared vector set on the named board.
- Captured UART and debug behavior match the host HDL scenario at the declared
  boundary.
- Reset and recovery behavior are observed rather than inferred.

Falsifiers:

- UART framing, phase order, reset behavior, or observed clock differs from the
  retained host or synthesis contract.
- The board cannot recover from the declared rollback procedure.

### physical-netlist-extraction

Status: blocked independently of FPGA host validation.

Required input:

- Provenance-backed mask or die images for 4001, 4002, and 4003, including
  licensing, resolution, layer identity, and coordinate transform.
- Contact, transistor, rail, and pad extraction artifacts with source hashes.
- Connectivity closure metrics, independent logical vectors, and a named
  mapping between physical nodes and logical ports.

Acceptance condition:

- Each chip has a coordinate-bearing transistor netlist with reproducible
  extraction, connected rails, and declared external-port mapping.
- Independent vectors test the extracted netlist against a behavioral contract
  without silently filling missing connectivity.

Falsifiers:

- A missing layer, disconnected rail, ambiguous pad, or vector mismatch keeps
  the affected chip below physical fidelity.

The existing 4004-oriented extraction artifacts do not imply equivalent
4001, 4002, or 4003 physical evidence.

## Current safe commands

```sh
just hdl-validate
make -C mcs4-emu/crates/mcs4-fpga gowin_lint
make -C mcs4-emu/crates/mcs4-fpga gowin_sim
```

These commands establish host HDL syntax, simulation, and bounded behavior.
They do not authorize `gowin_prog`, `gowin_flash`, or a hardware conformance
claim.

## Programming contract execution

The programming targets execute this order without treating a textual marker
as proof:

1. `gowin_clock_guard` validates the reviewed JSON clock record, the exact CST
   assignment, static source hashes, and generates the SDC.
2. `gowin_gen` emits the generated chip HDL.
3. `gowin_source_guard` validates the complete deployment source set and
   writes a source-and-clock contract.
4. `gowin_synth GOWIN_DEPLOYMENT_BUILD=1` adds the generated SDC to the Gowin
   project and produces the candidate bitstream.
5. `gowin_program_guard` validates the contract, PnR report, timing paths,
   bitstream, and generated build-evidence manifest.
6. Only then does `openFPGALoader` receive the bitstream path.

The repository deliberately contains no reviewed clock JSON record. The guards
remain executable and tested with synthetic contract fixtures, while every real
board-programming path remains blocked until physical evidence enters scope.
