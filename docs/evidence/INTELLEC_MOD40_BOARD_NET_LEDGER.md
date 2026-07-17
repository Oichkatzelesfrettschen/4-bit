# Intellec 4 MOD 40 Board Net Ledger

## Scope and evidence rule

This ledger records electrical claims from the retained 98-013A reference
schematic scan. It distinguishes a visible labeled endpoint from a complete
board connection. A row does not establish a runnable board cycle unless every
required endpoint, polarity, and timing condition is extracted and reconciled.

Status values:

- `direct`: the named signal and both local endpoints are visible on one sheet.
- `partial`: the named signal is visible, but its remote endpoint or complete
  path requires another reviewed sheet.
- `inventory`: a board layout establishes a fitted component, not a net.
- `open`: the sheet has not yet yielded the required endpoint record.

Source: `intellec4-mod40-reference-schematics`, SHA-256
`34f82ae6228f6ee0832c9696b4a0e922f8ea4bfccf92131476d013108b703bdb`.

## Motherboard and memory-control boundary

| Status | Source location | Record | Evidence and limit |
| --- | --- | --- | --- |
| partial | PDF 5, drawing 2000077 | Motherboard exposes the `MAD0` through `MAD11` group toward the RAM module. | The group labels occur at the motherboard RAM boundary. The complete connector-pin map and remote IN-28 pin mapping remain open. |
| partial | PDF 5, drawing 2000077 | Motherboard carries `CM-ROM`, `CM-RAM0` through `CM-RAM3`, and `CPU RESET`. | The labels establish the card-boundary signal set. They do not establish the active polarity or every physical route. |
| partial | PDF 5, drawing 2000077 | Motherboard exports TTY printer, keyboard, and reader-control labeled connections. | Terminal-side wiring requires reconciliation with PDF 29 and the rear connector board. |
| inventory | PDF 6, memory controller assembly | The controller board contains 3205, 3235, 3404, 8233, 8266, 9602, 74161, TTL, and discrete support population. | The layout supports a multi-device translation and arbitration model. It does not define the functional wiring alone. |
| direct | PDF 7, drawing 2000319 | Controller input labels include `WRITE`, `PM`, `MOD SEL 12` through `MOD SEL 15`, `MOD ENABLE`, `CMA EX`, `S/S PB`, `CMA WRITE`, `STOP ACK`, and `USER RESET`. | The labels enter the controller drawing at named connector contacts. Their panel and motherboard counterparts require cross-sheet resolution. |
| direct | PDF 7, drawing 2000319 | Controller exposes `CM-ROM`, `CM-RAM0` through `CM-RAM3`, `4002 RESET ENABLE`, `4002 RESET`, `ENABLE MON PROM`, `PROM SELECT`, and `CPU RESET`. | The source establishes controller-local logic around these signals. It does not yet establish a cycle-accurate Boolean/timing model. |
| partial | PDF 7, drawing 2000319 | Controller P1 contacts 11 through 20 carry `MAD0` through `MAD9`; contacts 90 and 92 carry `BYTE2` and `BYTE1`; contact 95 carries `WRITE`. | The controller-side contact assignment is visible. The remaining control-card Boolean equations and byte-cycle timing remain open. |

## Remaining sheet extraction records

