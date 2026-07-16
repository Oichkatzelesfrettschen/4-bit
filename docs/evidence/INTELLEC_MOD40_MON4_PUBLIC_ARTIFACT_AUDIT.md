# Intellec 4 MOD 40 MON4 Public Artifact Audit

## Result

The conjecture that MON4 data does not exist on the public network is false.
Kyle Owen's four Intel HEX wrappers remain available from the original
Retrotechnology HTTPS site. The Internet Archive also serves four 2015
captures that are byte-for-byte identical to the current wrappers. MAME and
public MAME-set indexes publish the complemented digests for the three
non-erased Kyle artifacts and a different candidate digest for A1.

This result proves public availability and preservation of byte artifacts. It
does not prove a second physical acquisition, a socket map, data polarity, or
the source of the MAME A1 candidate. The historical monitor gate remains
closed.

## Scope and method

This audit uses public HTTPS endpoints, Internet Archive CDX and replay
endpoints, the public MAME Git history, public code-hosting search, and
publicly indexed MAME-set metadata. It does not probe unindexed hosts, scan
network address ranges, or treat a mirror as an independent physical read.

The local-only acquisition cache uses this fixed downloader identity:

```text
Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0
```

The exact fetchable inputs are recorded in `intellec_sources.yaml`. The local
cache is intentionally ignored by Git until redistribution terms are known.

## Kyle acquisition record

Kyle's correspondence says that he removed four Intel C1702A devices from his
board and transferred four Intel HEX files through a Prolog 980 RS-232
adapter. It identifies `MON 4 000 V2.1` as all-zero operational data and all
`FF` bytes in the reader representation. It describes one acquisition, not a
repeat-read procedure. The source does not publish the two test programs that
the correspondence says were attached to the original email.

The correspondence contains the exact request, "Please find attached two
handy test programs (both with a starting address of 000) and my four HEX
dumps." The rendered page contains no attachment links or attachment names.
A 2026-07-14 Internet Archive CDX query for
`www.retrotechnology.com/restore/kyle*` returns only the correspondence page,
not an attachment object. This narrows the recovery path to an original mailbox
export or a separately documented archive. It does not prove that no copy
exists outside that URL pattern or archive index.

| Artifact | Local-only path | SHA-256 | Meaning |
| --- | --- | --- | --- |
| Correspondence | `kyle_owen_20260714/kyle_aug2013.html` | `7cd267e4f3bee88de48e1294be3d4fd742a995673ccc1a926e24a3203c61b241` | Edited 2013 correspondence describing the Prolog 980 acquisition and damaged `000` device. |
| Acquisition index | `kyle_owen_20260714/intel440_roms.html` | `53cc80b532132b323ef598accb7cb5d920e067052ed7c503116bbf48882abebf` | Identifies the four files and documents the secondary inversion claim. |
| Board photograph | `kyle_owen_20260714/intel440_kylecpu.jpg` | `eff5eef8fd74ef80b22e1132a7098a6a70a6a3b657b21133c6901e34502f244f` | Supports visible package labels only. |
| Monitor notes | `kyle_owen_20260714/440_mon.txt` | `51ba080de83d4df1d37fc5000c7ee6f70e1eba4b24089a9e516fea877cb62bb3` | Operational interpretation; it is not a PROM read. |
| Partial decode | `kyle_owen_20260714/krom1a.txt` | `902a5e021599c7ef87e2da503fe72329300a54a8907a05dc2a650bfe9dc45f90` | Manual inverse decode. The source states that two bytes were later found incorrectly inverted. |
| Full disassembly | `kyle_owen_20260714/440_kyle_disass.txt` | `6a9dd3a2ee0cd5dc389ef105da9ae737822f5f378a49c8194d5bf069c1088435` | Derived disassembly. The source records a later correction at address `00CE`. |

The two undeclared email attachments are an explicit missing-artifact item.
They must be obtained from an original mailbox export or a separately
documented archive before they enter this corpus.

## Published Intel HEX wrappers

The wrapper digests identify the files as published. The decoded raw digests
identify their 256-byte payloads. `FF count` confirms the all-`FF` first
artifact without treating it as valid monitor content.

