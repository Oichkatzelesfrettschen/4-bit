# Repository Audit

## Scope
Initial audit of documentation claims, build hygiene, and configuration alignment.
This file is a living log. Each claim is marked as verified, contradicted, or pending
with a source URL.

## Claims Verification (Phase 1)

| Claim | Source | Status | Notes |
| --- | --- | --- | --- |
| Intel 4004 clock: 750 kHz; 10.8 microsecond instruction cycle | docs/MCS-40/MCS4_Data_Sheet_Nov71.pdf ([bitsavers](http://bitsavers.org/components/intel/MCS4/MCS4_Data_Sheet_Nov71.pdf)) | Verified (primary, OCR) | Page 4 OCR: 10.8 usec cycle and 750 KHz clock. |
| Intel 4004 clock period 1.35-2.0 usec | docs/4004/intel-4004-datasheet.pdf | Verified (primary, OCR) | Page 8 OCR: "Clock Period 1.35 2.0 usec". Local scan; online source TBD. |
| Intel 4004 instruction cycle 10.8 microseconds | docs/4004/intel-4004-datasheet.pdf | Verified (primary, OCR) | Page 1 OCR: "10.8 Microsecond Instruction Cycle". Local scan; online source TBD. |
| Intel 4004 uses 10 um process | https://en.wikipedia.org/wiki/Intel_4004 | Verified (secondary) | Primary datasheet text does not list this value. |
| Intel 4004 contains ~2,300 transistors | https://en.wikipedia.org/wiki/Intel_4004 | Verified (secondary) | Not found in OCR of primary datasheets. |
| Intel 4040 max clock rate 500-740 kHz | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf ([bitsavers](http://bitsavers.org/components/intel/MCS40/MCS-40_Users_Manual_Nov74.pdf)) | Derived (primary system clock) | Page 103: 1.35-2.0 usec system clock period implies 0.5-0.74 MHz; CPU max clock not explicitly specified. |
| Intel 4040 instruction cycle standard 10.8 microseconds | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf ([bitsavers](http://bitsavers.org/components/intel/MCS40/MCS-40_Users_Manual_Nov74.pdf)) | Verified (primary) | Page 7 and 100: 10.8 microsecond cycle. |
| Intel 4040 adds 14 instructions for 60 total | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf | Verified (primary) | Page 30: 14 new instructions, 60 total. |
| Intel 4004 has 46 instructions | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf | Verified (primary) | Page 7: 46 instructions. |
| Intellec 4/MOD 40 system clock nominal 5.185 MHz | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf ([bitsavers](http://bitsavers.org/components/intel/MCS40/MCS-40_Users_Manual_Nov74.pdf)) | Verified (primary) | Page 100: system clock spec, not CPU max clock. |
| MCS-40 system clock period 1.35-2.0 usec (rise/fall 50 ns; width 380-480 ns) | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf ([bitsavers](http://bitsavers.org/components/intel/MCS40/MCS-40_Users_Manual_Nov74.pdf)) | Verified (primary) | Page 103: system characteristics table. |
| Intel 4040 contains ~3,000 transistors | https://en.wikipedia.org/wiki/Intel_4040 | Verified (secondary) | Not found in OCR of primary datasheets or 1975 Intel Data Catalog. |
| Intel 4040 executes ~62,000 IPS | https://en.wikipedia.org/wiki/Intel_4040 | Verified (secondary) | Primary confirmation pending. |
| 2025 discrete-transistor MCS-4 system at ~2x original clock | https://www.4004.com/ | Verified (secondary) | Homepage claim; primary confirmation pending. |
| eframe Linux dependencies list (xcb/xkbcommon/ssl) | https://github.com/emilk/egui/blob/master/crates/eframe/README.md | Verified | Used in INSTALLATION.md. |

## Immediate Discrepancies
- README now aligns with primary instruction-cycle timing; CPU max clock for 4040 remains derived from system clock spec (not explicitly stated).

## Next Verification Targets
- MCS-4/MCS-40 primary datasheets or catalogs for 4004/4040 transistor counts (not found in scanned PDFs).
- 4040 CPU max clock rate explicitly stated in a primary datasheet or hardware spec.
- Memory addressing claims (ROM/RAM limits) vs primary manuals.
- 2025 discrete-transistor implementation claim: seek a primary publication or build log.

## Config Gaps
- Transistor counts for 4004/4040 not found in OCR of MCS-4 Data Sheet, MCS-40 Users Manual, 4040 datasheets, or 1975 Intel Data Catalog.
- 4040 primary docs confirm instruction cycle timing and system clock period only; CPU max clock rate not explicitly stated.
- OCR output for MCS-4 datasheets is noisy; re-run with higher quality scans if available.

## OCR Coverage (2026-01)
- `docs/4004/intel-4004-datasheet.pdf`: instruction timing confirmed; no transistor count found.
- `docs/MCS-40/MCS4_Data_Sheet_Nov71.pdf`: 750 kHz clock and 10.8 usec cycle confirmed.
- `docs/MCS-40/4040_Datasheet.pdf` + `docs/4040/4040-datasheet.pdf`: instruction cycle wording only; no explicit max clock or transistor count.
- `docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf`: system clock period and instruction cycle confirmed; no transistor count.
- `docs/MCS-40/MCS-40_Advance_Specifications_Sep74.pdf`: no explicit transistor count found.
- `docs/MCS-40/1975_Intel_Data_Catalog.pdf`: no explicit transistor count found.

## Chip Design Verification (Primary Sources)

### MCS-4 (4001/4002/4003/4004)

| Chip | Design/Function | Source | Notes |
| --- | --- | --- | --- |
| 4001 | 256 x 8 bit mask programmable ROM | docs/MCS-40/MCS4_Data_Sheet_Nov71.pdf | Page 2 OCR: "4001 256 x 8 BIT MASK PROGRAMMABLE". |
| 4002 | 320 bit RAM and 4 bit output port | docs/MCS-40/MCS4_Data_Sheet_Nov71.pdf | Page 2 OCR: "4002 320 BIT RAM AND 4 BIT OUTPUT PORT". |
| 4003 | 10 bit serial-in/parallel-out shift register | docs/MCS-40/MCS4_Data_Sheet_Nov71.pdf | Page 3 OCR: "4003 10 BIT SERIAL-IN/PARALLEL-OUT". |
| 4004 | 4 bit central processor unit | docs/MCS-40/MCS4_Data_Sheet_Nov71.pdf | Page 3 OCR: "4004 4 BIT CENTRAL PROCESSOR UNIT". |

### MCS-40 (4040/4101/4201/4289/4308)

| Chip | Design/Function | Source | Notes |
| --- | --- | --- | --- |
| 4040 | 4-bit parallel CPU | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf | Page 2: "4040 4-BIT PARALLEL CPU". |
| 4101 | 1K static RAM (256x4) | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf | Page 2: "4101 1K STATIC RAM (256x4)". |
| 4201 | System clock / clock generator | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf | Pages 2 and 8: "4201 SYSTEM CLOCK" and "4201 - Clock Generator". |
| 4289 | Standard memory interface | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf | Page 8: "4289 - Standard Memory Interface". |
| 4308 | Mask programmable ROM with I/O | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf | Page 8: "4308 - Mask Programmable ROM". |

## Re-Audit Summary (2026-01-06)
- Confirmed instruction counts (4004: 46, 4040: 60) and 10.8 microsecond instruction cycle via MCS-40 Users Manual.
- Confirmed 4004 clock at 750 kHz and 10.8 usec instruction cycle via MCS4 Data Sheet (OCR).
- Confirmed 5.185 MHz system clock for Intellec 4/MOD 40 (system spec, not CPU max clock).
- Remaining primary gaps: 4004/4040 transistor counts and 4040 max clock remain unverified in primary sources.