| Status | Source location | Record | Evidence and limit |
| --- | --- | --- | --- |
| inventory | PDF 2, drawing 1000276-02 | CPU-board layout identifies sockets A1 through A4, A11 through A32, P1, P4, and the card edge. | The layout supports physical placement only. PDF 3 remains the electrical source for the central processor. |
| inventory | PDF 4, motherboard layout | Motherboard layout identifies J1 through J18 plus J22, J42, P37, P38, and P39. | The layout proves connector presence. PDF 5 and other endpoint sheets remain required for nets. |
| inventory | PDF 6, memory-controller layout | Controller layout identifies the A1 through A31 population, including TTL, Intel interface, and discrete devices. | The layout does not establish any controller signal polarity or timing. |
| inventory | PDF 12, drawing 1000143 | Front-panel logic layout identifies J1, J2, J3, P39, 8244/9348, 8233, 8266, 3205, 3404, and TTL placement. | The drawing does not connect a panel action to a CPU cycle. PDF 13 remains the electrical source. |
| direct | PDF 13, drawing 2000329 | Front-panel STOP pushbutton `S26` enters the panel logic through A36 conditioning gates as `STOP PB`. | The switch and local conditioning stages are visible. The resulting controller/CPU state transition and asserted polarity remain open. |
| direct | PDF 13, drawing 2000329 | The panel drawing exposes named `STOP ACK`, `CPU RESET`, `4002 RESET ENABLE`, `EOC SYNC`, and mode-selection inputs at its connector boundary. | The source establishes the panel-control signal set. It does not yet establish every cross-sheet endpoint, Boolean equation, or timing relation. |
| inventory | PDF 14, drawing 1000034-01 | Display layout identifies P40 and LED1 through LED46 as General Electric SSL-22 devices. | The board has a 46-LED physical population. Display segment polarity and panel logic mapping remain open. |
| partial | PDF 25, drawing 2000121 | Optional data-storage/OEM module contains sixteen 4002 devices, selection logic, `CM-RAM0` through `CM-RAM3`, `CPU RESET`, and `4002 RESET`. | This establishes an optional expansion topology, not the standard MOD 40 population or a complete board route. |
| direct | PDF 28, Power One CP110 and D5-12 S113 | The power drawings label +80 V, +12 V, -10 V, -12 V, +5 V, COM, and sense-output rails. | These are supply-level facts. Electrical startup sequencing and load behavior remain outside the digital execution model. |

## IN-28 program-RAM boundary

| Status | Source location | Record | Evidence and limit |
| --- | --- | --- | --- |
| direct | PDF 10, drawing 01-0176-001 | IN-28 contains thirty-two 2102 devices in four rows: `1K` through `1C`, `2K` through `2C`, `3K` through `3C`, and `4K` through `4C`. | The physical population is visible on the schematic. |
| direct | PDF 10, drawing 01-0176-001 | Each row orders locations `K, J, H, G, F, E, D, C`. | The source supports the model's bit-lane location naming. Byte-bit polarity and any inversion remain open until the data path is traced. |
| partial | PDF 10, drawing 01-0176-001 | IN-28 P1 contacts 11 through 20 receive `MAD0` through `MAD9`; contacts 94 and 96 receive `MAD11` and `MAD10`; contacts 90, 92, 93, and 95 carry `BYTE2`, `BYTE1`, `MODULE SELECT`, and `WRITE`. | The card-edge assignments are visible. The exact byte-cycle sequencing and complete control equation remain open. |
| partial | PDF 10, drawing 01-0176-001; Intel 1975 Data Catalog, printed pages 5-3 through 5-6 | The board uses 3404 active-low-write inverting latches, 7404 inversion stages, and TTL selection logic around the memory array. | The 3404 write-enable phases and their complete relationship to the external `WRITE/READ` net remain unextracted. The source does not authorize a static-buffer substitution. |
| direct | PDF 10, drawing 01-0176-001 | `BYTE2` at contact 90 and `BYTE1` at contact 92 visibly enter 7404 stages before joining local 7400-class logic. `MODULE SELECT` at contact 93 and `WRITE/READ` at contact 95 visibly enter separate 3404 channels before later local logic. | This establishes one local inversion-bearing stage for each named control input. It does not establish the final 2102 chip-enable or R/W state. |
| partial | Intel 1975 Data Catalog, printed pages 2-33 through 2-35 | Each 2102 is a 1024 by 1 static RAM with chip enable, a tri-state data output, and an active-low `R/W` write pulse that retains input data as the pulse ends. | The data sheet defines device behavior only. PDF 10 must still trace each board-level select, address, data, and write-pulse route. |

## Central-processor monitor boundary

