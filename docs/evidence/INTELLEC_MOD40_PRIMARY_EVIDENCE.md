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
| `intellec4-mod40-reference-manual-98-095a` | Printed pages 17-18 and Figure 2-11 | The imm4-43 uses a 4289 to simulate 4001-style program memory with four 1702A monitor PROMs. |
| `intellec4-mod40-reference-manual-98-095a` | Printed pages 20-23 and Figures 2-13 through 2-25 | The central processor module contains TTY receiver, transmitter, and reader-control circuits. |
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
enters active-low enable 2G of 74155 A8. A8 also receives the labeled `C0`,
`C1`, `OUT`, `C2`, and `C3` signals. The source establishes the selected
monitor path as decoded logic, not a direct ROM attachment. It does not yet
prove the socket select order, selected-ROM polarity, or ROM byte inversion.

## Reviewed program-RAM cycle boundary

The 98-095A manual, printed page 24, identifies the imm6-28 as four 1K by 8
blocks of 2102 devices. It states that normal program fetch addresses the
module in A1 through A3 and receives the requested byte in M1 and M2. It also
states that special instructions read and write program RAM during execution
phases, and that `PM`, `F/L`, memory address, and memory-data-in participate in
the interface with the imm4-72. This establishes the external cycle roles; it
does not replace the required schematic trace for every card-edge conductor,
polarity conversion, latch phase, or 2102 select/write input.

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
output-bit-0 path through Q3 to the reader-control relay. An electrical
simulation remains blocked on complete polarity, terminal-side, and
component-value extraction.

## Monitor recovery boundary

The standard monitor occupies four 256-byte 1702A PROMs. The local-only
`mon4-v21-listing-rehost` scan and `mod40-cpu-board-photo-kyle` photograph
provide a candidate V2.1 listing and visible MON 4 labels. Their provenance is
secondary and the recovery page records inversion, transcription, patch, and
damage risks. The historical profile remains blocked until all four bytestrings
match independent PROM reads, their inversion convention is recorded, and each
device digest is verified.
`docs/evidence/INTELLEC_MOD40_MON4_RECOVERY_PROTOCOL.md` records the required
raw-read provenance and the unresolved physical-read lineage.

## Next evidence actions

1. Index each 98-013A sheet by title, drawing number, module, and net labels.
2. Extract the imm4-43 clock, 4289, PROM, RAM-port, and terminal nets from the
   reviewed sheets.
3. Extract the imm4-72 to imm6-28 address, byte, write, and mode-control nets.
4. Reconcile the four candidate MON4 images against at least two independent
   PROM reads and a hand-checked listing.
5. Add a source-tagged reset-to-prompt trace before enabling historical boot.
