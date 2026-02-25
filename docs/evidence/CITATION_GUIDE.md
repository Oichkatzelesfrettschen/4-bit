# Citation Conventions for MCS-4 Research

This document establishes citation conventions for all research documents
in `docs/evidence/`. Follow these conventions when adding new sources or
cross-referencing existing parameters.

## BibTeX Key Naming

All sources are cataloged in `bibliography.bib`. Keys use the format:

    <first_author_surname_or_org><year><keyword>

Examples:
- `faggin1970sgt` -- Faggin & Klein, 1970, silicon gate technology
- `intel4004ds` -- Intel Corporation, 4004 datasheet
- `osburn2002tox` -- Osburn & Huff, 2002, gate oxide thickness
- `wp_intel4004` -- Wikipedia, Intel 4004

For institutional authors (Intel, Wikipedia), use a short identifier
instead of a surname.

## Required Fields by Source Type

### Primary sources (datasheets, manuals, specifications)
- author, title, year, howpublished
- `note` field describing relevance to this project
- `local_file` field if OCR'd text is available locally

### Secondary sources (papers, retrospectives, web)
- author, title, year, journal/booktitle/url
- `doi` if available
- `note` field describing relevance and limitations

### Textbooks
- author, title, publisher, year, chapter
- `note` field describing which values are cited and any caveats

## Markdown Citation Tags

In process_parameters and other research documents, use short bracket
tags that map to BibTeX keys:

| Tag    | BibTeX Key           | Source Type |
|--------|----------------------|-------------|
| [DS]   | intel4004ds          | Primary     |
| [MCS]  | intelmcs4ds1971      | Primary     |
| [FK]   | faggin1970sgt        | Primary     |
| [F15]  | faggin2015memoirs    | Secondary   |
| [WP]   | wp_intel4004         | Secondary   |

When adding a new source:
1. Create a BibTeX entry in `bibliography.bib`
2. Assign a short tag and add it to the citation table in the document
3. If the source has a local OCR file, add an entry to `source_manifest.json`
   with its SHA-256 checksum (computed via `sha256sum <file>`)

## Adding New OCR'd Sources

1. Place the OCR text file in `docs/evidence/ocr/`
2. Compute its checksum: `sha256sum docs/evidence/ocr/<filename>`
3. Add a BibTeX entry with `local_file` pointing to the OCR file
4. Add a JSON entry to `source_manifest.json` with:
   - `local_path`, `sha256`, `bytes`, `original_title`, `original_url`
   - `bibtex_key`, `ocr_tool`, `ocr_date`, `notes`

## Citation Requirements

Every claimed parameter in process_parameters must cite at least one
source. The confidence level reflects the source quality:

| Confidence | Meaning                                         |
|------------|--------------------------------------------------|
| Definite   | Value directly stated in a primary datasheet     |
| High       | Value from authoritative secondary source         |
| Estimated  | Value derived from physics, era norms, or partial |
|            | evidence; includes a Note section explaining why  |
