# Provenance & Reuse Checklist (Images / PDFs)

Use this checklist before adding external assets (die shots, masks, datasheets, scans) to the repo.

## Before importing an asset
- Identify the upstream page that publishes the file (not just an aggregator mirror).
- Confirm reuse permission on that page:
  - Acceptable for vendoring: `CC0`, `CC BY`, `CC BY-SA`, public domain.
  - Avoid vendoring (link only): unknown/no license, “all rights reserved”, unclear scope, or terms that prohibit redistribution.
  - If `BY-NC-SA` is the only option, vendor only if we explicitly need it and we keep it clearly separated + labeled as non-commercial.
- Capture evidence of permission:
  - Save the exact URL(s).
  - Copy the relevant license text snippet (HTML or page text) into `docs/evidence/photomicrograph_permissions.md`.
  - Record the retrieval date (UTC).

## When importing an asset
- Store under `docs/photomicrographs/<chip>/` (images) or `docs/evidence/<area>/` (PDFs, OCR sidecars).
- Preserve original filenames when possible; otherwise include a short mapping note in the relevant README.
- Compute and record SHA256 for the imported file.
- Add a short provenance note:
  - what it depicts (die shot / mask layer / package photo)
  - the chip + revision (e.g., 4004 vs 4004B, 4040)
  - the upstream source, author/uploader if known, and license

## After importing
- Update registries/manifests:
  - `docs/meta/registry.yaml` (new docs)
  - `docs/evidence/ocr_manifest.yaml` (new OCR sidecars)
- Run `scripts/doc_validate.sh`.
