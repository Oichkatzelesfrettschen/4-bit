# API

This repository exposes an evolving Rust API for simulating MCS-4 (4004) and MCS-40 (4040)
systems at an 8-phase bus-cycle level.

## Core Bus Types (`mcs4-bus`)

- `BusCycle`: A1/A2/A3, M1/M2, X1/X2/X3.
- `CycleState`: Tracks phase sequencing and two-cycle instruction fetch state.
- `DataBus`: 4-bit bidirectional bus with contention checks.
- `ControlSignals`: SYNC + CM-ROM/CM-RAM select lines + MCS-40 extras.
- `IoOp` / `ControlSignals.io_op`: best-effort decoded I/O operation for gating peripherals (avoids “always-on” behavior).

### `ControlSignals` select semantics

- `select_rom(bank, time)` / `select_ram(bank, time)` *enable* the select group and set the
  4-bit value, so selecting bank `0` is distinct from “none selected”.
- `deselect_rom(time)` / `deselect_ram(time)` disable the select group.
- `selected_rom()` / `selected_ram()` return `Option<u8>` based on enable state.
- `selected_chip()` is a best-effort diagnostic helper (logging/telemetry), not a full model.

## Systems (`mcs4-system`)

- `mcs4_system::mcs4::Mcs4System`
- `mcs4_system::mcs40::Mcs40System`

### Stepping

- `step()` advances one bus phase.
- `run_cycles(n)` advances `n` machine cycles (8 bus phases per cycle).

### X-phase ordering (bus direction matters)

- `X2` is write-oriented: CPU drives the bus, peripherals latch.
- `X3` is read-oriented: peripherals drive the bus, CPU latches.

### Port helpers

`Mcs4System`:
- `read_rom_port(chip_id)` reads the 4-bit ROM I/O output latch.
- `write_rom_port_input(chip_id, value)` writes the 4-bit ROM I/O input latch.
- `read_ram_port(bank_id, chip_id)` reads the 4-bit RAM output latch.
- `set_test_pin(state)` / `test_pin()` control/read the CPU TEST pin.

`Mcs40System`:
- `set_test_pin(state)` controls the CPU TEST pin.

### ROM loading and fixtures

- `load_rom(&[u8])`: distribute a flat ROM image across 4001 chips (256 bytes each).
- `load_rom_at(addr, &[u8])` (`Mcs4System`): patch bytes at a specific ROM address.
- `load_rom_file(path)` (`Mcs4System` / `Mcs40System`): mmap a raw byte image from disk.
- `load_rom_hex_file(path)` (`Mcs4System` / `Mcs40System`): load a text fixture containing whitespace-separated hex bytes.

The `.hex` fixture format accepts:
- whitespace-separated bytes (`DA E0 00`)
- optional `0x` prefix (`0xDA`)
- comments starting with `#`, `//`, or `;`

## CPU fetch/PC semantics (current model)

- Two-byte instructions are fetched over two machine cycles: after the first byte is decoded,
  the PC advances to fetch the operand byte next cycle.
- Instructions that *take* a branch/jump/call/return explicitly update PC and suppress the
  default “advance PC” at instruction end.

## Cluster (`mcs4-system`)

- `Cluster` connects multiple `Mcs4System` instances via `PortType`, with optional latency.
- `RomPort` and `RamPort` read from port latches; `TestPin` is readable/writable.

## Known Gaps

- 4308 and broader MCS-40 support-chip protocols are not yet integrated into system wiring.
- I/O control-line modeling is still a simplified, best-effort decode (`IoOp`), but RAM/ROM side-effects are no longer “always-on”.
