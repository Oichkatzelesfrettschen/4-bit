# Intellec 4 MOD 40 Evidence Closure Contract

`intellec/mod40_route_ledger_v1.json` is the canonical machine-readable
contract. A gate closes only when every listed closure requirement is
`verified`, each blocking route is direct, and the validator accepts the
ledger. A direct local segment does not close a board path by itself.

| Gate | Atomic closure requirements | Required closure artifact |
| --- | --- | --- |
| `cpu-phase-reset` | Divider state equation; phase polarity; 4040 edge timing; reset inversion; reset release phase | Pin-level clock and reset timing budget |
| `in28-write-timing` | 3404 logic; one-shot components; write width; 2102 control polarity; setup and hold budget | Reconciled imm4-72, motherboard, and IN-28 transaction table |
| `panel-arbitration` | STOP continuity; STOP polarity; STOP ACK polarity; control priority; transition timing | Panel state and transition table |
| `terminal-electrical` | Q3 reader driver; Q4 printer driver; Q5 keyboard receiver; relay state; mechanical timing | Current-loop and terminal timing truth tables |
| `monitor-socket-transform` | A18 socket map; chip-select polarity; data-bit routes; inversion vector; address-block socket order | Per-pin monitor selection and data-path table |
| `monitor-raw-provenance` | First raw set; repeat raw set; independent custody set; comparison report | Two repeatable, position-specific raw acquisitions per device |

The board exposes these IDs through `Mod40Board::blocked_evidence_gate_ids()`.
The board-cycle wiring condition remains separate because it requires an
implemented cycle participant after the documentary gates close.

`just mod40-evidence-report` writes the ignored deterministic status surface
to `target/mod40-evidence-status.json`. The report preserves canonical gate
order, links every requirement to its blocking route, and reports missing,
partial, and verified requirement counts for tooling and the future GUI.

## Evidence state

- `missing` means no artifact satisfies the stated acceptance condition.
- `partial` means an artifact exists but leaves a named acceptance condition
  open.
- `verified` means a source-bearing artifact satisfies the condition and the
  route ledger links it to the correct gate.

The validator rejects an unreferenced requirement, a partial route without
atomic requirements, a requirement assigned to another gate, or a closed gate
with any incomplete requirement.
