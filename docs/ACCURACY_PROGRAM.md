# Accuracy Program (Evidence → Netlist → Simulation)

This repo currently has a strong **cycle-/phase-accurate** emulator core, and an expanding set of
**evidence extraction** tools. The next fidelity jump is not “more code”, it is *traceability*:
every claim and every extracted artifact must map to a testable hypothesis.

This document scopes what “electron-accurate” could mean here, what it depends on, and what we can
ship incrementally (with acceptance tests).

## Current Baseline (What Works)

- **Level 1 (cycle-/phase-accurate) CPU + bus**: instruction timing and I/O phases validated by fixtures (`mcs4-emu/STATUS.md`).
- **Layout connectivity (`netlist_v0`)**: deterministic stitched layout nets from mask layers (`docs/evidence/netlists_v0/`).
- **Transistor candidates**: `poly ∩ diffusion` connected components (`docs/evidence/transistors/`), *not* a device netlist.
- **Schematic label OCR + anchors**: signal-coordinate cross-checks and matching scaffolds (`docs/NETLIST_WORKFLOW.md`).

## Accuracy Levels (Rescoped)

### Level 1 — Cycle-/Phase-Accurate (Current)

Acceptance:
- Instruction semantics match primary manuals.
- I/O control signals are asserted only in correct bus phases.
- End-to-end fixtures execute without bus contention and with stable read/write behavior.

### Level 2 — Gate-/Switch-Level Accurate (Next Practical Target)

**Goal:** derive behavior from extracted connectivity + devices, not hand-written logic.

Two deliverables (sequenced):
1) **Switch-level netlist**:
   - nodes + transistors (source/drain/gate) + always-on loads
   - deterministic connectivity resolution + contention detection
2) **Waveform validation**:
   - compare predicted node transitions on selected subcircuits against primary timing diagrams
     and/or analyzer waveforms (when provenance permits).

Acceptance:
- A curated set of “anchor subcircuits” (clock/SYNC, bus buffers, memory control outputs) reproduce
  expected waveforms and logic states over the correct phases.
- Regression suite: changes to extraction or solver do not regress those anchors.

### Level 3 — Electron-/Analog-Accurate (Long-Term Research Track)

**Goal:** approximate voltages/currents/RC charge dynamics sufficiently to explain edge cases.

This level requires **process + geometry** parameters and will likely be extremely slow.

Acceptance (only once prerequisites exist):
- Parasitic extraction from layout (R/C) + a calibrated transistor model (e.g. BSIM-like or a
  validated simplified MOS model).
- Demonstrated reproduction of one documented analog-sensitive phenomenon (e.g. charge sharing,
  dynamic node retention) with a primary-source justification.

Non-goals (for now):
- Full-chip SPICE simulation at useful clocks.

## Primary-Source Dependencies

“Electron-accurate” is blocked until we have first-party sources for:
- process characteristics (rails, threshold behavior, loads) and any MOS design notes,
- confirmed transistor counts (or an explicitly licensed netlist),
- 4040 die/layer imagery for parity with 4004.

Tracked in `docs/evidence/PRIMARY_SOURCES_BACKLOG.md`.

## Traceability Rule (Hard Requirement)

Every added claim must have:
- a primary source reference (URL + local OCR excerpt path), or be explicitly marked “secondary/pending”,
- a test or extraction artifact that can fail if the claim is wrong.

See `docs/CLAIMS_TO_TESTS.md`.