| Label | Wrapper SHA-256 | Raw CRC32 | Raw SHA-1 | Raw SHA-256 | FF count |
| --- | --- | --- | --- | --- | ---: |
| 000 | `fb582d424a8b7da89e7aa1c9bfdfe5a5d2e0b92dd7c6abb8cbe47264bb941da1` | `fea8a821` | `c744cac6af7621524fc3a2b0a9a135a32b33c81b` | `3d6876a0146de8576eb2395a858de1213d1b92c65b779df3a331cfd5a4584546` | 256 |
| 100 | `75111c1273ddac7196bf32eb1c0f774d2e6e658bcf3993992a2da7edb351e65b` | `95680736` | `a245916e8f0a38b7f1b4460886a738ef388405a1` | `7b8a4793e8294afeb9e5275742d220b4be896b474a0289c620fa4d2d484fdc86` | 6 |
| 200 | `f44535e41685264c45acd60996015096e180ef1d9604af7c7355992228ed761e` | `0d3db111` | `1270ce2c29070e3ee9a7cc229f8f1ce16c797e15` | `5a1f3e556772f12b365e8d1041df2abf2a5ad45ab18eafaf6a82efac92ccd467` | 4 |
| 300 | `405792e956d8ddbf3715fa467c768f0af30d12792b44ebe68ad9cc94dc75d74f` | `c41af8d6` | `0a6f272bf6e948c4117a7fffaf56a3117d3816c8` | `4dab5b59d54acaabd8243b558cbd8f7bc0641159919581b2612450adb193aba4` | 13 |

## Independent archival preservation

Internet Archive CDX indexes one 2015 capture for each wrapper. Replay with
the `id_` representation yields the exact current wrapper bytes. This is
independent preservation of a web object, not an independent observation of a
C1702A.

| Label | CDX timestamp | CDX digest | Current wrapper equality |
| --- | --- | --- | --- |
| 000 | `20150716020313` | `MW7IEZ4BJLAZ5Z7K742L3WGOIQ3T6RGB` | exact |
| 100 | `20150716020354` | `Y6U2QY22CXOXAAVE4LNPDBIH75YBOKGO` | exact |
| 200 | `20150716020323` | `NSVPZUTBSRXK4EVVWHVPOPQO7ZRHIBKP` | exact |
| 300 | `20150716020340` | `67FWIYFNSBQHUNWYPRY7FPM5JLMNS352` | exact |

The archived files are retained under
`kyle_owen_20260714/wayback_20150716/` and have the same SHA-256 values as
the wrappers above.

## Public derivative distribution

MAME declares a complemented candidate for the four devices. Kyle `100`,
`200`, and `300` complement to the MAME digests exactly. MAME uses a different
A1 candidate, `CRC32 8d1f56ff` and
`SHA-1 96bc19be9be4e92195fad82d7a3cadb763ab6e3f`.

The initial MAME declaration used a different A1 candidate,
`CRC32 0a08d83d` and
`SHA-1 e7044f3b54a16d925aaffc85b0c6001f740ea4c5`. The public MAME commits
that introduce and update the driver do not identify the physical chip, reader,
operator, date, raw output, or repeat procedure. A public MAME-set index
repeats the current hashes. These sources show distribution lineage only.

The 2017-06-27 MAME correction patch describes the A1 update as a
"transcription error" fix. That wording strengthens the conclusion that the
patch is not evidence of a new physical read. Its retained digest and URL are
recorded under `mame-mon4-a1-correction-patch` in `intellec_sources.yaml`.

## Falsification result and remaining boundary

The broad conjecture is falsified: public byte artifacts exist through the
original web server, an archival replay service, source-hosted MAME metadata,
and public MAME-set indexes.

The narrow historical conjecture remains unresolved: no searched public
source supplies two independently documented, repeatable raw reads for each
of four socket-identified C1702A devices. The record does not prove that no
such evidence exists on any unindexed medium or protocol. It records only that
the searched public surface does not supply it.

`INTELLEC_MOD40_MON4_READ_MATRIX.md` remains the acceptance record. Neither
this audit nor any web mirror authorizes a monitor image, historical boot,
FPGA wrapper, or equivalence trace.
