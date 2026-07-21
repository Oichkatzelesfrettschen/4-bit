# Intellec 4 MOD 40 Unobtained Evidence Register

## Scope

This register names evidence that the repository does not possess and cannot
derive honestly from its retained corpus. It does not assert that an item no
longer exists or can never be obtained. Each status applies only to the public
surfaces, archival indexes, retained scans, and physical observations recorded
by this repository.

The route ledger remains the machine-readable gate contract. This document is
the publication-facing acquisition list. A new artifact changes a gate only
after intake, hashing, provenance review, and validation under the linked
acceptance protocol.

## Status vocabulary

- `not-located-public` means the named object is absent from the bounded public
  surfaces recorded in `INTELLEC_MOD40_PUBLIC_DISCOVERY_LOG.md`.
- `external-custody-required` means the evidence requires an original mailbox,
  physical device, physical board, or custodian record that the repository
  does not control.
- `higher-authority-source-required` means the retained source is incomplete,
  ambiguous, or illegible at the required pin, component, or timing boundary.
- `measurement-required` means a source-backed physical observation is needed
  because the documentary record does not close the electrical behavior.

No status means permanently unavailable. One provenance-complete artifact
that satisfies the stated intake condition falsifies the corresponding status.

## Public objects not located

| ID | Unobtained object | What exists | Required intake | Status |
| --- | --- | --- | --- | --- |
| `U-PUB-001` | Original RFC 822 or MIME Kyle Owen correspondence with its two test-program attachments | The edited correspondence page and four public Intel HEX wrappers exist. The page exposes no attachment link or filename. | Preserve the original message, headers, MIME structure, attachment bytes, custodian, acquisition date, and SHA-256 digests. | not-located-public |
| `U-PUB-002` | Sid Jones Mk I and Mk II C1702A reader raw outputs | Public discussion records the reader efforts but publishes no device-bound raw output or repeat log. | Preserve unmodified output files, reader versions, settings, device photographs, socket or label identity, operator, date, and digests. | not-located-public |
| `U-PUB-003` | Physical-acquisition provenance for MAME's corrected A1 image | MAME publishes the candidate digest and calls the change a transcription correction. | Preserve the source device or source document, acquisition method, raw bytes before repair, operator, date, repeat result, and digests. | not-located-public |
| `U-PUB-004` | Downloadable Intellec 4 Operator's Manual | A public catalog record and adjacent Intel manuals exist. No downloadable scan appears in the recorded search surface. | Preserve a complete scan with title, revision, page accounting, source URL or custodian, and file digest. | not-located-public |
| `U-PUB-005` | Downloadable Intellec 4 Hardware and Microcomputer Modules Reference Manual | System, MCS-4, MCS-40, and MOD 40 materials exist, but this separately titled manual is not retained. | Preserve a complete revision-identified scan and digest. | not-located-public |
| `U-PUB-006` | Dedicated imm4-90 paper-tape reader service or schematic package | The module number and documented operating rate exist in adjacent sources. | Preserve the official schematic, service, assembly, or parts package with drawing and revision identity. | not-located-public |
| `U-PUB-007` | Separate official imm4-43 CPU-module parts list, ECO package, or master drawing set | 98-013A supplies assembly and schematic sheets but not a complete separately released revision history. | Preserve Intel-issued parts, ECO, or master-drawing records with revision identity and digests. | not-located-public |
| `U-PUB-008` | Separate official imm4-72 memory-control parts list, ECO package, or master drawing set | 98-013A supplies assembly and schematic sheets but leaves component and route ambiguities. | Preserve Intel-issued parts, ECO, or master-drawing records with revision identity and digests. | not-located-public |

## Physical media and custody evidence

| ID | Unobtained object | Gate requirement | Required intake | Status |
| --- | --- | --- | --- | --- |
| `U-MEDIA-001` | First accepted raw acquisition of all four socket-identified monitor C1702A devices | `monitor-first-raw-read-set` | Record board and device photographs, A1-A4 identity, labels, reader, adapter, voltages, operator, date, raw 256-byte outputs, two consistent reads per device, CRC32, SHA-1, and SHA-256. | external-custody-required |
| `U-MEDIA-002` | Second accepted acquisition from an independent custody chain | `monitor-independent-custody-set` | Record a separately identified board or separately documented custody chain with the complete `U-MEDIA-001` acquisition record. | external-custody-required |
| `U-MEDIA-003` | Byte-for-byte comparison of two accepted raw acquisition sets | `monitor-read-set-comparison` | Compare every device before complement, repair, normalization, socket reordering, or reconstruction and retain every difference. | external-custody-required |

