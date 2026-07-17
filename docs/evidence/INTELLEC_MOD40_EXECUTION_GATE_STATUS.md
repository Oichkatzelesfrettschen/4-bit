# Intellec 4 MOD 40 Historical Execution Gate Status

## Decision

The historical MOD 40 execution path remains blocked. The retained sources
establish physical card inventory and several connector and local-circuit
routes. They do not establish the complete electrical or physical-media
conditions required to execute a source-faithful board cycle.

The public-byte-absence conjecture is false. Kyle Owen's wrappers are public
and the Internet Archive preserves exact copies. The historical-execution
conjecture remains unresolved because public byte availability does not provide
independent physical reads, socket identity, or a primary-backed transform.

## Gate matrix

| Gate | Verified input | Missing condition | Closure artifact | State |
| --- | --- | --- | --- | --- |
| Physical C1702A reads | Kyle documents one Prolog 980 transfer and published wrappers. Herb and Sid document separate read efforts. | Two repeatable raw 256-byte reads for every socket-identified device from two independently documented custody chains. | Per-device raw files, photos, reader/adapter/voltage logs, operator/date record, CRC32, SHA-1, SHA-256, and byte comparison report. | blocked |
| Monitor socket and transform map | PDF 3 shows A1-A4, shared 4289 A0-A7, A18 decode inputs, A10 and A8 74158 packages, 8095/TTL fabric, and a divider network. | A18 output to each C1702A CSO, selected state, complete C1702A data pin path, every inversion, and reset/phase timing. | Pin-to-pin route table with primary-sheet locators and independently checked Boolean and timing equations. | blocked |
| imm4-72 to imm6-28 cycle | PDFs 5, 7, and 10 establish MAD0-MAD11 contacts, BYTE1, BYTE2, MODULE SELECT, WRITE, high-address 7404 stages, and the 32-device array. | 3404 latch enables, local 7404/7400 equations, active-low 2102 R/W pulse, bank enable, and setup/hold timing. | Reconciled controller, motherboard, and IN-28 transaction table with timing diagram. | blocked |
| Panel and terminal control | PDF 13 establishes STOP conditioning and mode/reset boundaries. PDF 29 establishes printer, keyboard, and reader current-loop conductors and component values. 98-095A resolves the Q3/Q4/Q5 CPU-port senses and 11-symbol terminal framing. | Panel priority and edges; the conflicting CPU versus external STOP ACK polarity; terminal transistor thresholds; reader relay state and mechanical timing. | Panel state transition table and terminal electrical truth table. | blocked |
| Historical trace | Candidate public bytes and a behavioral board model exist. | Four accepted media images, primary-backed transform, complete routes, and all earlier gates closed. | Source-tagged reset-to-prompt trace with raw and normalized image digests. | blocked |

## Public-provenance search result

The 2013 Kyle correspondence explicitly says that the email attached two test
programs and four HEX dumps. The retained page exposes the four HEX wrappers,
but no attachment links or attachment filenames. A 2026-07-14 Internet Archive
CDX query for `www.retrotechnology.com/restore/kyle*` returns only the
correspondence page. The programs remain an original-mailbox or separately
archived-evidence gap.

Kyle's page reports a Prolog 980 transfer over an RS-232 adapter. It reports
that `MON 4 000 V2.1` produced all-zero operational data and all `FF` reader
bytes, and it discusses a complement relation. It does not provide a second
repeat, a socket map, voltage settings, reader log, or a second custody chain.

Public MAME history establishes two A1 candidate digests, but its commits do
not name a device, reader, operator, date, raw acquisition, or repeat
procedure. Sid Jones is documented as using Mk I and Mk II 1702A readers, but
the searched public record supplies no raw output, digest, device photo, or
repeat log. These are recovery leads, not gate-closing evidence.

A 2022 public Intel-devsys post says that one Kyle-derived binary version was
archived and that Kyle's board confirmed a working set. The post supplies no
raw 256-byte attachment, device identity, reader setting, repeated read, or
independent custody record. It establishes public binary persistence, not
physical-read lineage.

The 2026-07-16 public sweep adds the Intel Insite 4004/4040 program-library
scan, a 600 ppi 98-013A image derivative, connector transcriptions, and public
discussion records. It falsifies the narrow claim that no more relevant public
material is available. It does not change a gate state: no acquired source
contains a second raw repeat, socket-identified device evidence, a complete
monitor transform, or a primary-derived board timing equation. The bounded
search method, source digests, and protocol limits are recorded in
`INTELLEC_MOD40_PUBLIC_DISCOVERY_LOG.md`.

The recovered Silent 700 interface note prescribes terminal-specific J43 and
motherboard modifications. It establishes that a working terminal conversion
can alter the baseline machine, so terminal demonstrations remain invalid as
baseline polarity evidence unless their adapter and modification state are
identified.

## Implementation rule

`Mod40Board::source_gate()` mirrors these individual boundaries. It exposes
documented inventory, connector facts, and the reconciled terminal CPU-port
polarity, but reports zero accepted monitor read sets and false for socket map,
transform, clock/reset timing, program-RAM write timing, panel arbitration,
board-cycle wiring, and monitor-media verification. No FPGA wrapper or
equivalence trace may bypass these false conditions.

## Required external evidence intake

The repository does not contact custodians or solicit new media. If a future
publicly released artifact supplies any of the following, the recovery protocol
requires preserving it before normalization:

1. An original Kyle message with MIME attachments or a mailbox export.
2. Two complete raw dumps of every C1702A from an identified board, with
   reader logs and photos recorded before any complement operation.
3. A second identified board or another independently documented custody chain
   that supports replication.
4. A publicly released Sid Jones raw Mk I and Mk II set with reader settings
   and device photographs.
5. A publicly released provenance record for MAME's corrected A1 image, or an
   archival source that documents its acquisition.

The repository accepts no reconstructed, manually repaired, mirror-derived,
or MAME-derived image as a substitute for those artifacts.
