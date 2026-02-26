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

5. Generate OCR sidecars from downloaded PDFs:
   `ocrmypdf --skip-text --output-type pdf --sidecar <txt> <pdf> /tmp/<out>.pdf`

6. Extract evidence lines:
   Use `rg` and `sed -n` against the OCR sidecar text.

7. Update checksums:
   `sha256sum <file>` and update ocr_manifest.yaml / source_manifest.json.

Audit claim backlog
- Generate a tracking view of “derived/pending” claims called out in `docs/AUDIT.md`:
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
- Output (best-effort “transistor candidates”, not a complete netlist):
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
