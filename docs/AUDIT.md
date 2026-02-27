# Repository Audit

See also: `docs/ACCURACY_PROGRAM.md` for the full accuracy roadmap and evidence-to-simulation pipeline.

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
| 4004: 750 kHz; 10.8 usec cycle | docs/MCS-40/MCS4_Data_Sheet_Nov71.pdf ([bitsavers][bitsavers-mcs4-data-sheet]) | Verified (primary) | OCR: docs/evidence/ocr/mcs4_data_sheet_nov71.txt (p4). |
| Intel 4004 clock period 1.35-2.0 usec | docs/4004/intel-4004-datasheet.pdf ([chipdb mirror][chipdb-4004-datasheet]) | Verified (primary, OCR) | OCR sidecar in docs/evidence/ocr_results.md. |
| Intel 4004 instruction cycle 10.8 microseconds | docs/4004/intel-4004-datasheet.pdf | Verified (primary, OCR) | Page 1 OCR: "10.8 Microsecond Instruction Cycle". Local scan; online source TBD. |
| Intel 4004 uses 10 um process | [Wikipedia][wiki-4004] | Verified (secondary) | Primary datasheet text does not list this value. |
| Intel 4004 contains ~2,300 transistors | [Wikipedia][wiki-4004] | Verified (secondary) | Not found in OCR of primary datasheets; 4004.com netlist counts differ (see below). |
| Intel 4004 executes ~92,000 IPS | [Wikipedia][wiki-4004] | Verified (secondary) | Primary confirmation pending. |
| 4040 clock: 1.35-2.0 usec period (derived 0.5-0.74 MHz) | docs/4040/4040-datasheet.pdf ([chipdb mirror][chipdb-4040-datasheet]) | Derived (primary) | OCR: docs/evidence/ocr_results.md. |
| Intel 4040 max clock 500-740 kHz | [Wikipedia][wiki-4040] | Verified (secondary) | Matches clock-period-derived range; primary explicit spec pending. |
| 4040: 10.8 usec instruction cycle | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf ([bitsavers][bitsavers-mcs40-users-manual]) | Verified (primary) | Pages 7 and 100. |
| Intel 4040 adds 14 instructions for 60 total | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf | Verified (primary) | Page 30: 14 new instructions, 60 total. |
| Intel 4004 has 46 instructions | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf | Verified (primary) | Page 7: 46 instructions. |
| Intellec 4/MOD 40: 5.185 MHz system clock | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf ([bitsavers][bitsavers-mcs40-users-manual]) | Verified (primary) | Page 100 (system clock, not CPU max). |
| MCS-40 system clock period 1.35-2.0 usec | docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf ([bitsavers][bitsavers-mcs40-users-manual]) | Verified (primary) | Page 103 (system characteristics). |
| Intel 4040 contains ~3,000 transistors | [Wikipedia][wiki-4040] | Verified (secondary) | Not found in OCR of primary datasheets or 1975 Intel Data Catalog. |
| Intel 4040 executes ~62,000 IPS | [Wikipedia][wiki-4040] | Verified (secondary) | Primary confirmation pending. |
| 2025 discrete-transistor MCS-4 system at ~2x original clock | [4004.com][site-4004] | Verified (secondary) | Nov 2025 update cites ~1.5 MHz and "÷8" instruction rate; primary confirmation pending. |
| 4004 analyzer netlist counts (readme) | [4004.com analyzer repack ZIP][i400x-analyzer-zip] | Verified (secondary, forensic) | OCR: docs/evidence/ocr_results.md. |
| eframe Linux dependencies list (xcb/xkbcommon/ssl) | [eframe README][eframe-readme] | Verified | Used in INSTALLATION.md. |

## Immediate Discrepancies
- README now aligns with primary instruction-cycle timing; CPU max clock for 4040 remains derived from system clock spec (not explicitly stated).

## Next Verification Targets
- MCS-4/MCS-40 primary datasheets, Intel reliability reports, or data books that explicitly list 4004/4040 transistor counts.
- 4040 CPU max clock rate explicitly stated in a primary datasheet or hardware spec (not just derived from clock period).
- Memory addressing claims (ROM/RAM limits) vs primary manuals.
- 2025 discrete-transistor implementation claim: seek a primary publication or build log.

## Config Gaps
- Transistor counts for 4004/4040 not found in OCR of MCS-4 Data Sheet, MCS-4 Users Manual, MCS-40 Users Manual, 4040 datasheets, or chunked 1975 Intel Data Catalog OCR.
- 4040 primary docs confirm instruction cycle timing and clock period only; CPU max clock rate still derived, not explicitly stated.
- OCR output for some MCS-4/MCS-40 scans is noisy; re-run with higher quality scans if available.
- OCR of the 1975 Intel Data Catalog succeeded only via chunked runs; Ghostscript/tesseract warnings remain.

## OCR Coverage (2026-01)
- `docs/4004/intel-4004-datasheet.pdf`: clock period confirmed via OCR sidecar in docs/evidence/ocr/4004-datasheet.txt.
- `docs/4040/4040-datasheet.pdf`: clock period confirmed via OCR sidecar in docs/evidence/ocr/4040-datasheet.txt.
- `docs/MCS-4/MCS-4_UsersManual_Feb73.pdf`: no explicit transistor count found; see docs/evidence/ocr/mcs4_users_manual.txt.
- `docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf`: no explicit transistor count found; see docs/evidence/ocr/mcs40_users_manual.txt.
- `docs/MCS-40/MCS-40_Advance_Specifications_Sep74.pdf`: no explicit transistor count found; see docs/evidence/ocr/mcs40_advance_specs.txt.
- `docs/MCS-40/1975_Intel_Data_Catalog.pdf`: chunked OCR in docs/evidence/ocr/1975_catalog_mcs40_232-252.txt, 1975_catalog_mcs40_276-282.txt, 1975_catalog_mcs4_302.txt; no transistor counts found.
- `docs/MCS-40/MCS4_Data_Sheet_Nov71.pdf`: clock/750 KHz evidence in docs/evidence/ocr/mcs4_data_sheet_nov71.txt.

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
- 1975 Intel Data Catalog chunked OCR captured Intellec 4/MOD 4 and 4/MOD 40 memory specs; no transistor counts found.
- Remaining primary gaps: 4004/4040 transistor counts and 4040 max clock remain unverified in primary sources.

[bitsavers-mcs4-data-sheet]: http://bitsavers.org/components/intel/MCS4/MCS4_Data_Sheet_Nov71.pdf
[bitsavers-mcs40-users-manual]: http://bitsavers.org/components/intel/MCS40/MCS-40_Users_Manual_Nov74.pdf
[chipdb-4004-datasheet]: https://datasheets.chipdb.org/Intel/MCS-4/datashts/intel-4004.pdf
[chipdb-4040-datasheet]: https://datasheets.chipdb.org/Intel/MCS-40/4040.pdf
[eframe-readme]: https://github.com/emilk/egui/blob/master/crates/eframe/README.md
[i400x-analyzer-zip]: https://www.4004.com/assets/i400x_analyzer_repacked_20221111.zip
[site-4004]: https://www.4004.com/
[wiki-4004]: https://en.wikipedia.org/wiki/Intel_4004
[wiki-4040]: https://en.wikipedia.org/wiki/Intel_4040
