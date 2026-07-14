# Evidence Trails

Purpose: Record primary sources, OCR sidecars, hashes, and extraction commands for MCS-4/MCS-40 evidence.

Layout
- `docs/evidence/ocr/` -- sidecar text from ocrmypdf
- `docs/evidence/ocr_manifest.yaml` -- PDF URLs, SHA-256 hashes, sidecar mappings
- `docs/evidence/source_manifest.json` -- OCR sidecar provenance (text files, checksums, bibtex keys)
- `docs/evidence/bibliography.bib` -- BibTeX entries for all primary and secondary sources
- `docs/evidence/CITATION_GUIDE.md` -- citation conventions and key table
- `docs/evidence/ocr_results.md` -- snippets, "not found" checks
- `docs/evidence/audit_claims_backlog.md` -- derived/pending claims from `docs/AUDIT.md`
- `docs/evidence/PRIMARY_SOURCES_BACKLOG.md` -- missing primary-source materials to prioritize
- `docs/evidence/url_reachability_audit.md` -- periodic URL reachability test results
- `docs/evidence/intellec_sources.yaml` -- Intellec and ASR-33 source ledger,
  local-only download provenance, hashes, and unresolved source gates
- `docs/evidence/INTELLEC_MOD40_PRIMARY_EVIDENCE.md` -- MOD 40 module,
  terminal, monitor, and source-gate facts extracted from retained manuals
- `docs/evidence/INTELLEC_MOD40_MON4_PUBLIC_ARTIFACT_AUDIT.md` -- public
  MON4 wrapper, archive, and derivative-distribution audit with explicit
  physical-read provenance limits
- `docs/evidence/INTELLEC_MOD40_OCR_STATUS.md` -- 98-013A OCR engine choices,
  capture limits, and the visual-verification boundary
- `docs/evidence/INTELLEC_MOD40_BOARD_NET_LEDGER.md` -- source-backed and open
  board-route records

Reproduction workflow
1. Download all primary source PDFs:
   `./scripts/fetch_sources.sh`
   This parses `ocr_manifest.yaml`, downloads each PDF to its `local_path`,
   and verifies the SHA-256 checksum. Idempotent: skips files already present
   with matching checksums.

2. Verify checksums only (no downloads):
   `./scripts/fetch_sources.sh --verify`

3. Test URL reachability (dry run, HEAD requests only):
   `./scripts/fetch_sources.sh --dry-run`

4. Full URL reachability audit (all project URLs, not just PDFs):
   `./scripts/fetch_sources_test.sh`

5. Fetch the local-only Intellec and terminal sources when their redistribution
   terms require a local cache:
   `./scripts/fetch_intellec_sources.sh`
   The script uses HTTPS, a fixed Mozilla user agent, a temporary download,
   and the hash recorded in `intellec_sources.yaml`.

6. Generate OCR sidecars from downloaded PDFs:
   `ocrmypdf --skip-text --output-type pdf --sidecar <txt> <pdf> /tmp/<out>.pdf`

7. Extract evidence lines:
   Use `rg` and `sed -n` against the OCR sidecar text.

8. Update checksums:
   `sha256sum <file>` and update ocr_manifest.yaml / source_manifest.json.

9. Render and OCR all retained Intellec 4 MOD 40 98-013A sheets:
   `./scripts/extract_mod40_schematic_ocr.sh --surya-python <python> --surya-pages 3,5,7,10,13,15,29`
   The command emits a reproducible local cache under
   `docs/evidence/ocr/mod40_98013_*/full-sheet*/`. It records engine versions,
   source and artifact digests, and timeout status. The cache is not tracked.

10. Build the page-scoped OCR candidate index:
    `./scripts/index_mod40_ocr_candidates.py --input <ocr-cache> --output <candidate-json>`
    The JSON is a review queue only. Confirm both endpoints and each polarity
    stage on the primary sheet before changing the board net ledger.

