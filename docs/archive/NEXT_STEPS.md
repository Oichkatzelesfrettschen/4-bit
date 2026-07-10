# MCS-4 / MCS-40 Ecosystem: Next Steps (ARCHIVED)

> ARCHIVE NOTE (2026-07-09): Point-in-time roadmap snapshot dated 2026-03-05,
> formerly at the repository root as `NEXT_STEPS.md`. Archived as a superseded
> snapshot; the Phase 6-8 work it planned has shipped. Current status lives in
> `mcs4-emu/CLAUDE.md` (canonical); the forward plan lives in `docs/ROADMAP.md`.

**Date**: 2026-03-05 (point-in-time roadmap snapshot)
**Status authority**: SNAPSHOT, not canonical. Items below were planned 2026-03-05;
many are now complete. Cross-check against `mcs4-emu/CLAUDE.md` (canonical phase
status) and `docs/ROADMAP.md` (forward plan) before acting on any item. Phase 6
gate-extraction, Phase 7 new chips and behavioral Verilog (22 modules), and
Phase 8 FPGA constraints/Makefile have all completed since this document was
written; see `mcs4-emu/STATUS.md` session log.
**Based on**: SCOPING_ASSESSMENT.md findings
**Scope**: Actionable work items, ordered by ROI and dependency chain

---

## EXECUTIVE SUMMARY

The project has two independent work streams with no blockers:

1. **Circuit-level completion for MCS-4 core** (4 chips with transistor data)
2. **Behavioral chip expansion + FPGA deployment** (no die shots needed)

A third stream (circuit-level MCS-40) is **hard-blocked** on acquiring 4040 die shots.

---

## PHASE 6: CIRCUIT-LEVEL MCS-4 CORE

### WHY
The full solver stack (21K lines, 504 tests) is proven end-to-end but only exercised on
6 transistors (clock buffer) and 37 transistors (4003). The remaining 3,705
(corrected; per-chip kept-device sum, see SCOPING_ASSESSMENT.md) transistors
across 4 chips have extracted netlists and subcircuits but no simulation wiring. Completing
this stream delivers the project's core differentiator: circuit-level simulation of real
silicon from photomicrographs.

### 6.1 Gate extraction -- unblock gate-level pipeline

**Current state**: `extract_gates_v0.py` has pattern detectors (inverter, NAND, NOR,
transmission gate) implemented but the `extract_gates_from_netlist()` function at line 211
is a placeholder that returns empty gates. The detection functions are never called.

**Task**: Wire the detection functions into the main extraction pipeline.

```
File: scripts/extract_gates_v0.py
What: In extract_gates_from_netlist(), load the NetlistV1 JSON, parse transistors
      into Transistor dataclass instances, call detect_inverter() + detect_nand_gates()
      + detect_nor_gates() + detect_transmission_gates(), aggregate results, and
      populate the output dict.
Input: docs/evidence/netlists_v1/{chip}_netlist_v1.json
Output: docs/evidence/gates_v0/{chip}/{chip}_gates_v0.json (populated)
```

**Acceptance criteria**:
- [ ] `python scripts/extract_gates_v0.py` produces nonzero gate counts
- [ ] gates_v0 JSON files contain gate instances with input/output node IDs
- [ ] Gate count is plausible: 4003 (37T) should yield ~15-18 gates;
      4004 (1,030T) should yield ~400-500 gates

**Estimated scope**: ~50 lines of Python (loading JSON, parsing transistors, calling
existing detectors, writing results)

### 6.2 Subcircuit enumeration in I4004 bridge

**Current state**: `I4004::subcircuit_names()` returns only `["clock_buffer"]`.
The subcircuit JSON data exists for 11 nonzero-transistor subcircuits in
`docs/evidence/subcircuits_v0/4004/`.

**Task**: Extend `I4004`'s `ChipSolverBridge` to load subcircuit JSON data and expose
all 11 subcircuits.

```
File: mcs4-emu/crates/mcs4-chips/src/i4004/solver_bridge.rs
What: Add a method that loads a subcircuit JSON from the evidence directory,
      parses it via layout_netlist, and converts to CircuitGraph via
      netlist_bridge::netlist_v1_to_circuit().
Pattern: Follow the nodal_4003 test (mcs4-core/tests/nodal_4003.rs) which
         already does: load_netlist_v1() -> netlist_v1_to_circuit() -> graph
```

