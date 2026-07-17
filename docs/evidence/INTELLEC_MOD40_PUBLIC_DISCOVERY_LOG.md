# Intellec 4 MOD 40 Public Discovery Log

## Result

The claim that no further relevant public material exists is false. The
2026-07-16 public sweep located a 600 ppi schematic-image derivative, an Intel
Insite 4004/4040 program-library scan, secondary connector transcriptions,
and public provenance discussions. The sweep does not close any historical
execution gate.

The stronger claim that every relevant object on every network has been
exhausted is not a finite or testable result. The defensible absence statement
is limited to the named public source sets, indexes, and protocol-specific
catalogs searched on 2026-07-16.

## Retained discoveries

All downloaded artifacts remain in the ignored local evidence cache. Their
URLs, sizes, and SHA-256 digests are in `intellec_sources.yaml`.

| Source ID | Direct result | Gate effect |
| --- | --- | --- |
| `internet-archive-mod40-schematic-jp2-600ppi` | A 73,676,821-byte 600 ppi page-image derivative of 98-013A is available for legibility and OCR review. | Improves G2-G4 review only. It is not an independent board observation. |
| `intel-insite-4004-4040-library-hale` | The library index names item 40-5, an Intellec 4/40 Silent 700 terminal interface, and item 40-6, an Intellec 4 plus ASR-33 PROM dump utility. | Adds adjacent terminal and dump-program evidence. It does not identify monitor devices or their raw contents. |
| `retrotechnology-intellec4-*-pinout-transcription` | CPU, control, and program-RAM connector transcriptions expose a secondary cross-check graph for MAD, BYTE, WRITE, module select, clock, and TTY labels. | Detects OCR or transcription conflicts during G2-G4 extraction. Primary sheet images remain controlling. |
| `retrotechnology-dwight-mod40-note` | A former Intel test-engineer recollection describes dynamic bus drivers that require running speed. | Identifies a timing risk. It provides no timing equation. |
| `retrotechnology-440-serial-circuit` | An external serial current-loop interface drawing is public. | Supports terminal-interface investigation only; it does not prove CPU-board Q3/Q4/Q5 polarity. |
| `mame-mon4-a1-correction-patch` | MAME changed A1 on 2017-06-27 and labeled the change a transcription correction. | Proves public candidate-hash history, not a reread or physical provenance. |
| `intel-devsys-mon4-provenance-discussion` | Public discussion describes a fragmented archive and one circulating ROM version. | Confirms the provenance gap; it supplies no raw repeat-read record. |

The Insite index reference numbers are not physical PDF page numbers. The
ignored full-library OCR locator pass identifies the actual scan pages before
any circuit or program claim enters a tracked evidence record.
`INTELLEC_INSITE_4004_4040_LIBRARY_INDEX.md` records complete 369-page capture
accounting, page locators, source tiers, and non-inference boundaries.

## MON4 public provenance classification

The 2013 Kyle correspondence and the 2022 Intel-devsys post are retained as
separate evidence objects. The Kyle page records one Prolog 980 transfer and
describes four attached HEX wrappers. The later post reports that one
Kyle-derived binary version was archived and that Kyle's board confirmed a
working set. Neither object publishes a raw 256-byte device read, device
photograph, reader voltage setting, repeated read, or independent custody
chain. They preserve public-byte availability only.

| Public object | URL | Classification | Gate effect |
| --- | --- | --- | --- |
| Kyle Owen ROM recovery correspondence | `https://www.retrotechnology.com/restore/kyle_aug2013.html` | One described Prolog 980 acquisition with public HEX wrappers; no repeat or raw-read provenance. | Does not close G1 or G2. |
| Intel-devsys archive discussion | `https://groups.google.com/g/intel-devsys/c/BhE4On1pJJA` | Public statement that one Kyle-derived binary version was archived; no acquisition record. | Does not close G1 or G5. |

The repository does not solicit artifacts from the named authors or any other
custodian. A future public release is evaluated under the recovery protocol
before it influences a source gate.

## Insite extraction: items 40-5 and 40-6

The OCR locator pass identifies item 40-5 at scan pages 67 through 81 and item
40-6 at pages 83 through 87. These are archival program-library pages, not
Intel MOD 40 board drawings.

