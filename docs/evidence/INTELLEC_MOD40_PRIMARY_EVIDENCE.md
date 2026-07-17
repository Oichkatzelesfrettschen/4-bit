# Intellec 4 MOD 40 Primary Evidence Map

## Scope

This record separates the standard Intellec 4/MOD 40 from a generic MCS-40
machine. It records only claims that a retained Intel source supports. The
candidate MON4 listing, MAME driver, and photographs remain secondary evidence.
They guide recovery work but do not authorize historical execution.

## Retained primary sources

| Source ID | Locator | Supported fact |
| --- | --- | --- |
| `intellec4-mod40-reference-manual-98-095a` | Printed pages 7-9 and Figure 1-1 | The standard machine contains an imm4-43 central processor module, imm4-72 control module, imm6-28 program-RAM module, imm6-76 PROM programmer, and control/display panel. |
| `intellec4-mod40-reference-manual-98-095a` | Printed pages 17-18 and Figure 2-11 | The functional description uses a 4289 to simulate 4001-style program memory and explicitly names four 4702A PROMs. This conflicts with the board-local 98-013A CPU schematic, which labels A1-A4 as 1702A. |
| `intellec4-mod40-reference-manual-98-095a` | Printed pages 20-23 and Figures 2-13 through 2-25 | The central processor module contains TTY receiver, transmitter, and reader-control circuits. |
| `intellec4-mod40-reference-manual-98-095a-chipdb` | PDF pages 144, 150, and 152, printed pages 128, 134, and 136 | The terminal uses 11 symbols per character at 9.09 ms per symbol. ROM 0 bit 0 reads a start bit as one, RAM 0 bit 0 drives marking current when high, and RAM 1 bit 0 enables the reader when high. |
| `mcs40-users-manual-nov74` | Printed pages 5-1 through 5-2 and Figure 5-1 | The MOD 40 control module selects monitor, RAM, or PROM program storage. |
| `mcs40-advance-specifications-sep74` | PDF page 1 and device sections | I4201, I4289, and I4308 are generic MCS-40 components with separate clock, standard-memory-interface, and mask-ROM roles. |
| `intel-1975-data-catalog` | Printed pages 2-33 through 2-35 | I2102 is a 1024 by 1 static RAM with chip enable, active-low write pulse, tri-state output, and a write waveform that retains data as `R/W` rises. |
| `intel-1975-data-catalog` | Printed pages 5-3 through 5-6 | I3404 is a six-bit latch with two active-low write enables and inverted outputs; it is not a generic buffer or NAND device. |
| `intellec4-mod40-reference-schematics` | 98-013A PDF pages 3 and 29 | Reviewed CPU-module and TTY sheets establish the source-backed component and connector boundary below. |

## Source-backed standard topology

```text
Control/display panel
        |
        v
imm4-72 control module ---- imm6-28 program RAM (2102)
        |                         |
        +----- mode and bus ------+
        |
        v
imm4-43 central processor module
  4040 + system clock + 4289 + four 1702A monitor PROMs + four 4002 RAMs
        |
        +----- imm6-76 PROM programmer
        +----- TTY receiver, printer transmitter, reader relay
```

The standard topology does not establish I4201, I4308, or I4101 as installed
MOD 40 components. The reference manual explicitly states that neither 4001
nor 4308 ROMs occur anywhere in the standard MOD 40. I4201 and I4101 remain
generic MCS-40 family capabilities until a reviewed 98-013A sheet identifies
their placement.

## Reviewed schematic sheets

| PDF page | Drawing | Title | Established boundary |
| --- | --- | --- | --- |
| 3 | 410-2000318 | Schematic Central Processor Module | Shows the central-processor card's 4040, 4289, four 1702A sockets at A1-A4, four 4002 sockets, oscillator/clock logic, and TTY circuitry. It does not identify an I4201, I4308, or I4101 on that card. |
| 29 | 410-2000325 | Schematic TTY, Ref Int 4/MOD 4 | Shows the CPU-board P4/J4 terminal nets passing through J42/PA2 and rear-panel J43/P43 to the ASR 33 terminal strip, including the printer, keyboard, and tape-reader-control loops. |

