# Intellec Source-Bound Architecture

## Scope and truth boundary

The repository models the Intel Intellec 4 family through explicit evidence
profiles. A profile names its cards, source documents, clock rate, and only
the connector wires that a retained source identifies. The implementation
rejects a historical phase advance or monitor load when its profile lacks the
operator manual, console net map, terminal port map, physical monitor-media
provenance, transform record, or per-device digest. A bench profile remains
available for deterministic tests and always identifies itself as
nonhistorical.

This boundary prevents three invalid substitutions:

- A generated test monitor never becomes historical monitor firmware.
- A host UI action never becomes a panel, backplane, or terminal wire event.
- A generic 4004 FPGA wrapper never becomes an Intellec 4 or MOD 40 FPGA.

## Implemented composition

`mcs4-intellec` provides these source-bound components:

- `IntellecProfile` records model identity, card inventory, primary-source
  references, unresolved evidence, and typed terminal endpoints. A terminal
  endpoint may be a ROM input bit or a RAM output-port bit; it never treats
  those distinct devices as interchangeable.
- `IntellecPanel` converts switches and controls into named reset, TEST, and
  console-memory requests. It does not mutate ROM or CPU state directly.
- `IntellecMachine` serializes panel drives, terminal wires, and one MCS bus
  phase into an immutable `IntellecFrame`. It rejects an absent or invalid
  terminal ROM/RAM endpoint before advancing a phase; it never substitutes an
  idle terminal value for an unmodeled port.
- `MonitorRom` verifies a SHA-256 digest, model identity, and load address
  before a nonhistorical machine receives ROM bytes. Digest agreement proves
  bytes, not a physical read, socket identity, transform, or independent
  provenance; the historical profile keeps those as separate gates.
- `IntellecReplaySession` records only external panel and terminal events plus
  phase advances, then rebuilds a fresh target to check the final frame.
- `Teletype33` models ASR-33 keyboard transmit, printer receive, paper reader,
  and punch state with a rational 110-baud, 8N2 symbol clock. It exposes serial
  wires, not a CPU-visible byte FIFO.
- The egui Intellec workspace renders the panel, profile gate, terminal paper,
  and punch output. The original profile remains visibly blocked until its
  evidence set is complete.

The older `IntellecSystem`, front panel, monitor, and UART remain compatibility
fixtures for existing tests. They use generated monitor behavior and direct
host memory helpers. They do not supply historical-fidelity evidence.

## Evidence ledger

`docs/evidence/intellec_sources.yaml` is the source of truth for retained and
local-only manuals. `scripts/fetch_intellec_sources.sh` uses a fixed Mozilla
user agent, HTTPS-only downloads, temporary files, and SHA-256 verification.
It does not commit scans with unresolved redistribution terms. The ledger
currently leaves the original Intellec 4 operator manual, console schematics,
and monitor PROM set unresolved. It retains the MOD 40 reference manual, the
primary schematic set, a candidate MON4 V2.1 listing, a visual CPU-board
reference, and documented secondary MON4 read artifacts. The candidate
listing, photograph, and public read artifacts remain secondary evidence.
They do not satisfy the MOD 40 monitor-PROM gate.
`docs/evidence/INTELLEC_MOD40_PRIMARY_EVIDENCE.md` records the page-level
module and terminal claims that the implementation consumes.
`docs/evidence/INTELLEC_MOD40_MON4_READ_MATRIX.md` records public read
provenance, raw-artifact digests, disagreement counts, and the remaining
physical-read acceptance work.
`docs/evidence/INTELLEC_MOD40_EXECUTION_GATE_STATUS.md` separates the media,
electrical, cycle, panel/terminal, and trace closure conditions.

## MOD 40 boundary

`Mcs40System` is a behavioral compatibility assembly. It contains a 4040,
4201, 4289, and MCS-4 4001/4002 memory devices. It does not represent the
standard Intellec 4/MOD 40 backplane.

The primary MOD 40 reference manual defines the standard machine differently:

- The imm4-43 central processor module contains the 4040, system clock, 4289,
  four 1702A PROMs for the one-kilobyte monitor, and four 4002 RAMs.
- The imm4-72 control module selects monitor, program RAM, or PROM execution.
  The imm6-28 program-RAM module uses 2102 static RAM, and the imm6-76 module
  programs 1702A PROMs.
- The terminal receiver reaches ROM 0 input bit 0. The printer transmitter
  uses RAM 0 output bit 0, and the reader relay uses RAM 1 output bit 0.
  The manual identifies these as the Q5 receiver, Q4 transmitter, and Q3
  reader-control circuits.

