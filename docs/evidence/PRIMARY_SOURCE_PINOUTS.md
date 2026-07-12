# Primary-source pinouts (evidence notes)

This file records **where** pinout information comes from (primary sources) so that layout/schematic anchor decisions can be reproduced and audited later.

## 4001 (ROM + 4-bit I/O)

- **Source**: `docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf`
- **How to locate**: `pdftotext -f 70 -l 70 docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf - | rg -n \"Figure 4-5\"`
- **Figure**: **Figure 4-5. 4001 Pin Configuration.**

Signal ↔ pin number (as rendered by `pdftotext`, plus diagram):

- Pins `1..4`: `D0..D3` (bidirectional data bus)
- Pin `5`: `VSS`
- Pins `6..7`: `φ1, φ2` (non-overlapped clocks)
- Pin `8`: `SYNC`
- Pin `9`: `RESET`
- Pin `10`: `CL` (clear input for I/O lines)
- Pin `11`: `CM-ROM` (chip enable from processor)
- Pin `12`: `VDD`
- Pins `13..16`: `I/O0..I/O3` (4-bit I/O port)

Notes:

- The repo’s schematic-side naming uses `CLK1`/`CLK2` for `φ1`/`φ2`, and `CM` for `CM-ROM` (see `docs/emulators/i4001-signals.txt`).
- Metal-mask edge tokens on 4001 are sparse and ambiguous; we now seed the external-pad anchors via cyclic angle-alignment (`docs/evidence/anchor_seed_suggestions_v0/4001_angle_alignment.json`) rather than trusting tokens like `S`/`C` as direct signal labels.
- `D0_PAD..D3_PAD` are stable from bottom-edge pad detections; `CLK1/CLK2/RESET/CL/SYNC/CM/IO0..IO3` are seeded from angle-alignment then remapped to incident nodes (see `docs/evidence/schematic_layout_anchors_v1.json` notes).

## 4002 (320-bit RAM + 4-bit output port)

- **Source**: `docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf`
- **How to locate**: `pdftotext -f 78 -l 78 docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf - | rg -n \"Figure 4-14\"`
- **Figure**: **Figure 4-14. 4002 Pin Configuration.**

Signal ↔ pin number (as rendered by `pdftotext -layout`, plus diagram):

- Pins `1..4`: bidirectional data bus (`D0..D3`)
- Pin `5`: `VSS`
- Pins `6..7`: `φ1, φ2` (non-overlapping clocks; schematic naming uses `CLK1`/`CLK2`)
- Pin `8`: `SYNC` (input from CPU)
- Pin `9`: `RESET`
- Pin `10`: `P0` / `Po` (chip selection metal-option input)
- Pin `11`: `CM` (command input driven by CPU `CM-RAM`)
- Pin `12`: `VDD` (main power supply; “VSS − 15V ± 5%” per manual)
- Pins `13..16`: 4-bit **output port** (CPU→user system; exact bit names vary by schematics)

## 4003 (10-bit shift register)

- **Source**: `docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf`
- **How to locate**: `pdftotext -f 84 -l 84 docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf - | rg -n \"Figure 4-21\"`
- **Figure**: **Figure 4-21. 4003 Pin Configuration.**

Signal ↔ pin number (as rendered by `pdftotext -layout`, plus diagram):

- Pin `1`: `CP` (clock pulse input)
- Pin `2`: `DATA IN` (serial input)
- Pins `3,4,6,7,8,9,10,11,12,13`: `O0..O9` (parallel outputs)
- Pin `15`: `Serial out` (serial output)
- Pin `16`: `E` (enable; E=0 outputs valid, E=1 outputs at VSS per manual)
- Pin `5`: `VSS` (explicit in text)
- Pin `14`: `VDD` (confirmed from the Figure 4-21 diagram; this is not reliably captured by `pdftotext` in our current extraction pipeline)

Notes:

- The repo’s schematic-side naming uses `CLOCK`/`DATA`/`EN`/`OUT` for the serial interface, and uses `Q0..Q9` for the 10 parallel outputs (see `docs/emulators/i4003-signals.txt`). These correspond to the primary-source `O0..O9` pins.

### External behavioral contract

- The retained MCS-40 Users Manual OCR at
  docs/evidence/ocr/mcs40_users_manual.txt lines 8855-8859 states that a CP
  transition from 0 to 1 shifts data in.
- Lines 8888-8899 and 8963-8972 state that E low exposes valid parallel
  output data, E high drives the parallel outputs to VSS, and serial output
  remains unaffected so devices can cascade.
- Lines 8974-8977 state that power-on clear clears the shift register before
  the first CP signal. They do not establish a power-on level for the
  externally driven E input.
- Lines 10423-10434 specify a 200 to 1250 ns CP-to-serial-output delay for
  the stated load condition.

The behavioral Rust and generated behavioral Verilog models enforce this
external contract. The standalone Rust constructor holds E low only so its
parallel outputs remain observable until a caller drives the pin. That
convenience default is not a primary-source claim about package power-up.
The generic behavioral Verilog module represents power-on clear with an
initialized shift register. The FPGA-safe module clears the same state through
its host reset. Target synthesis must verify that either mechanism maps to the
deployed hardware reset or initialization behavior.
The behavioral models do not schedule the retained CP-to-serial-output delay;
that timing remains a fidelity and hardware-validation boundary.

The retained text does not establish a complete per-stage Q0 through Q9
state-order vector relative to serial output. It therefore does not establish
equivalence with the partial extracted 4003 gate artifact.

## 4004 (CPU)

- **Source**: `docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf`
- **How to locate**: `pdftotext -f 13 -l 14 docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf - | rg -n \"Figure 1-1\"`
- **Figure**: **Figure 1-1. 4004 Pin Configuration.**

Signal ↔ pin number (as rendered by `pdftotext -layout`, plus diagram):

- Pins `1..4`: bidirectional data bus (`D0..D3`)
- Pin `5`: `VSS`
- Pins `6..7`: `φ1, φ2` (non-overlapping clocks; schematic naming uses `CLK1`/`CLK2`)
- Pin `8`: `SYNC` (output to ROM/RAM)
- Pin `9`: `RESET`
- Pin `10`: `TEST`
- Pin `11`: `CM-ROM`
- Pin `12`: `VCC` (main supply voltage to the processor; naming varies across PMOS-era docs)
- Pins `13..16`: `CM-RAM0..CM-RAM3`