| Status | Source location | Record | Evidence and limit |
| --- | --- | --- | --- |
| direct | PDF 3, drawing 2000318 | The central processor contains I4289 A5 and four Intel 1702A devices A1 through A4. A5 address outputs A0 through A7 fan out to the matching address inputs on all four monitor devices. | The shared eight-bit monitor-address wiring is visible. The fetched-byte path and timing still require a complete 4289 and memory-bus trace. |
| conflict | PDF 3, drawing 2000318; 98-095A printed pages 17-18 | The board-local PDF 3 labels A1 through A4 as 1702A. The functional manual explicitly names four 4702A devices for its simulated program-memory description. | The retained sources do not establish whether the difference is a board revision, configuration, or documentation error. The physical-board model follows the board-local schematic; no 4702A claim enters the MOD 40 board model without revision-linked physical evidence. |
| direct | PDF 3, drawing 2000318 | The board net `ENABLE MON PROM` enters 74155 A18 at active-low enable 2G. A18 also receives `C0`, `C1`, `OUT`, `C2`, and `C3` inputs. | The input labels and A18 pins are visible. The active state, output-to-PROM-select mapping, and every intervening gate remain unextracted. |
| partial | PDF 3, drawing 2000318 | A18 exposes output pins `1Y0` through `1Y3` at pins 7, 6, 5, and 4, and `2Y0` through `2Y3` at pins 9, 10, 11, and 12. The outputs leave the monitor-select region toward the A1 through A4 PROM area. | The visible pins establish a finite decoder-output boundary rather than a direct unqualified PROM enable. The source still does not establish each output's downstream gate path, socket order, selected-ROM polarity, or ROM-byte inversion. |
| direct | PDF 3, drawing 2000318 | Y1 is labeled 5.185 MHz and drives an A16 gate that reaches the A7 74161/9316 counter clock input. | This establishes the oscillator source, the A16 gate, and the first documented counter input. The derived phase waveform, counter state, and 4040 clock-pin route remain open. |
| direct | PDF 3, drawing 2000318 | The `CPU RESET` card-edge net reaches the `RESET` input at A11 4040 pin 12. | The physical endpoint is direct. The complete panel or controller source, assertion duration, and release relation to divider state remain open. |
| direct | PDF 3, drawing 2000318 | A18 receives `C0` at A pin 13, `C1` at B pin 3, and `ENABLE MON PROM` at active-low 2G pin 14. `OUT`, `C2`, and `C3` also enter the A18 monitor-select region. | This establishes the decoder inputs without deriving an output-to-socket equation. |
| partial | PDF 3, drawing 2000318 | The four C1702A D0 through D7 output groups occupy a shared monitor-data region with A10 and A8 74158 packages, output pull networks, and nearby 8095/TTL fabric. | Visual review establishes the finite components and region. It does not establish that every bit traverses each listed stage, the selected-device timing, data-bit polarity, or socket-order transform. |
| direct | PDF 3, drawing 2000318 | The monitor region contains two visible 74158 packages, A10 and A8, each with output inversion bubbles and a resistor-pull network. | This corrects the earlier A15 misidentification. The scan does not yet establish a complete per-bit route from C1702A output through a 74158 output and the downstream 4289/4040 path. |
| partial | PDF 3, drawing 2000318 | The 5.185 MHz oscillator drives the A7 74161/9316 counter through an A16 gate; counter outputs feed later 7404/7400-class clock logic. | The source establishes a divider network, not the complete phi1/phi2 waveform, reset release edge, or 4040 clock-pin timing. |
| partial | PDF 3, drawing 2000318 | A7 divider outputs enter a 74H00 and 7404 phase-conditioning network that feeds A32 MH0026. | The visible stages establish the phase-generation boundary. The seven-state counter equation, individual gate polarity, and output phase ordering remain unextracted. |
| direct | PDF 3, drawing 2000318 | A32 MH0026 output paths pass through R33 and R32 on the labeled phi1 and phi2 paths toward the A11 4040 clock inputs. | The physical phase-output endpoints are direct. This does not establish the pulse sequence, pulse width at the CPU pins, or reset-release edge. |
| partial | 98-095A printed pages 15-16 | The functional CPU description specifies a 5.185 MHz crystal, a divide-by-seven counter, and two non-overlapping negative clock pulses with nominal 386 ns width at 740.7 kHz. | This establishes a card-level timing target. It does not identify every physical board net, propagation delay, reset-release edge, or 4040 clock-pin waveform. |
| partial | 98-095A printed page 15 | `CPU RESET` is negative true, must remain asserted for at least eight full instruction cycles or 64 external clock periods, and resets the program counter to zero. `STOP` is negative true and asynchronous; the CPU completes the present instruction before acknowledging stop. | This establishes behavioral timing constraints. PDF 3 still requires a complete reset-conditioning and panel-to-CPU route extraction before a board-cycle model can use them. |

