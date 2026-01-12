# Claims → Tests Traceability

This file ties documentation claims to either:
1) a **primary-source excerpt** (OCR sidecar path and/or upstream URL), and
2) a **repo test** or **deterministic extraction** that would fail if the claim becomes wrong.

It exists to prevent “documentation drift” as we move toward higher fidelity.

## Timing + Instruction-Cycle Claims

| Claim | Primary evidence | Enforcement |
| --- | --- | --- |
| 4004 instruction cycle is 10.8 µs | `docs/evidence/ocr/mcs4_data_sheet_nov71.txt` | Emulator fixtures + phase-accurate bus tests (`cargo test`) |
| 4040 has 60 instructions (14 new) | `docs/evidence/ocr/mcs40_users_manual.txt` | Decoder unit tests (`mcs4-chips`) |
| I/O ops asserted only in transfer phases | MCS-4/MCS-40 manuals (timing sections) | `mcs4-system` timing tests (bus phase assertions) |

## Netlist + Transistor Evidence Claims

| Claim | Evidence | Enforcement |
| --- | --- | --- |
| `netlist_v0` deterministically stitches layout masks | `docs/NETLIST_WORKFLOW.md` + `docs/evidence/netlists_v0/manifest.json` | `mcs4-core` test loads committed `4004_netlist_v0.json` |
| `poly ∩ diffusion` output is “candidates”, not netlist | `docs/evidence/DIAGRAM_EXTRACTION.md` | `scripts/validate_analyzer_claims.py` + metrics artifacts |
| `netlist_v1` is a traceable bridge (anchors + schematic traces) | `docs/NETLIST_V1_SCHEMA.md` + `docs/evidence/netlists_v1/4004_netlist_v1.json` | Local regen scripts (`scripts/emit_netlist_v1_draft.py`, `scripts/extract_schematic_connectivity_v0.py`) |

## OCR Claims

| Claim | Evidence | Enforcement |
| --- | --- | --- |
| OCR of tiny edge labels is benchmarked + stable | `docs/evidence/ocr_benchmarks_v0/` | `python3 scripts/ocr_benchmark_v0.py --bench ...` |
| Periphery edge label OCR is bounded/fast by default | `scripts/detect_layout_edge_labels_v0.py` | Manual perf budget (recorded runs), future CI budget candidate |

## Primary-Source Registry Discipline

| Claim | Evidence | Enforcement |
| --- | --- | --- |
| Imported photomicrographs are provenance-tracked | `docs/evidence/photomicrograph_permissions.md` | Manual check + SHA256 entries required for new imports |
| OCR’d PDFs are registered | `docs/evidence/ocr_manifest.yaml` + `docs/evidence/ocr_results.md` | `bash scripts/doc_validate.sh` (registry + docs) |

## Primary-Source Gaps (Must Stay Explicit)

These are intentionally *not enforced* yet; they must remain marked pending until proven:
- primary transistor counts (4004/4040),
- 4040 explicit max clock quote,
- 4040 die/layer imagery provenance.

Tracked in `docs/evidence/PRIMARY_SOURCES_BACKLOG.md` and `docs/AUDIT.md`.