11. Compare declared four-device MON4 artifact sets without inferring a
    physical read, socket order, or polarity transform:
    `python3 scripts/compare_intellec_mod40_proms.py --set first=a,b,c,d --set second=e,f,g,h --format first=intel-hex-preamble --format second=hex-listing-preamble`
    The command requires a declared format for every set and rejects padding,
    truncation, sparse Intel HEX records, and address discontinuities.

Audit claim backlog
- Generate a tracking view of "derived/pending" claims called out in `docs/AUDIT.md`:
  - `python3 scripts/audit_claims_backlog.py`

Schematic OCR (circuit-level labels)
- Inputs: `docs/emulators/i400{1,2,3,4}-schematic.bmp` (source; `*.png` previews exist) (plus `i400{1,2,3,4}-signals.txt`).
- Command: `./scripts/ocr_schematics.py --all --scale 4 --psm 11`
- Outputs (tracked artifacts):
  - `docs/evidence/ocr_schematics/*_schematic.txt` (plain OCR text)
  - `docs/evidence/ocr_schematics/*_schematic.tsv` (word-level boxes/confidence)
  - `docs/evidence/ocr_schematics/*_schematic.meta.json` (input hashes + params)
  - `docs/evidence/ocr_schematics/manifest.json`

Coordinate-based label OCR verification (signals.txt)
- Inputs: `docs/emulators/i400x-schematic.bmp` (source; `i400x-schematic.png` previews exist) + `docs/emulators/i400{1,2,3,4}-signals.txt`
- Command (calibration example): `./scripts/ocr_signal_labels.py --chip 4004 --name-regex '^(CLK1|CLK2)$'`
- Outputs:
  - `docs/evidence/ocr_signal_labels/<chip>/*_signal_ocr_report.json` (detailed per-point results)
  - `docs/evidence/ocr_signal_labels/<chip>/*_signal_ocr_report.tsv` (summary for spreadsheets)
  - `docs/evidence/ocr_signal_labels/<chip>/crops/*.png` (annotated mismatch crops)
  - `docs/evidence/ocr_signal_labels/metrics.json` + `docs/evidence/ocr_signal_labels/metrics.md` (regression summary)
- Notes:
  - Some `i400{1,2,3,4}-signals.txt` entries are *net names*, while the schematic may show a *pin number* (e.g. `CLK1` vs printed `01`).
    These are handled via `scripts/ocr_signal_aliases.json`.
  - Report reasons distinguish "no nearby text blobs" vs "OCR empty" vs "likely not printed here" (`no_text_components`, `ocr_no_tokens`, `ocr_low_conf`, `not_printed_near_point`).

Transistor candidate extraction (poly/diffusion intersections)
- Inputs: `docs/emulators/i400{1,2,3,4}-{poly,diffusion}.bmp` (source; `*.png` previews exist)
- Command: `./scripts/extract_transistors.py --all`
- Output (best-effort "transistor candidates", not a complete netlist):
  - `docs/evidence/transistors/*_poly_diffusion_transistors.json` (components w/ bbox + centroid)
  - `docs/evidence/transistors/manifest.json`
  - `docs/evidence/transistors/metrics.json` + `docs/evidence/transistors/metrics.md` (regression summary)

Netlist analyzer extraction
- Download https://www.4004.com/assets/i400x_analyzer_repacked_20221111.zip
- `unzip -o ... -d /tmp/i400x_analyzer_repacked`
- `7z x -p"Four-0-Zero-4 forever!" -o/tmp/i400x_analyzer ...`
- `unzip -o /tmp/i400x_analyzer/i400x_analyzer_20181115.zip -d /tmp/i400x_analyzer/unpacked`
- Use `/tmp/i400x_analyzer/unpacked/readme.txt` for component counts.

Warnings and limitations
- ocrmypdf warns about Ghostscript 10.6 JPEG issues and PDF/A size growth.
- OCR output is noisy; verify against the original PDF pages for final citations.
- Chunked OCR of docs/MCS-40/1975_Intel_Data_Catalog.pdf completed; warnings remain for Ghostscript/tesseract and large output sizes.
- Output PDFs are stored in /tmp and are not committed.