The remaining 98-013A sheets require per-sheet title, connector, net, polarity,
and component extraction before they enter a board-level execution model. A
sheet scan establishes an electrical claim only after that extraction records
the source location and both endpoints.
`docs/evidence/INTELLEC_MOD40_SCHEMATIC_INDEX.md` indexes all 35 scan pages
and identifies the ten pages that remain unreadable at the review resolution.

The PDF 3 monitor region provides a direct intermediate boundary: I4289 A5
fans A0 through A7 to the four 1702A address buses, while `ENABLE MON PROM`
enters active-low enable 2G of 74155 A18. A18 also receives the labeled `C0`,
`C1`, `OUT`, `C2`, and `C3` signals. The source establishes the selected
monitor path as decoded logic, not a direct ROM attachment. It does not yet
prove the socket select order, selected-ROM polarity, or ROM byte inversion.

## Monitor device population conflict

The two retained Intel primary sources disagree about the monitor-device type.
The board-local 98-013A PDF 3 schematic labels the four fitted locations A1,
A2, A3, and A4 as `1702A`; the high-resolution sheet also shows their shared
A0 through A7 address wiring and individual `CSO` pins. The 98-095A functional
manual instead explicitly calls its four simulated program-memory devices
`4702A` on printed pages 17 and 18.

The repository models the physical-board profile from the board-local
schematic and therefore uses four 1702A slots. The available sources do not
identify whether the manual describes another board revision, configuration,
or documentation error. The manual remains valid evidence for the behavioral
4289, clock, and terminal descriptions that do not depend on device identity;
it does not establish the fitted monitor population for the retained CPU-card
schematic. A physical-board photograph with readable part markings and a
revision-linked bill of materials are required to close this source conflict.

## STOP acknowledge endpoint-polarity conflict

The manual contains a separate polarity conflict that blocks panel behavior.
Its CPU description on printed page 15 states that the 4040 clamps the CPU
`STOP ACKNOWLEDGE` status line low while halted. Its external-interface
description on printed page 129 states that the `STOP ACKNOWLEDGE` status line
is high while the processor is halted, while also describing remote `STOP` as
a temporary clamp low.

These statements can describe different endpoints separated by board logic,
or they can reflect a revision or documentation inconsistency. The available
sources do not yet trace the CPU output through the control card, panel, and
rear connector. The model records the physical S26 conditioning boundary only;
it does not assign STOP or STOP ACK polarity, restart behavior, or priority.

## Reviewed program-RAM cycle boundary

The 98-095A manual, printed page 24, identifies the imm6-28 as four 1K by 8
blocks of 2102 devices. It states that normal program fetch addresses the
module in A1 through A3 and receives the requested byte in M1 and M2. It also
states that special instructions read and write program RAM during execution
phases, and that `PM`, `F/L`, memory address, and memory-data-in participate in
the interface with the imm4-72. This establishes the external cycle roles; it
does not replace the required schematic trace for every card-edge conductor,
polarity conversion, latch phase, or 2102 select/write input.

Printed pages 40, 47, and 52 add a bounded electrical and transaction result.
The imm6-28 `WRITE/READ` input writes on TTL low and reads on TTL high. During
WPM, `PM` coincides with X2; the 4289 F/L state selects `BYTE2` or `BYTE1`; and
the control module presents address, write data, byte selection, momentary
`MOD ENABLE`, and the write command together. The controller description also
states that its write-command generator inverts its internal high result before
routing the command to the RAM module. This proves card-input polarity and the
semantic transaction coincidence. It does not prove the exact connector-level
waveform after the PDF 5 and PDF 10 route stages, the 3404 latch enables, or
the final 2102 R/W pulse width and setup/hold intervals.

## Generic MCS-40 component boundary

I4201, I4289, I4308, and I4101 belong to the generic MCS-40 component set.
The advance specifications identify I4201 as the clock generator, I4289 as the
standard memory interface, and I4308 as a 1K by 8 mask ROM with four 4-bit I/O
ports. The MCS-40 Users Manual identifies I4101 as a 1K static RAM. Those facts
support a separately configured generic MCS-40 machine. They do not prove a
standard MOD 40 card, connector, net, or monitor topology.

## Terminal endpoint map

| Function | Central processor endpoint | Circuit | Evidence |
| --- | --- | --- | --- |
| Keyboard to machine | ROM 0 input bit 0 at A25-10 | Q5 receiver | 98-095A, printed pages 21-22 |
| Machine to printer | RAM 0 output bit 0 | Q4 transmitter | 98-095A, printed page 22 |
| Reader run control | RAM 1 output bit 0 | Q3 relay driver | 98-095A, printed page 22 |

