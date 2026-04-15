# Primary Sources Backlog (Gaps)

This backlog tracks *missing primary-source* materials needed to validate claims in `docs/AUDIT.md`
and to improve circuit/transistor fidelity. It is intentionally scoped to items that can be
retrieved with clear provenance and licensing.

## Highest priority

- **4040 mask-layer imagery + additional die coverage**
  - One licensed 4040 die photo is now in-repo: `docs/photomicrographs/4040/4040-die-wepwawet-117.jpg` (CC BY-NC-SA 4.0).
  - Still needed: mask-layer imagery (or higher-fidelity die/layer sets) to support transistor/layer evidence parity with 4004.
  - Acceptance: license/provenance recorded in `docs/evidence/photomicrograph_permissions.md`, plus SHA256 recorded.
- **Primary confirmation of 4004/4040 transistor counts**
  - Current sources are secondary (Wikipedia) and analyzer forensic counts (`docs/emulators/readme.txt`).
  - Target: Intel datasheets, databooks, or reliability reports that explicitly state transistor counts.
- **4040 max clock spec as an explicit primary quote**
  - Current: derived from clock period; need an explicit figure from a primary Intel spec.

- **Process + device characteristics required for “electron-accurate” modeling**
  - Goal: parameterize any future switch-/analog-level solver with *first-party* constraints.
  - Targets (first-party preferred):
    - supply rails and recommended operating regions for MCS-4/MCS-40 parts (already partially captured),
    - pMOS logic family notes (enhancement + depletion load conventions, threshold/drive behavior),
    - any Intel MOS design notes or process summaries tied to MCS-4 era parts.
  - Acceptance: archived under `docs/` with OCR sidecars, and cross-referenced from `docs/ACCURACY_PROGRAM.md`.

## Medium priority

- **MCS-40 support chip silicon/layer artifacts** (4101/4201/4289/4308)
  - Any die shots, mask composites, or layer-separated artwork with clear license.
- **Full-size composite photomicrographs for 4001–4004**
  - Current repo contains thumbnails; full-size would improve OCR and layer alignment.

- **Primary timing diagrams for subcircuit validation**
  - Goal: validate extracted netlists/switch models against canonical waveforms (CLK1/CLK2/SYNC, CM-ROM/CM-RAM, RD/WR).
  - Acceptance: OCR excerpts (or vector diagrams) stored under `docs/evidence/ocr/` with precise page references.

## Lower priority / opportunistic

- **Second-source documentation** (National/NEC clones)
  - Useful for cross-checking specs and possibly locating alternate die photos.
- **Board-level schematics with higher resolution scans**
  - Can improve wiring-level evidence and OCR extraction quality.

- **Layout-to-device extraction references**
  - Goal: document, with primary or well-scoped secondary sources, how to interpret contacts/vias/poly/diffusion
    for this family’s masks (e.g., diffusion split-by-poly).
  - Acceptance: references linked from `docs/NETLIST_WORKFLOW.md`.

## Process (when adding a source)

- Record URL + license + attribution in `docs/evidence/photomicrograph_permissions.md`.
- Store file under an appropriate `docs/` subfolder (prefer `docs/photomicrographs/` or chip-specific `docs/<chip>/`).
- Add SHA256 to the provenance record and ensure references are linked from `docs/CHIP_ARTIFACTS.md`.

## Current lead status (2026-04-07)

- Wikimedia Commons: file search returns package photos/pinouts for 4040, but no 4040 die-shot images discovered.
- CPU Graveyard - Die shots (`https://happytrees.org/dieshots/Intel_-_4040`) provides a 4040 die image
  (`Intel-4040die1shot_117.jpg`) with an explicit CC BY-NC-SA 4.0 license; imported into repo with
  provenance + SHA256.
- Local scan of `docs/datasheets/1976_Intel_Data_Catalog.pdf` and
  `docs/datasheets/1978_Intel_Component_Data_Catalog.pdf` did not yield explicit
  primary quotes for 4004/4040 transistor counts or an explicit 4040 max-clock figure.
- Remaining highest-priority evidence gaps are now primary transistor-count confirmation, explicit 4040 max-clock quote,
  and mask-layer artifacts suitable for extraction.
- OCR benchmarks for 4001/4002/4003 still pending (tasks #70-74, deferred).