**Subcircuits to wire** (from `subcircuits_v0/4004/metrics.json`):
1. CLK1 -- 680 nodes, 437 transistors
2. CMROM -- 462 nodes, 289 transistors
3. CMRAM0 -- 382 nodes, 233 transistors
4. D0_PAD -- 382 nodes, 233 transistors
5. CLK2 -- 169 nodes, 149 transistors
6. D2_PAD -- 91 nodes, 57 transistors
7. CMRAM3 -- 60 nodes, 39 transistors
8. CMRAM2 -- 42 nodes, 30 transistors
9. D1_PAD -- 4 nodes, 2 transistors
10. D3_PAD -- 3 nodes, 2 transistors
11. CMRAM1 -- 3 nodes, 1 transistor

**Acceptance criteria**:
- [ ] `subcircuit_names()` returns 12 entries (clock_buffer + 11 from JSON)
- [ ] `subcircuit("CLK1")` returns a CircuitGraph with 437 transistors
- [ ] Each subcircuit graph has valid VDD/VSS power rails assigned
- [ ] New tests validate graph structure for at least 3 subcircuits

### 6.3 ChipSolverBridge for I4001, I4002, I4003

**Current state**: Only I4004 and I4040 implement ChipSolverBridge. I4001/I4002/I4003
have transistor netlist data (nonzero subcircuits: 11, 6, 5 respectively) but no bridge.

**Task**: Implement ChipSolverBridge for the three remaining core chips. Each follows
the same pattern as I4004 -- load subcircuit JSONs, expose via trait methods.

```
Files to create or extend:
  mcs4-chips/src/i4001.rs -- add ChipSolverBridge impl
  mcs4-chips/src/i4002.rs -- add ChipSolverBridge impl
  mcs4-chips/src/i4003.rs -- add ChipSolverBridge impl
```

**Acceptance criteria**:
- [ ] All three chips return subcircuit names matching their evidence data
- [ ] I4003 subcircuit("CLOCK") returns a graph with <=9 transistors
- [ ] Pin maps use node IDs from schematic_layout_anchors_v1.json

### 6.4 Full-chip 4003 transient simulation

**Current state**: The `nodal_4003` test runs DC operating point only.

**Task**: Extend to a full transient simulation of the 4003 shift register clocking
a data bit through the 10-stage pipeline.

```
File: mcs4-emu/crates/mcs4-core/tests/nodal_4003.rs (extend)
What: After DC solve, drive CLOCK with a pulse, DATA with a bit pattern,
      and EN high. Verify Q0 output toggles after clock edges.
Pattern: Use TransientSolver::run() with PulseSource stimuli, same as
         the clock_buffer_transient_produces_waveforms test.
```

**Acceptance criteria**:
- [ ] Transient solver converges for 37 transistors
- [ ] Clock-to-Q delay measurable from waveform data
- [ ] At least 10 clock cycles simulated without divergence

### 6.5 Full-chip 4004 DC operating point

**Current state**: Not attempted. The DC solver has been tested on 37 transistors (4003)
and 6 transistors (clock buffer). The 4004 has 1,030 transistors.

**Task**: Load the full 4004 netlist and run DC operating point.

```
File: mcs4-emu/crates/mcs4-core/tests/dc_4004.rs (new)
What: Load docs/evidence/netlists_v1/4004_netlist_v1.json
      Convert via netlist_v1_to_circuit() + apply parasitics
      Run DcSolver with sparse backend (>100 nodes triggers auto-switch)
      Verify convergence and check key node voltages (CLK, SYNC, D0-D3)
```

**Acceptance criteria**:
- [ ] DC solver converges for 1,030 transistors
- [ ] Sparse backend auto-selected (1,588 nodes > 100 threshold)
- [ ] Power rails at expected voltages (VDD=-15V, VSS=0V)
- [ ] Convergence in <50 Newton-Raphson iterations

**Risk**: Convergence failure at this scale. Mitigation: use subcircuit decomposition
(solve largest subcircuit CLK1 at 437T first, then progressively larger sets).

### 6.6 Behavioral-vs-circuit cross-validation

**Current state**: FidelityManager designed but no end-to-end test comparing a behavioral
tick() output against circuit simulation output for the same machine cycle.

**Task**: For one complete 4004 bus phase (e.g., A1: CPU drives address bits 0-3),
run the behavioral model tick, then solve the same phase using circuit simulation,
and compare the output bus values.

