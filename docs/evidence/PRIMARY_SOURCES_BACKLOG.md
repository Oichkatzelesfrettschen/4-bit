# Primary Sources Backlog (Gaps)

This backlog tracks *missing primary-source* materials needed to validate claims in `docs/AUDIT.md`
and to improve circuit/transistor fidelity. It is intentionally scoped to items that can be
retrieved with clear provenance and licensing.

## Highest priority

- **4040 die shot / mask-layer imagery**
  - Needed to move beyond package photos and enable transistor/layer evidence comparable to the 4004.
  - Acceptance: license/provenance recorded in `docs/evidence/photomicrograph_permissions.md`, plus SHA256 recorded.
- **Primary confirmation of 4004/4040 transistor counts**
  - Current sources are secondary (Wikipedia) and analyzer forensic counts (`docs/emulators/readme.txt`).
  - Target: Intel datasheets, databooks, or reliability reports that explicitly state transistor counts.
- **4040 max clock spec as an explicit primary quote**
  - Current: derived from clock period; need an explicit figure from a primary Intel spec.

## Medium priority

- **MCS-40 support chip silicon/layer artifacts** (4101/4201/4289/4308)
  - Any die shots, mask composites, or layer-separated artwork with clear license.
- **Full-size composite photomicrographs for 4001–4004**
  - Current repo contains thumbnails; full-size would improve OCR and layer alignment.

## Lower priority / opportunistic

- **Second-source documentation** (National/NEC clones)
  - Useful for cross-checking specs and possibly locating alternate die photos.
- **Board-level schematics with higher resolution scans**
  - Can improve wiring-level evidence and OCR extraction quality.

## Process (when adding a source)

- Record URL + license + attribution in `docs/evidence/photomicrograph_permissions.md`.
- Store file under an appropriate `docs/` subfolder (prefer `docs/photomicrographs/` or chip-specific `docs/<chip>/`).
- Add SHA256 to the provenance record and ensure references are linked from `docs/CHIP_ARTIFACTS.md`.

## Current lead status (2026-01-11)

- Wikimedia Commons: file search returns package photos/pinouts for 4040, but no 4040 die-shot images discovered.
- CPU Grave Yard (happytrees.org): contains a 4040 family index and package-photo references, not die shots.
