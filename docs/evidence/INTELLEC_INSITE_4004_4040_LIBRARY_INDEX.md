# Intel Insite 4004 and 4040 Library Extraction Index

## Scope and reproducibility

This index classifies the complete 369-page Intel Insite 4004/4040 program
library scan retained under the local-only source ID
`intel-insite-4004-4040-library-hale`. It extracts only records that can
inform Intellec 4/MOD 40 historical investigation. It does not promote a
user-library program, a terminal conversion, or a generic MCS-4/MCS-40
example into evidence for a standard MOD 40 board route.

The downloaded ZIP has SHA-256
`9f4a4f0873f7b2b7d9f1638be6373065f838160a5138e0be750d4ffd286f23a7`.
It contains `Intel.pdf`, 36,672,145 bytes, SHA-256
`f83ede05f3bd106b695cece508befabf2f68116c21cec296552bb2a3cd98e10e`.
The embedded library index has SHA-256
`803fe43f5cca08921678115a3b42e8562084c63c4d847c12f41e0aefa9e9215e`.

The ignored cache renders every PDF page at 120 DPI JPEG quality 95 and runs
Tesseract with PSM 11. The first transient launcher stops after page 322
without a recognition failure. A persistent `tmux` continuation completes
pages 323 through 369 with the same command and parameters. The result
contains 369 OCR text files. Pages 2, 82, 307, and 308 have empty OCR output;
the capture records them as blank or non-text pages rather than inserting
invented text. Tesseract diagnostics on dense pages remain diagnostics, not
recognition failures.

## Evidence classification

| Tier | Meaning | Use boundary |
| --- | --- | --- |
| P2 | Archival scan of a contemporaneous Intel user-library submission. | Establishes what the submitted program or conversion note says. |
| P3 | Generic program, sample driver, or claimed hardware configuration inside the P2 scan. | Provides a software or interface comparison only. |
| P1 | Retained Intel manual or 98-013A schematic. | Controls standard MOD 40 card population, routes, polarity, timing, and monitor-media claims. |

## Page-level target index

| Scan pages | Item | Direct finding | Tier | Gate relevance and boundary |
| --- | --- | --- | --- | --- |
| 67-81 | 40-5, `Intellec 4/MOD 40-Silent 700 Interface` | The note describes a current-loop to RS-232C adapter, a Silent 700 Model 733ASR conversion, and terminal-specific Intellec modifications. | P2 | G4 comparison only. It does not describe the baseline MOD 40 terminal path. |
| 71 | 40-5 hardware modification | The conversion grounds J43 pin 8, reverses J43 pins 3 and 4, and bypasses a 220-ohm motherboard resistor. | P2 | This proves a conversion can alter host wiring. It prohibits treating a converted terminal demonstration as baseline-polarity proof. |
| 72-81 | 40-5 software conversion | The note directs loading a modified monitor into program RAM and programming four replacement monitor EPROMs. The scan reads `4702A`. | P2 | This describes a modified 4702A-compatible development profile. Standard MOD 40 population remains four 1702A devices under 98-013A and 98-095A. The scan cannot establish the replacement part in a baseline machine, socket order, or byte transform. |
| 83-87 | 40-6, `PDUP: PROM Dump Utility Program` | A 77-hex-word utility prints front-panel PROM address-content changes. The listing says it was assembled and tested on an Intellec 4 MOD 40. | P2 | G1 tool-behavior lead only. Change-only printout is not a 256-byte raw read, socket record, reader log, or accepted monitor set. |
| 110-111 | 40-8, MCS-4 or 4/40 disassembler | The library contains a disassembler intended for code in ROM, PROM, RAM, or paper-tape input. | P3 | G5 software-analysis lead only. It does not authenticate monitor bytes. |
| 207 | 40-11, `HEXBCD` | The form marks 4040 and names an Intellec 4/40 V3.0 assembler environment; the routine itself is hardware-neutral. | P3 | No route or media-gate effect. |
| 310-318 | 4-18, General Purpose ROM | The item claims 1702A programming and reading, but its required platform is SIM4 or Intellec 4, not MOD 40. | P3 | A generic 1702A tool lead only. It does not document a MOD 40 monitor acquisition. |
| 334-339 | 40-13, selector subroutine | The form marks 4040 and describes ROM-port selection behavior. | P3 | Generic MCS-40 I/O behavior only; no standard MOD 40 card or net assertion. |
| 340-345 | 40-14, 5-level teletype to 8-level ASCII conversion | The form identifies a 4004 configuration with TTY ports 0 and 1 despite the `40-14` library identifier. | P2 | The library prefix does not itself prove a 4040 target. This is paper-tape format evidence only. |
| 346-364 | 40-15, Conway Life | The form marks 4040 and describes serial TTY input on ROM input port 0 bit 0, output on RAM output port 0 bit 0, and a 50 kHz bit-timing clock. | P3 | G4 software-level comparison only. It is not a standard MOD 40 port map or electrical trace. |
| 362-363 | 40-15 sample TTY driver | The driver samples with `RDR`, applies `CMA` because the input is inverted, and emits serial output through RAM port 0. | P3 | Records a software inversion in this example. It cannot be composed with J43 or Q3/Q4/Q5 without the primary board route and known terminal conversion state. |
| 367-369 | 4-12, `KBD` Touch-Tone scanner | The item uses an I4211 as a generic GPI/O example. | P3 | It does not place I4211 in a standard MOD 40. |

## Reconciled observations

The corpus exposes two distinct inversion layers. Item 40-5 changes the
physical terminal interface, while the 40-15 sample driver inverts a received
software bit. These facts are compatible with several electrical arrangements.
They do not form a valid end-to-end polarity equation because the program,
adapter, terminal, and baseline board configurations are not proven identical.

Item 40-5 specifies its own modified monitor conversion and names `4702A`.
The Intel MCS-40 Users Manual identifies 4702A as a 4289-compatible,
electrically reprogrammable 256 by 8 PROM for development use. The standard
MOD 40 record remains four 1702A monitor PROMs under 98-013A and 98-095A.
These sources describe different configurations, not a reason to replace the
standard board population. The conversion note still cannot establish a
baseline socket order, data transform, or monitor byte set.

The `40-14` library identifier appears alongside a form marked for 4004, while
other 40-prefixed forms explicitly mark 4040. Therefore a library reference
prefix is not a sufficient CPU-family classification rule.

## Closed and open boundaries

The corpus closes its own extraction boundary: all 369 rendered pages have an
OCR result or an explicit empty-output record. It does not close a MOD 40
historical execution gate. The remaining governing work is unchanged:

1. Trace the primary 98-013A CPU, controller, motherboard, IN-28, panel, and
   TTY nets with both endpoints, every inverter, and timing context.
2. Obtain two independently documented, repeatable raw C1702A reads for every
   socket-identified monitor device.
3. Reconcile raw bytes, socket order, data transform, terminal electrical
   polarity, and program-RAM write timing before enabling a historical trace,
   FPGA wrapper, or equivalence check.

`INTELLEC_MOD40_PUBLIC_DISCOVERY_LOG.md` records the public-source search
boundary. `INTELLEC_MOD40_EXECUTION_GATE_STATUS.md` remains the authority for
gate state.