## Terminal connector routes

The following routes are direct, connector-to-connector observations from
PDF 29. They establish the cable harness boundary. They do not yet establish
the serial symbol polarity at the 4040-visible ports or a complete ASR-33
electrical model.

| Status | Source location | Route | Evidence and limit |
| --- | --- | --- | --- |
| direct | PDF 29, drawing 2000325 | CPU P4/J4 contact 26, `TTY OUT`, to J42/P42 contact 1, J43/P43 contact 1, and ASR-33 printer terminal 7. | The printer forward current-loop conductor is visible end to end. |
| direct | PDF 29, drawing 2000325 | ASR-33 printer terminal 6 to J43/P43 contact 2 and J42/P42 contact 2, then through 270 ohm to -10 V. | The return-side supply condition is visible. The CPU driver transistor behavior remains a separate CPU-board extraction. |
| direct | PDF 29, drawing 2000325 | CPU P4/J4 contact 1, `TTY IN`, to J42/P42 contact 4, J43/P43 contact 4, and ASR-33 keyboard terminal 4. | The keyboard forward conductor is visible end to end. |
| direct | PDF 29, drawing 2000325 | CPU P4/J4 contact 89, `RDR CONT`, to J42/P42 contact 5 and J43/P43 contact 5. | The named reader-control conductor crosses the motherboard and rear connector. The terminal-strip relay contacts and CPU-port assertion polarity remain open. |
| direct | PDF 29, drawing 2000325 | J42/P42 contact 3 supplies +5 V through 220 ohm; J42/P42 contact 6 supplies -10 V through 68 ohm. | The external current-loop bias rails are explicit. The remaining component and terminal contacts need a full current-loop extraction. |
| direct | PDFs 3 and 29, drawings 2000318 and 2000325 | The `TTY IN` conductor enters CPU P4/J4 contact 1, passes through the Q5 receiver circuit and its TTL buffer, and reaches the `ROM 0 BIT 0` input path at A25. | The external conductor and internal receiver path are source-visible end to end. The source-reconciled CPU-port sense is recorded below; threshold waveform remains open. |
| direct | PDFs 3 and 29, drawings 2000318 and 2000325 | The RAM 0 output-bit-0 transmitter path reaches Q4, then CPU P4/J4 contact 26, `TTY PRINTER`, and the external printer current loop. | The path and Q4 output-driver stage are visible end to end. The source-reconciled command-bit convention and serial framing are recorded below; driver waveform remains open. |
| direct | PDFs 3 and 29, drawings 2000318 and 2000325 | The RAM 1 output-bit-0 reader-control path reaches Q3, then CPU P4/J4 contact 89 and the reader-control relay current loop. | The path and Q3 relay-driver stage are visible end to end. The CPU-port logical convention is recorded below; relay state and reader mechanical timing remain open. |

## Terminal current-loop and panel-arbitration extraction

