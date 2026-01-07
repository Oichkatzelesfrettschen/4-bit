# Evidence Trails

Purpose: Record primary sources, OCR sidecars, hashes, and extraction commands for MCS-4/MCS-40 evidence.

Layout
- docs/evidence/ocr/ (sidecar text from ocrmypdf)
- docs/evidence/ocr_manifest.yaml (URLs, hashes, sidecars)
- docs/evidence/ocr_results.md (snippets, "not found" checks)

Reproduction workflow
- Fetch PDFs from Bitsavers and ChipDB per ocr_manifest.yaml.
- Run `ocrmypdf --force-ocr --sidecar <txt> <pdf> /tmp/<out>.pdf`.
- Use `rg` and `sed -n` to extract evidence lines.
- Hash sources with `sha256sum` and update ocr_manifest.yaml.

Netlist analyzer extraction
- Download https://www.4004.com/assets/i400x_analyzer_repacked_20221111.zip
- `unzip -o ... -d /tmp/i400x_analyzer_repacked`
- `7z x -p"Four-0-Zero-4 forever!" -o/tmp/i400x_analyzer ...`
- `unzip -o /tmp/i400x_analyzer/i400x_analyzer_20181115.zip -d /tmp/i400x_analyzer/unpacked`
- Use `/tmp/i400x_analyzer/unpacked/readme.txt` for component counts.

Warnings and limitations
- ocrmypdf warns about Ghostscript 10.6 JPEG issues and PDF/A size growth.
- OCR output is noisy; verify against the original PDF pages for final citations.
- OCR of docs/MCS-40/1975_Intel_Data_Catalog.pdf failed due to resource limits; use pdfplumber or chunked OCR if needed.
- Output PDFs are stored in /tmp and are not committed.
