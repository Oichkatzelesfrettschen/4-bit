# Intellec 4 MOD 40 MON4 Recovery Protocol

## Current result

At least three separately documented physical-read efforts have been located:
Kyle Owen in 2013, Herb Johnson in 2013, and Sid Jones in 2022 through 2023.
The public record does not contain two repeatable, machine-readable raw
256-byte acquisitions for each device in one revision-identified,
socket-identified set. No position-specific image has a verified socket map,
primary-backed polarity record, independent lineage, or accepted
concatenation order. Historical boot therefore remains blocked.

The retained local artifacts are evidence inputs, not executable monitor media:

| Artifact | Class | Permitted use |
| --- | --- | --- |
| `MON4_V2.1_Listing.pdf` | secondary listing scan | Compare a later verified reconstruction against listing text. |
| `MOD40_Central_Processor_Module_Kyle.jpg` | secondary visual reference | Record package-label and photograph-relative position hypotheses. |
| `krom0.hex` through `krom3.hex` | documented secondary physical-read artifacts | Preserve raw public bytes and their digests; do not use as monitor firmware. |
| `hrom0.txt` through `hrom3.txt` | documented secondary manual transcription | Preserve comparison bytes and their digests; do not treat them as direct reader output. |

The photograph visibly labels four C1702A packages, from photograph-left to
right, as `MON 4 V2.1 000`, `100`, `200`, and `300`. It does not prove socket
identity, address order, electrical connectivity, contents, or polarity. The
Kyle `krom0.hex` artifact decodes to 256 `FF` bytes. The Herb comparison set
contains different package labels and manual transcription. The public Sid
Jones account publishes no raw byte files. These are useful provenance leads,
not accepted monitor media. `INTELLEC_MOD40_MON4_READ_MATRIX.md` records the
per-artifact digest and mismatch evidence.

## Required raw-read manifest

Each acquisition records one row for every physical device before any transform:

| Required field | Requirement |
| --- | --- |
| Device identity | Visible label, board coordinate, socket designator, and full-resolution photo. |
| Raw payload | Exactly 256 bytes for a C1702A, preserved unchanged. |
| Reader provenance | Reader, adapter, voltage settings, software version, operator, date, and error log. |
| Digest | SHA-256 of the raw 256-byte file. |
| Custody | Board or device provenance sufficient to distinguish independent sets. |

Two independent normalized read sets are required position by position. A
second reader applied to the same chips is replication, not independent physical
provenance. Any mismatch remains a conflict; the process never repairs a byte
silently. An independently documented physical read requires a device photo,
custodian, date, reader and adapter, raw output, two consistent reads, and
recorded CRC32, SHA-1, and SHA-256 digests.

## Transform gate

Raw data remains authoritative. A normalized image may exist only after a
primary hardware source and reproducible functional check identify every
polarity, data-bit, address-bit, and socket-order transform. The manifest keeps
the raw input, transform description, normalized output, and SHA-256 digest for
each `000`, `100`, `200`, and `300` device.

Only after the two normalized sets agree may the four images be concatenated
into a 1024-byte monitor set and compared against the secondary listing. A
listing agreement is a consistency check, not source authority.

## Acceptance gate

`intellec4-mod40-monitor-prom-set` resolves only when all conditions hold:

1. Two independent position-specific raw read sets exist.
2. Every raw and normalized image has a recorded SHA-256 digest.
3. Socket order and transform are primary-source-backed.
4. Both normalized sets agree at every byte.
5. A source-tagged reset-to-prompt trace reproduces the retained monitor set.

Until then, the profile, board, GUI, replay, and FPGA paths reject historical
monitor execution. The public MAME source records candidate MON 4 V2.1 hashes,
but MAME-source history does not establish physical-read independence. The
candidate values remain recovery leads rather than accepted monitor media.