| Status | Source location | Record | Evidence and limit |
| --- | --- | --- | --- |
| direct | PDF 29, drawing 2000325 | The printer loop has a source-side +5 V transistor driver through 180 ohm to CPU P4/J4 contact 26, terminal contact 7, printer terminal 6, J43/P43 contact 2, J42/P42 contact 2, and a 270 ohm return to -10 V. | The physical loop direction is source-visible. The RAM 0 logical convention is reconciled below; transistor threshold and waveform behavior remain open. |
| direct | PDF 29, drawing 2000325 | The keyboard loop supplies +5 V through 220 ohm at J42/P42 contact 3, crosses terminal contact 3 and the keyboard mechanism, then returns from terminal contact 4 through J43/P43 and J42/P42 contact 4 to the CPU `TTY IN` receiver. | The external loop endpoints are direct. The ROM 0 logical convention is reconciled below; receiver threshold and waveform behavior remain open. |
| partial | PDF 29, drawing 2000325 | `RDR CONT` crosses CPU P4/J4 contact 89, J42/P42 contact 5, and J43/P43 contact 5. The nearby contact-6 path has a 68 ohm return to -10 V. | The forward conductor and return bias are source-visible. The terminal-strip relay contact path, asserted coil state, and reader mechanical pulse width remain untraced. |
| partial | PDFs 3 and 29, drawings 2000318 and 2000325; 98-095A PDF 152, printed page 136 | The terminal procedure defines a start bit as a current-absent space and directs software to read ROM 0 bit 0 as one during that start bit. Combined with the traced Q5 path, ROM 0 bit 0 is high when keyboard-loop current is absent. | The CPU-port logical sense is reconciled. The source does not establish every Q5 transistor threshold or switching waveform. |
| partial | PDFs 3 and 29, drawings 2000318 and 2000325; 98-095A PDF 152, printed page 136 | The terminal procedure defines every odd RAM 0 value as a marking line and every even value as a spacing line. Combined with the traced Q4 path, RAM 0 bit 0 high drives printer-loop marking current. | The CPU-port logical sense and serial mark/space convention are reconciled. Driver rise/fall and current thresholds remain open. |
| partial | PDFs 3 and 29, drawings 2000318 and 2000325; 98-095A PDF 152, printed page 136 | The terminal procedure defines every odd RAM 1 value as reader enable and every even value as reader disable. Combined with the traced Q3 path, RAM 1 bit 0 high enables the reader relay command. | The CPU-port logical sense is reconciled. Relay contact state, mechanical delay, and reader timing remain open. |
| partial | 98-095A PDF pages 150 and 152, printed pages 134 and 136 | The ASR-33 protocol uses 11 symbol intervals per character, a 9.09 ms symbol interval, start-space, seven information intervals, one parity interval, and two marking stop/recovery intervals. | This defines monitor-terminal framing. It does not supply a measured individual terminal timing trace or the CPU-card clock phase. |
| direct | PDFs 7 and 13, drawings 2000319 and 2000329 | The controller exposes `S/S PB`, `STOP ACK`, `USER RESET`, `CPU RESET`, `MOD SEL 12` through `MOD SEL 15`, and `MOD ENABLE`; the panel contains STOP, RESET, MON, RAM, and PROM control circuitry for that signal family. | The two sheets establish the arbitration interface. They do not yet establish each control equation, interlock priority, or transition edge. |
| partial | PDF 13, drawing 2000329 | The STOP pushbutton S26 passes through A36 conditioning before leaving the local STOP PB region. | The local conditioned path is visible. The resulting controller state transition, active polarity, and timing are unextracted. |
| direct | PDF 13, drawing 2000329 | The `4002 RESET ENABLE` connector net enters panel switch S31, which visibly selects the `SYSTEM` and `CPU` mode paths. | The switch and mode boundary are visible. Reset priority, electrical assertion level, and controller consequences remain unextracted. |
| direct | PDF 13, drawing 2000329 | S26 presents two switch contacts to ground and feeds the parallel A36 7404 input-conditioning pair before the labeled `STOP PB` boundary. | The physical conditioning topology is direct. The subsequent controller input, asserted STOP level, priority against reset, and stop/restart transition edge remain untraced. |
| partial | PDF 3, drawing 2000318 | imm4-43 P1 contact 30 carries `STOP ACK` into an 8095 buffer and R14 network at the A11 4040 boundary. | The CPU-card path is direct to its local boundary. The CPU-pin direction, asserted level, and external connector continuation remain open. |
| direct | PDF 7, drawing 2000319 | imm4-72 P1 contact 73 carries `STOP ACK` through A15 7404 to J2 contact 9. | This establishes the control-card local inversion and panel-facing connector boundary. The motherboard or cable continuation remains untraced. |
| direct | PDF 13, drawing 2000329; 98-095A printed page 63 | Front-panel J2 contact 9 receives `STOP ACK` and enters A29 7417. The manual identifies this signal as arriving from the memory-control module and states that the buffer output lights the RUN indicator when the system runs. | This establishes the panel-local receiver and RUN-indicator function. The connector mapping from the memory-control card, electrical assertion level, and relationship to the separate CPU-card STOP ACK contact remain open. |
| direct | PDF 13, drawing 2000329 | S31 places the J2 `4002 RESET ENABLE` line at a `SYSTEM` or `CPU` switch contact before the labeled `MODE` boundary. | The physical mode-selection boundary is direct. The source does not establish a complete panel-to-controller memory-store or reset arbitration equation. |
| direct | MCS-40 Advance Specifications printed page 37; 4040 data sheet pin description | The 4040 enters STOP on high `STP`. The single-step sequence says `STPA` returns low when execution leaves STOP, so its open-drain output is released high while the CPU is stopped. | This binds the 4040 package endpoint only. It does not assign a level to a board card edge, panel path, or rear connector. |
| conflict | 98-095A printed pages 15 and 129 | The CPU description says `STOP ACKNOWLEDGE` is low while the CPU is halted. The external-interface description says its `STOP ACKNOWLEDGE` status line is high while the processor is halted and says `STOP` is driven by a temporary low clamp. | The source-bound 4040 endpoint differs from at least one manual sentence. The CPU-to-control-to-panel-to-rear path remains untraced, so no board STOP/ACK polarity or panel transition enters the model. |
| negative boundary | PDFs 32 and 34, drawings 4000168 and 4000242 | The flat-cable assembly specifies conductor counts, connector families, pin-one orientation, and cable lengths. The cable-routing drawing shows physical harness placement. | Neither drawing assigns conductors to board contacts. They cannot establish the missing CPU, control-card, and panel `STOP ACK` continuation. |