| Item | Direct content | Allowed use | Prohibited inference |
| --- | --- | --- | --- |
| 40-5, `Intellec 4/MOD 40-Silent 700 Interface` | It describes an adapter from Intellec current loops to RS-232C for a Silent 700 Model 733ASR, then lists terminal and Intellec hardware modifications. The Intellec modifications ground J43 pin 8, reverse J43 pins 3 and 4, and add a jumper across a 220-ohm motherboard resistor. | Treat the three modifications as a conditional adapter recipe and review aid for terminal endpoints. | Do not treat the modified J43 wiring as baseline MOD 40 topology or as proof of Q3/Q4/Q5 logical polarity. |
| 40-5, software section | It instructs the operator to modify monitor and assembler paper tapes in program RAM before programming four monitor EPROMs for this terminal conversion. | Treat it as evidence that terminal behavior can require monitor replacement. | Do not assign the modified monitor to A1-A4, identify its PROM family, or use it as a raw monitor acquisition. The scanned `4702A` term conflicts with the standard 1702A population established by the retained Intel manual and 98-013A, so it remains an unresolved source-specific discrepancy. |
| 40-6, `PDUP: PROM Dump Utility Program` | It documents a 77-hex-word stand-alone utility for an Intellec 4 plus ASR-33. The utility prints address-content changes from the front-panel PROM socket. The listing says it was assembled and tested on an Intellec 4 MOD 40. | Treat it as a public historical program and a possible future tool-behavior cross-check. | Do not treat the listing or its described output as a C1702A raw dump, a four-device monitor set, a socket map, or a reader log. |

The Silent 700 material resolves a historical ambiguity without closing an
electrical gate: a terminal conversion changes the machine. Therefore a
terminal demonstration or photograph cannot establish baseline current-loop
polarity unless it identifies the exact adapter and modification state.

## Public surfaces examined

| Surface | Bounded method | Result |
| --- | --- | --- |
| HTTP and HTTPS primary and collector archives | Direct title, drawing-number, part-number, filename, checksum, and site-index retrieval. | Located the Insite archive, 98-013A 600 ppi derivative, MOD 40 records, listings, and secondary card material. |
| Internet Archive | Item metadata, file listing, replay, and CDX queries for known URL patterns. | Confirmed published wrapper preservation and located a high-resolution schematic derivative. A targeted CDX request that returned service failure is not absence evidence. |
| Code hosting | MAME source and commit history. | Located the documented A1 transcription correction but no physical-read provenance. |
| Public discussion archives | Google Groups and CCTalk HTTP archives. | Located provenance and terminal-restoration context, but no socket-identified raw C1702A outputs. |
| Museum and library catalogs | Public catalog records. | Located an operator-manual catalog record without a downloadable scan. |

No new package installation is required. Existing tools cover the useful
known public surfaces. A protocol carries a request to a known endpoint; it is
not a global document index. FTP, FTPS, rsync, Gopher, Gemini, NNTP, IRC,
BitTorrent, IPFS, WebDAV, SMB, NFS, RCP, RSH, SSH, Tor, and I2P require a
named public endpoint, an index, a content identifier, or authorization.
Arbitrary host probing does not create acceptable archival evidence and is not
part of this research method.

Potential tools remain conditional: `pan` for a named public NNTP server,
`weechat` or `irssi` for a named public IRC channel or log, `yaz` for a named
Z39.50 or SRU catalog, and Tor or I2P tooling for a concrete public URI. None
of these tools establishes exhaustive discovery or substitutes for a source.

## Gate impact

| Gate | New information | State |
| --- | --- | --- |
| G1 physical C1702A reads | Public artifacts, a corrected MAME candidate, and provenance discussion exist. No source provides two repeatable raw reads for every socket-identified device from independent custody chains. | blocked |
| G2 monitor electrical mapping | 600 ppi sheets and connector transcriptions improve review. The A18 select outputs, full data path, inversion transform, and phase/reset equations remain unextracted. | blocked |
| G3 imm4-72 to imm6-28 cycle | Cross-check labels improve review. Primary equations for the 3404/7404 path, active-low write pulse, latch phase, and timing remain unextracted. | blocked |
| G4 panel and terminal control | The Insite terminal interface and external current-loop drawing add adjacent material. Panel arbitration and central-processor electrical polarity remain unproven. | blocked |
| G5 historical trace | Public program and byte artifacts exist. Earlier gates remain open. | blocked |

## Next bounded units

1. Locate and visually review the actual Insite pages for items 40-5 and 40-6.
2. Use the 600 ppi page images to extract only the CPU, controller,
   motherboard, program-RAM, panel, and TTY routes with endpoint and polarity
   locators.
3. Search known public raw-mail, archive, and source-history endpoints for
   Kyle attachments, Sid raw outputs, and MAME A1 acquisition provenance.
4. Record a negative result only against the named index or endpoint. Do not
   convert a transport failure, missing search result, or unindexed surface
   into a global absence claim.

The repository does not enable the source-gated FPGA wrapper or equivalence
trace until the five gate artifacts in
`INTELLEC_MOD40_EXECUTION_GATE_STATUS.md` exist.