```
File: mcs4-emu/crates/mcs4-system/tests/cross_validation.rs (new)
What: Create a MCS-4 system with one 4004 at Behavioral fidelity.
      Step through A1 phase, capture bus output.
      Build the relevant subcircuit (e.g., D0_PAD), set input to the
      address bit value, run DC solve, compare output voltage to
      expected logic level.
```

**Acceptance criteria**:
- [ ] Behavioral output bit matches circuit simulation output voltage polarity
- [ ] Voltage is within rail-to-rail range (logic 1 -> near VSS, logic 0 -> near VDD)
- [ ] Test documents any discrepancies with analysis

### 6.7 Populate gate-level Verilog

**Depends on**: 6.1 (gate extraction must produce nonzero gates)

**Task**: Run `gate_to_verilog_v0.py` against populated gates_v0 data.

```
Command: python scripts/gate_to_verilog_v0.py
Input: docs/evidence/gates_v0/{chip}/{chip}_gates_v0.json (populated from 6.1)
Output: docs/evidence/verilog_v0/{chip}/*.v (populated module bodies)
```

**Acceptance criteria**:
- [ ] Verilog module bodies contain wire declarations and gate instances
- [ ] Gate library primitives (inv, nand2, nor2, etc.) instantiated correctly
- [ ] Verilog passes syntax check (`iverilog -t null` or similar)

---

## PHASE 7: BEHAVIORAL CHIP EXPANSION

### WHY
5 Intel first-party chips and 4 external memory chip families are missing behavioral
implementations. The 3205 decoder and 2101 SRAM are simple, well-documented, and enable
more complete system configurations.

### 7.1 Intel 3205 -- 1-of-8 binary decoder

**Difficulty**: LOW (standard TTL decoder, 20-30 lines of Rust)

```
File: mcs4-emu/crates/mcs4-chips/src/i3205.rs
What: 3-bit address input -> 8 active-low outputs, chip enable
Tests: 4-6 (decode each output, chip enable, all-high default)
```

### 7.2 Intel 3404 -- 6-bit D-type latch / dual NAND buffer

**Difficulty**: LOW (standard TTL latch, 20-30 lines)

```
File: mcs4-emu/crates/mcs4-chips/src/i3404.rs
What: 6-bit D latch with clock input + dual NAND gate buffer
Tests: 4-6 (latch data, clock edge, NAND outputs)
```

### 7.3 Intel 2101 -- 256x4 SRAM

**Difficulty**: LOW (same interface pattern as existing 4101)

```
File: mcs4-emu/crates/mcs4-chips/src/i2101.rs
What: 256x4 asynchronous SRAM, CE/WE/OE control, tri-state output
Reference: 4101 implementation (i4101.rs) for interface pattern
Tests: 8-10 (read/write, address range, chip enable, tri-state)
```

### 7.4 Intel 4269 -- Programmable Keyboard/Display Interface

**Difficulty**: HIGH (complex state machine with scan matrix)

```
File: mcs4-emu/crates/mcs4-chips/src/i4269.rs
What: 8x8 keyboard scan matrix, 16-digit display driver, programmable modes
Reference: 1975 Intel Data Catalog (docs/MCS-40/1975_Intel_Data_Catalog.pdf)
Tests: 15-20 (scan modes, display segments, interrupt generation)
```

### 7.5 Behavioral Verilog for remaining chips

**Task**: Extend `mcs4-fpga` crate to generate Verilog for all 18 (or more) chips.

```
File: mcs4-emu/crates/mcs4-fpga/src/verilog.rs
What: Add chip_i4008(), chip_i4009(), chip_i3216(), chip_i3226(),
      chip_i4101(), chip_i4201(), chip_i4207(), chip_i4209(),
      chip_i4211(), chip_i4265(), chip_i4289(), chip_i4308(),
      chip_i4316(), chip_i4702() functions.
Pattern: Follow existing chip_i4001()/chip_i4002()/chip_i4003()/chip_i4004()
```

**Acceptance criteria**:
- [ ] `all_chip_modules()` returns 18+ modules
- [ ] Each module has correct port declarations matching chip datasheet

---

## PHASE 8: FPGA DEPLOYMENT

### WHY
Behavioral Verilog already exists for 4 chips and can be extended to 18. The synthesis
workflow is fully documented. Only constraint files and actual synthesis runs are missing.

### 8.1 Create iCE40 constraint files