The manual does not identify I4201, I4308, or I4101 as standard MOD 40 card
components. It explicitly excludes 4001 and 4308 ROMs from the standard MOD 40
configuration. These components remain generic MCS-40 family components until
a schematic sheet proves a specific MOD 40 placement. A dedicated MOD 40 board
model must therefore prove the following independently before the profile
becomes bootable:

1. The imm4-43 clock circuit drives exactly eight accepted CPU phases per
   machine cycle with source-backed reset, STOP, and single-step behavior.
2. The 4289 multiplexes the imm4-43 monitor PROM path and the imm4-72 program
   RAM path without a 4001/4002 compatibility fallback.
3. The imm6-28 2102 program-RAM path preserves all 32 physical devices, their
   four-bank by eight-lane organization, unknown power-on state, and 3404
   active-low-write address-latch boundary. It does not substitute direct
   address, byte, selection, or write behavior for the unextracted board cycle.
4. The source-bound terminal endpoints preserve the ROM-input and two-RAM-port
   distinction, including current-loop polarity and reader relay behavior.
5. Four revision-identified 1702A monitor images reproduce a retained boot
   phase trace after inversion convention and per-device digests are verified.

## FPGA and Verilator boundary

The existing Verilator target is a generic generated 4004, 4001, and 4002
system. Its shared control paths now expose the behavioral CM-RAM selection
code as a four-bit value and expose the generic RAM-device ownership under a
separate FPGA-only signal. They do not report selected Intellec cards or
terminal connector wires. The target is useful for generic HDL regression, not
historical Intellec equivalence.

An evidence-gated Intellec FPGA target requires these distinct observables:

- panel address, write-data, controls, strobe, and lamps;
- valid selected ROM and RAM card identifiers, not boolean selection or a CM
  pulse substituted for selection;
- resolved bus value, driver mask, and retained idle value;
- ROM I/O input, output, read, write, and source-named slot identifiers;
- terminal receive, transmit, reader-control, reader-data, and punch-control
  wires after a source establishes each mapping;
- profile, ROM, RTL, terminal-timing, and source-manifest digests in every
  comparable trace.

No FPGA profile may use a temporary fixture ROM as historical monitor firmware.
No trace may compare a CM pulse against a behavioral selected-card value.

## Capture and acceptance plan

`scripts/callgraph_capture.sh` captures lexical cflow and cscope maps, MIR
direct-call extraction, focused Intellec source-gate and replay tests, generic
Verilator runs, and a checksummed source snapshot. The verifier requires every
named status surface. The cflow and cscope Rust maps remain lexical evidence;
MIR and runtime traces establish compiler and execution reachability.

The work completes in this order:

1. Index the retained console, terminal, CPU-module, control-module, and
   motherboard sheets into page and net records.
2. Reconcile the candidate MON4 V2.1 listing with independent four-PROM reads,
   record the inversion convention and per-device digests, and add a golden
   source-tagged boot trace.
3. Build the dedicated imm4-43, imm4-72, and imm6-28 board assembly and prove
   its clock, program-memory, panel, and terminal paths.
4. Add the source-gated Intellec FPGA wrapper and selected-card trace contract.
5. Run behavioral and Verilator replay from one event transcript only after
   each field has matching semantics and provenance.

The current implementation delivers the source gate, event path, terminal
timing model, replay boundary, GUI, and capture contract. It does not claim a
complete original Intellec board, a complete MOD 40 topology, physical ASR-33
electrical behavior, FPGA timing closure, or transistor-level equivalence.

`Mod40Board` now owns a non-executable imm4-43, imm4-72, and imm6-28 card
inventory. Its CPU card has four unread `I1702A` monitor devices rather than
4702A substitutes. Its IN-28 card retains thirty-two unknown-state `I2102`
instances at the source-visible `1K` through `4C` locations. The model exposes
logical storage tests but does not attach those devices to a historical CPU
cycle. The source gate rejects that attachment until the control, clock, panel,
terminal, monitor-image, and electrical net records close.

`mod40_routes.rs` carries only the directly reviewed topology into typed code:
the ten low IN-28 address contacts, the C2/C3 high-address boundary, the three
TTY cable conductors, the shared eight-line monitor-address fanout, and the
eight visible A18 decoder output pins. The output records remain partial because
the source has not yet traced them through downstream gates to C1702A select
pins. The remaining byte, module-select, write, decoder, polarity, and timing
records remain partial. `Mod40SourceGate` exposes that distinction and continues
to reject board-cycle execution.
