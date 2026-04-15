# Audit claims backlog

Generated from `docs/AUDIT.md` by `scripts/audit_claims_backlog.py`.

## Backlog

| Claim | Status | Notes |
| --- | --- | --- |
| Intel 4004 executes ~92,000 IPS | Verified (secondary) | Primary confirmation pending. |
| 4040 clock: 1.35-2.0 usec period (derived 0.5-0.74 MHz) | Derived (primary) | OCR: docs/evidence/ocr_results.md. |
| Intel 4040 max clock 500-740 kHz | Verified (secondary) | Matches clock-period-derived range; primary explicit spec pending. |
| Intel 4040 executes ~62,000 IPS | Verified (secondary) | Primary confirmation pending. |
| 2025 discrete-transistor MCS-4 system at ~2x original clock | Verified (secondary) | Nov 2025 update cites ~1.5 MHz and "÷8" instruction rate; primary confirmation pending. |

## Next verification targets (from AUDIT)

- MCS-4/MCS-40 primary datasheets, Intel reliability reports, or data books that explicitly list 4004/4040 transistor counts.
- 4040 CPU max clock rate explicitly stated in a primary datasheet or hardware spec (not just derived from clock period).
- Memory addressing claims (ROM/RAM limits) vs primary manuals.
- 2025 discrete-transistor implementation claim: seek a primary publication or build log.
- Note: 1976 and 1978 Intel data catalogs were scanned locally on 2026-04-06; they did not close the transistor-count/max-clock gaps.