```
File: mcs4-emu/fpga/ice40/4001.pcf (new)
      mcs4-emu/fpga/ice40/4002.pcf (new)
      mcs4-emu/fpga/ice40/4003.pcf (new)
What: Pin assignments from FPGA_SYNTHESIS_WORKFLOW_V0.md templates
      Map chip I/O pins to iCE40-HX4K TQ144 package pins
```

### 8.2 Synthesis Makefile

```
File: mcs4-emu/fpga/Makefile (new)
What: Yosys synth_ice40 -> nextpnr-ice40 -> icepack pipeline
      Targets: synth, pnr, pack, prog, clean
      Start with 4003 (smallest, ~50 LUTs)
```

### 8.3 Synthesize and validate 4003 (smallest chip)

**Acceptance criteria**:
- [ ] Yosys synthesis succeeds for i4003_behavioral
- [ ] nextpnr place-and-route succeeds on iCE40-HX4K
- [ ] Resource utilization reported (~50 LUTs expected)
- [ ] Timing analysis passes at 2 MHz

### 8.4 System-level Verilog top module

```
File: mcs4-emu/fpga/mcs4_top.v (new)
What: Instantiate 4004 + 4001 + 4002 + 4003 behavioral modules
      Wire 4-bit data bus, control signals, clock
      Add UART debug bridge for host communication
```

---

## PHASE 9: EXTENDED SYSTEMS

### 9.1 Intellec-4/MOD 40

**Difficulty**: MEDIUM (reuse existing Intellec-4 crate + 4040 CPU)

```
What: Extend mcs4-intellec to support MOD 40 configuration
      Use Intel_Intellec_4_MOD_40_Reference_Schematics.pdf (78 MB, in repo)
      Add 4040-specific front panel controls, ROM/RAM expansion
```

### 9.2 Busicom 141-PF calculator

**Difficulty**: HIGH (requires ROM dump, mechanical model for printer)

```
What: Implement the Busicom calculator application ROM
      Model the printer mechanism and keyboard
      ROM image may be available from the Intel 4004 50th anniversary project
```

---

## DEPENDENCY GRAPH

```
              [6.1 Gate extraction]
                      |
                      v
              [6.7 Gate-level Verilog] ------> [8.x FPGA]
                                                   ^
[6.2 I4004 subcircuits] --+                        |
[6.3 I400x bridges]    --+--> [6.5 DC 4004] --> [6.6 Cross-validation]
                          |                        |
                          +--> [6.4 4003 transient]|
                                                   |
[7.1-7.4 New chips] --> [7.5 Behavioral Verilog] --+

[9.1 MOD 40] -- independent
[9.2 Busicom] -- independent, HIGH difficulty
```

Items 6.1, 6.2, 6.3, 7.1-7.4 can all start in parallel.
Items 6.4 and 6.5 depend on 6.2/6.3.
Item 6.6 depends on 6.4/6.5.
Item 6.7 depends on 6.1.
All Phase 8 items depend on 7.5.

---

## HARD BLOCKERS (cannot be solved by coding)

| Blocker | Impact | Current Status |
|---------|--------|----------------|
| 4040 die shot | Blocks ALL circuit-level MCS-40 work | No known public source |
| MCS-40 support chip die shots | Blocks circuit-level 4101/4201/4289/4308 | No known public source |
| FPGA board procurement | Blocks Phase 8.3+ hardware validation | ~$10-20, documented |

---

## RECOMMENDED EXECUTION ORDER

**Week 1**: Items 6.1 + 6.2 + 6.3 in parallel
- Gate extraction is ~50 lines of Python
- Subcircuit wiring is mechanical (follow nodal_4003 pattern)
- Bridge impls for I4001/I4002/I4003 are ~100 lines each

**Week 2**: Items 6.4 + 6.5 + 7.1 + 7.2
- 4003 transient simulation (37T, should converge easily)
- 4004 DC operating point (1,030T, main risk item)
- 3205 decoder + 3404 latch (simple behavioral chips)

**Week 3**: Items 6.6 + 6.7 + 7.3
- Cross-validation (depends on 6.4/6.5 results)
- Gate-level Verilog population (depends on 6.1)
- 2101 SRAM behavioral model

**Week 4+**: Items 7.5 + 8.1-8.4
- Behavioral Verilog for all chips
- FPGA constraint files and synthesis

---

*Generated 2026-03-05. Companion to SCOPING_ASSESSMENT.md.*
