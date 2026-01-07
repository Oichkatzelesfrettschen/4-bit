# Repository Audit

## Scope
Initial audit of documentation claims, build hygiene, and configuration alignment.
This file is a living log. Each claim is marked as verified, contradicted, or pending
with a source URL.

## Evidence Trails
- docs/evidence/README.md
- docs/evidence/ocr_manifest.yaml
- docs/evidence/ocr_results.md

## Claims Verification (Phase 1)

| Claim | Source | Status | Notes |
| --- | --- | --- | --- |
| Intel 4004 clock: 750 kHz; 10.8 microsecond instruction cycle | docs/MCS-40/MCS4_Data_Sheet_Nov71.pdf ([bitsavers](http://bitsavers.org/components/intel/MCS4/MCS4_Data_Sheet_Nov71.pdf)) | Verified (primary, OCR) | Page 4 OCR: 10.8 usec cycle and 750 KHz clock. |
| Intel 4004 clock period 1.35-2.0 usec | docs/4004/intel-4004-datasheet.pdf (chipdb mirror: https://datasheets.chipdb.org/Intel/MCS-4/datashts/intel-4004.pdf) | Verified (primary, OCR) | See docs/evidence/ocr_results.md; OCR shows "Clock Period 1.35 2.0 usec". |
| Intel 4004 instruction cycle 10.8 microseconds | docs/4004/intel-4004-datasheet.pdf | Verified (primary, OCR) | Page 1 OCR: "10.8 Microsecond Instruction Cycle". Local scan; online source TBD. |
| Intel 4004 uses 10 um process | https://en.wikipedia.org/wiki/Intel_4004 | Verified (secondary) | Primary datasheet text does not list this value. |
| Intel 4004 contains ~2,300 transistors | https://en.wikipedia.org/wiki/Intel_4004 | Verified (secondary) | Not found in OCR of primary datasheets; 4004.com netlist counts differ (see below). |
| Intel 4040 max clock rate 500-740 kHz | docs/4040/4040-datasheet.pdf (chipdb mirror: https://datasheets.chipdb.org/Intel/MCS-40/4040.pdf) | Derived (primary clock period) | See docs/evidence/ocr_results.md; clock period 1.35-2.0 usec implies 0.5-0.74 MHz; CPU max clock not explicitly specified. |
| Intel 4040 instruction cycle standard 10.8 microseconds | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf ([bitsavers](http://bitsavers.org/components/intel/MCS40/MCS-40_Users_Manual_Nov74.pdf)) | Verified (primary) | Page 7 and 100: 10.8 microsecond cycle. |
| Intel 4040 adds 14 instructions for 60 total | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf | Verified (primary) | Page 30: 14 new instructions, 60 total. |
| Intel 4004 has 46 instructions | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf | Verified (primary) | Page 7: 46 instructions. |
| Intellec 4/MOD 40 system clock nominal 5.185 MHz | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf ([bitsavers](http://bitsavers.org/components/intel/MCS40/MCS-40_Users_Manual_Nov74.pdf)) | Verified (primary) | Page 100: system clock spec, not CPU max clock. |
| MCS-40 system clock period 1.35-2.0 usec (rise/fall 50 ns; width 380-480 ns) | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf ([bitsavers](http://bitsavers.org/components/intel/MCS40/MCS-40_Users_Manual_Nov74.pdf)) | Verified (primary) | Page 103: system characteristics table. |
| Intel 4040 contains ~3,000 transistors | https://en.wikipedia.org/wiki/Intel_4040 | Verified (secondary) | Not found in OCR of primary datasheets or 1975 Intel Data Catalog. |
| Intel 4040 executes ~62,000 IPS | https://en.wikipedia.org/wiki/Intel_4040 | Verified (secondary) | Primary confirmation pending. |
| 2025 discrete-transistor MCS-4 system at ~2x original clock | https://www.4004.com/ | Verified (secondary) | Homepage claim; primary confirmation pending. |
| 4004 netlist component counts (layout vs schematics) | https://www.4004.com/assets/i400x_analyzer_repacked_20221111.zip | Verified (secondary, forensic) | See docs/evidence/ocr_results.md; readme lists 1,807 transistors and 2,308 total components. |
| eframe Linux dependencies list (xcb/xkbcommon/ssl) | https://github.com/emilk/egui/blob/master/crates/eframe/README.md | Verified | Used in INSTALLATION.md. |

## Immediate Discrepancies
- README now aligns with primary instruction-cycle timing; CPU max clock for 4040 remains derived from system clock spec (not explicitly stated).

## Next Verification Targets
- MCS-4/MCS-40 primary datasheets, Intel reliability reports, or data books that explicitly list 4004/4040 transistor counts.
- 4040 CPU max clock rate explicitly stated in a primary datasheet or hardware spec (not just derived from clock period).
- Memory addressing claims (ROM/RAM limits) vs primary manuals.
- 2025 discrete-transistor implementation claim: seek a primary publication or build log.

## Config Gaps
- Transistor counts for 4004/4040 not found in OCR of MCS-4 Data Sheet, MCS-4 Users Manual, MCS-40 Users Manual, 4040 datasheets, or 1975 Intel Data Catalog.
- 4040 primary docs confirm instruction cycle timing and clock period only; CPU max clock rate still derived, not explicitly stated.
- OCR output for some MCS-4/MCS-40 scans is noisy; re-run with higher quality scans if available.

## OCR Coverage (2026-01)
- `docs/4004/intel-4004-datasheet.pdf`: clock period confirmed via OCR sidecar in docs/evidence/ocr/4004-datasheet.txt.
- `docs/4040/4040-datasheet.pdf`: clock period confirmed via OCR sidecar in docs/evidence/ocr/4040-datasheet.txt.
- `docs/MCS-4/MCS-4_UsersManual_Feb73.pdf`: no explicit transistor count found; see docs/evidence/ocr/mcs4_users_manual.txt.
- `docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf`: no explicit transistor count found; see docs/evidence/ocr/mcs40_users_manual.txt.
- `docs/MCS-40/MCS-40_Advance_Specifications_Sep74.pdf`: no explicit transistor count found; see docs/evidence/ocr/mcs40_advance_specs.txt.
- `docs/MCS-40/1975_Intel_Data_Catalog.pdf`: no explicit transistor count found.
- `docs/MCS-40/MCS4_Data_Sheet_Nov71.pdf`: OCR pending in evidence trail; prior notes retained.

## Diagram Coverage
- Diagram and layer artifacts are indexed in `docs/CHIP_ARTIFACTS.md`.
- MCS-4 chips have metal/poly/diffusion/via layers plus schematics via the i400x analyzer assets.
- MCS-4 photomicrograph thumbnails (4001-4004) are present under `docs/photomicrographs/`.
- 4040 has datasheets only; die shots and layer sets are not present in repo, and only package photos have been located so far.

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