## Newly traced program-RAM boundary facts

| Status | Source location | Record | Evidence and limit |
| --- | --- | --- | --- |
| direct | PDFs 5, 7, and 10 | The controller P1 `MAD0` through `MAD9` contacts 11 through 20 cross the motherboard memory-controller to IN-28 boundary and enter IN-28 P1 contacts 11 through 20. | Each low address conductor reaches an IN-28 3404 data-latch input. The latch enable phase and the stored address value require cycle extraction. |
| direct | PDFs 5 and 10 | The motherboard maps controller-side C3 at contact 94 to IN-28 `MAD11` at contact 94, and controller-side C2 at contact 96 to IN-28 `MAD10` at contact 96. | Each high address conductor then crosses a 7404 inversion stage before bank-selection logic. The source-visible connector route is complete; the Boolean bank-select equation remains open. |
| direct | PDFs 5, 7, and 10 | `BYTE2`, `BYTE1`, `MODULE SELECT`, and `WRITE/READ` use the IN-28 contacts 90, 92, 93, and 95; the controller sheet identifies `BYTE2`, `BYTE1`, and `WRITE` at the matching program-memory boundary. | The named card-edge control paths are source-backed. The signals still require a complete local TTL and 2102 control-pin trace before the model drives a source-faithful cycle. |
| partial | 98-095A, printed page 24; PDFs 5, 7, and 10 | The imm6-28 uses normal program-fetch address phases A1 through A3 and returns the requested byte during M1 and M2. Special instructions use execution phases for program-RAM reads and writes. `PM`, `F/L`, memory address, and memory-data-in form the stated external interface. | The manual establishes phase roles, not individual card-edge polarity or setup/hold timing. The schematic trace remains controlling evidence for the implementation. |
| partial | 98-095A, printed pages 40, 47, and 52; PDFs 5, 7, and 10 | The imm6-28 `WRITE/READ` card input writes on TTL low and reads on TTL high. For WPM, `PM` coincides with X2, the 4289 F/L state selects `BYTE2` or `BYTE1`, and the controller presents address, write data, byte selection, momentary `MOD ENABLE`, and the low write command together. | The manual defines input polarity and transaction coincidence. It does not trace the controller output through every motherboard, IN-28 3404, TTL, and 2102 control pin, nor define setup, hold, pulse width, or propagation delay. |