The terminal model uses these typed endpoints. It does not route printer or
reader output through a ROM port. The source identifies current-loop operation,
a 20 mA terminal conversion, and external current-limit components. Drawing
410-2000325 additionally identifies the CPU-board TTY OUT, TTY IN, and RDR CONT
nets at the external connector boundary. Drawing 2000318 traces `TTY IN`
through Q5 and its TTL buffer to the ROM 0 bit 0 input path at A25. It also
traces the RAM 0 output-bit-0 path through Q4 to `TTY PRINTER` and the RAM 1
output-bit-0 path through Q3 to the reader-control relay.

The alternate 98-095A scan resolves the CPU-port convention without replacing
the board drawings. It defines a marking teleprinter line as current present
and a spacing line as current absent. Its input procedure reads ROM 0 bit 0 as
one during the current-absent start bit. It defines any odd RAM 0 value as a
mark and any even value as a space, and any odd RAM 1 value as reader enable.
This reconciles the Q5, Q4, and Q3 logical senses at the CPU-port boundary.
Transistor switching thresholds, reader-relay mechanics, and panel arbitration
remain open and continue to block a historical electrical simulation.

## STOP ACK route boundary

The reviewed sheets expose three local STOP ACK segments: imm4-43 P1 contact
30 enters an 8095 and R14 network at the A11 boundary; imm4-72 P1 contact 73
passes through A15 7404 to J2 contact 9; and front-panel J2 contact 9 enters
an A29 7417 buffer. The 98-095A manual, printed page 63, identifies the panel
signal as arriving from the memory-control module and states that the buffer
output lights RUN when the system runs. These observations establish local
components, one control-card inversion, and the RUN-indicator function. They
do not establish the connector and cable mapping between the three segments,
CPU-pin direction, electrical asserted level, or a shared net. The existing
STOP ACK polarity conflict therefore remains open and blocks panel-arbitration
execution.

## Adjacent public cross-check material

The retained Intel Insite 4004/4040 library index identifies an Intellec 4/40
Silent 700 terminal interface and an Intellec 4 plus ASR-33 PROM dump utility.
The same public sweep retains secondary CPU, control-card, and program-RAM
connector transcriptions plus an external current-loop interface drawing.
These artifacts can expose OCR conflicts and software expectations, but the
98-013A visual drawing and the retained Intel manuals control every board-net,
polarity, timing, and monitor-media assertion. The Insite reference numbers
are not PDF page numbers; extraction waits for actual-page identification.
`INTELLEC_MOD40_PUBLIC_DISCOVERY_LOG.md` records the source boundaries.

## Monitor recovery boundary

The board-local CPU schematic shows four 256-byte 1702A PROM slots. The local-only
`mon4-v21-listing-rehost` scan and `mod40-cpu-board-photo-kyle` photograph
provide a candidate V2.1 listing and visible MON 4 labels. Their provenance is
secondary and the recovery page records inversion, transcription, patch, and
damage risks. The historical profile remains blocked until all four bytestrings
match independent PROM reads, their inversion convention is recorded, and each
device digest is verified.
`docs/evidence/INTELLEC_MOD40_MON4_RECOVERY_PROTOCOL.md` records the required
raw-read provenance and the unresolved physical-read lineage.

`docs/evidence/INTELLEC_MOD40_EXECUTION_GATE_STATUS.md` separates the five
independent closure conditions: physical media, monitor electrical mapping,
program-RAM transactions, panel and terminal control, and a source-tagged
historical trace. A candidate byte artifact never closes an electrical gate,
and a complete schematic trace never substitutes for two independent raw
physical acquisitions.

## Next evidence actions

1. Index each 98-013A sheet by title, drawing number, module, and net labels.
2. Extract the imm4-43 clock, 4289, PROM, RAM-port, and terminal nets from the
   reviewed sheets.
3. Extract the imm4-72 to imm6-28 address, byte, write, and mode-control nets.
4. Reconcile the four candidate MON4 images against at least two independent
   PROM reads and a hand-checked listing.
5. Add a source-tagged reset-to-prompt trace before enabling historical boot.
