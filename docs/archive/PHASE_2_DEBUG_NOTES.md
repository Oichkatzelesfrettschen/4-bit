# Phase 2 RAM Operation Debugging Notes (ARCHIVED)

> ARCHIVE NOTE (2026-07-09): Debugging notes for the Phase 2 SRC/WRM/RDM
> RAM round-trip failure, formerly at `docs/PHASE_2_DEBUG_NOTES.md`. Archived
> as a superseded snapshot; the bug was fixed and the phase is complete.
> Current status lives in `mcs4-emu/CLAUDE.md` (canonical).

Date: 2026-01-29
Focus: test_end_to_end_src_wrm_rdm_roundtrip failure analysis

## TEST SCENARIO

ROM Bytecode:
- 0xDA: LDM 0xA       (load accumulator with 0xA)
- 0x20, 0x01: FIM P0, 0x01  (fill immediate pair 0: R0=0x0, R1=0x1)
- 0x21: SRC P0        (set RAM source: chip=0, address=1)
- 0xE0: WRM           (write RAM: write 0xA to RAM[0][1])
- 0xD0: LDM 0x0       (load accumulator with 0x0)
- 0xE9: RDM           (read RAM: read RAM[0][1] to accumulator)
- 0x00: NOP

Expected Result:
- After RDM: accumulator = 0xA (10 decimal)
- RAM[0][1] = 0xA

Actual Result:
- After RDM: accumulator = 0xE (14 decimal)  FAIL
- RAM[0][1] = 0xA (this assertion not reached)

## ROOT CAUSE HYPOTHESIS

The 0xE value (14) is suspicious:
- 0xE appears in ROM: 0xE0 (WRM), 0xE9 (RDM)
- Suggests RDM might be reading from ROM instead of RAM
- Or reading garbage/uninitialized data

## DIAGNOSTIC STEPS FOR NEXT SESSION

### Step 1: Verify RAM Write
Add test to check:
- Does WRM actually write to RAM chip?
- Is ram_address/ram_chip set correctly from SRC?
- Is bus.write() called in phase_x2 for WRM?

### Step 2: Verify RAM Read
Add test to check:
- Does RAM respond to read during phase_x3?
- Is bus data valid after RAM responds?
- Is bus.read() getting correct value?

### Step 3: Bus Protocol Sync
Check:
- IO operation control lines (CM-RAM) asserted at correct phases
- RAM chip select working correctly
- Bus data timing (write in X2, read after X3)

## KEY CODE LOCATIONS

execute_4004 SRC handler (line ~315):
```
Src { pair } => {
    let addr = self.registers.get_pair(pair);
    self.ram_address = addr & 0x0F;
    self.ram_chip = (addr >> 4) & 0x0F;
}
```

phase_x2 WRM execution (line ~248):
```
execute_4004(instr, bus);  // Calls bus.write()
```

phase_x3 RDM execution (line ~289):
```
execute_4004(instr, bus);  // Calls bus.read()
```

## COMPARISON: 4004 vs 4040

- 4004 test (mcs4::tests) PASSES
- 4040 test (mcs40::tests) FAILS
- Both use identical register/bus APIs

Possible difference:
- 4040 uses i4004::InstructionDecoder
- 4040 uses i4004::Instruction enum
- Register file might have subtle behavioral difference

## NEXT SESSION PLAN

1. Add debug logging to phase_x2/x3 execution
2. Verify SRC correctly sets ram_address/ram_chip
3. Trace bus.write()/read() calls during WRM/RDM
4. Check RAM chip's write/read implementation
5. Verify IO operation control signals

Estimated effort: 2-3 hours to complete debugging + fix

---
Status: BLOCKED (requires detailed tracing)
Next action: Add instrumentation logging to phase methods