The Kyle wrappers, Internet Archive replays, MAME files, mirrors, repaired
images, listings, and reconstructed binaries do not satisfy these three
items. They represent public byte artifacts or preserved copies, not separate
physical observations.

## Higher-authority electrical evidence

| ID | Unobtained object | Blocked boundary | Required intake | Status |
| --- | --- | --- | --- | --- |
| `U-ELEC-001` | Definitive A18-output to A1-A4 C1702A select map | `monitor-a18-output-socket-map`, `monitor-chip-select-polarity`, and `monitor-address-block-socket-order` | Supply a legible primary master, revision-controlled ECO, or continuity record that identifies every package pin and selected level. | higher-authority-source-required |
| `U-ELEC-002` | Complete C1702A D0-D7 route and inversion vector | `monitor-data-bit-routes` and `monitor-data-inversion-vector` | Supply a pin-to-pin primary route or board continuity record through every 74158, 8095, TTL, and 4289 boundary. | higher-authority-source-required |
| `U-ELEC-003` | Complete CPU divider, phase polarity, reset inversion, and reset-release timing | `cpu-phase-reset` | Supply legible primary equations and package timing, or capture both 4040 clock pins and reset at identified package pins on an unmodified board. | measurement-required |
| `U-ELEC-004` | Complete IN-28 one-shot composition and active-low 2102 write timing | `in28-write-timing` | Supply legible component values and pins plus the calculated tolerance budget, or capture address, data, CE, and R/W at identified IN-28 and 2102 pins. | measurement-required |
| `U-ELEC-005` | Complete panel STOP, STOP ACK, reset, single-step, and store-selection arbitration | `panel-arbitration` | Supply the missing continuity and priority equations, or record identified panel, motherboard, control-card, and 4040 pins for every transition. | measurement-required |
| `U-ELEC-006` | Complete baseline Q3, Q4, and Q5 current-loop polarity and timing | `terminal-electrical` | Supply transistor thresholds, relay contact state, and mechanical timing from the unmodified baseline circuit or an instrumented baseline machine. | measurement-required |

The 600 ppi 98-013A derivative, registered 1200 DPI crops, MinerU output,
Tesseract output, RapidOCR output, and secondary connector transcriptions are
review aids. They do not replace an unreadable endpoint with a verified net.

## Items that remain engineering work

The following outputs are not unobtainable external artifacts. They remain
blocked implementation products whose prerequisites appear above:

1. The reconciled MOD 40 board-cycle transaction model.
2. The source-tagged reset-to-prompt historical execution trace.
3. The source-gated MOD 40 FPGA wrapper.
4. The behavioral versus FPGA equivalence trace.

The repository implements these products only after all controlling evidence
gates close. Loading candidate monitor bytes or copying generic MCS-40 wiring
does not satisfy the prerequisites.

## Intake and status-change procedure

1. Preserve the acquired object without transformation.
2. Record source, custodian, acquisition time, retrieval method, and license or
   redistribution constraint.
3. Record byte size and SHA-256 before OCR, repair, normalization, or format
   conversion.
4. Add the object to `intellec_sources.yaml` or the local-only evidence cache,
   according to its retention terms.
5. Apply the acceptance rules in
   `INTELLEC_MOD40_MON4_RECOVERY_PROTOCOL.md` or
   `INTELLEC_MOD40_CLOSURE_CONTRACT.md`.
6. Update the route and component-pin ledgers only when the artifact closes a
   named acceptance condition.
7. Run `just mod40-evidence-validate` and the full repository verification.
8. Change this register only after the accepted artifact falsifies its prior
   absence status.

## Search boundary

The bounded public search covers the surfaces and methods in
`INTELLEC_MOD40_PUBLIC_DISCOVERY_LOG.md`. A protocol is a transport to a known
endpoint, not a global index. FTP, FTPS, rsync, Gopher, Gemini, NNTP, IRC,
BitTorrent, IPFS, WebDAV, SMB, NFS, RCP, RSH, SSH, Tor, and I2P require a named
public endpoint, catalog, content identifier, or authorization. Failure to
locate an object through those bounded surfaces never proves nonexistence.