## Reconciled controller-to-IN-28 route matrix

| Route class | PDF 7 controller boundary | PDF 5 motherboard identity | PDF 10 IN-28 endpoint | Result |
| --- | --- | --- | --- | --- |
| Low address | P1 contacts 11 through 20 carry `MAD0` through `MAD9`. | The same labeled conductors cross the memory-controller and RAM-module boundary. | P1 contacts 11 through 20 enter the 3404 address-latch bank. | Physical endpoint continuity is direct. The latch-enable phase remains open. |
| High address | C3 is at P1 contact 94 and C2 is at P1 contact 96. | The conductors retain contacts 94 and 96. | Contact 94 is `MAD11`; contact 96 is `MAD10`. Each enters a visible 7404 stage before bank-selection logic. | Physical continuity and one visible inverter stage per high bit are direct. The bank-select Boolean equation remains open. |
| Byte controls | P1 contact 90 is `BYTE2`; contact 92 is `BYTE1`. | The contacts cross to the RAM-module boundary. | P1 contacts 90 and 92 enter 7404 and 7400-class local logic. | The route is direct. Assertion level and byte-cycle edge are partial. |
| Module selection | The controller boundary supplies the program-memory selection family. | The motherboard exposes the program-RAM module-select conductor at contact 93. | P1 contact 93 is `MODULE SELECT` and visibly enters a 3404 channel before later local logic. | The endpoint and first local stage are direct. The source equation and asserted polarity are partial. |
| Write/read | P1 contact 95 carries the controller `WRITE` command. | The conductor crosses unchanged to the RAM-module boundary. | P1 contact 95 is the imm6-28 `WRITE/READ` input and visibly enters a separate 3404 channel before later local logic. | The card input is low for write and high for read. The complete active-low 2102 R/W pulse and its setup/hold timing remain partial. |

## Consequences for implementation

The `Imm6_28` model preserves the thirty-two physical SRAM locations and
unknown power-on state. It does not claim that the present behavioral board has
the documented controller timing. `Imm472` remains a source gate until this
ledger contains a complete, polarity-checked map for every program-memory
cycle path.

`mcs4-intellec/src/mod40_routes.rs` mirrors the reviewed card-edge and terminal
records as typed route data. It also records the direct oscillator-to-divider
route and A8 decode inputs. Its direct entries preserve only complete visible
endpoint pairs. Its partial entries intentionally keep the CPU, monitor, and
program-RAM execution gates open until the local decode, inversion, polarity,
and timing stages are traced.

The following execution conditions remain open:

1. Bind two independent raw C1702A read sets to A1 through A4 with complete
   reader, custody, photo, voltage, log, and digest records.
2. Extract the A18 output-to-CSO map, C1702A data paths, 74158/8095/TTL
   polarity, divider output, 4040 phase, and reset-release timing from PDF 3.
3. Reconcile each PDF 5 card-edge contact with PDF 7 and PDF 10 endpoints,
   then derive the 3404 latch, 7404, bank-select, and active-low 2102 write
   equations with setup and hold timing.
4. Extract panel STOP, reset, step, MON/RAM/PROM, and interlock transition
   logic from PDF 13, and derive Q3/Q4/Q5 current-loop logical polarity,
   reader relay state, framing, and timing from PDFs 3 and 29.
5. Compare the accepted raw read sets before applying only the resulting
   primary-backed transform, then capture a source-tagged reset-to-prompt
   trace.
6. Enable a MOD 40 FPGA wrapper or equivalence trace only after all five
   preceding conditions close.
