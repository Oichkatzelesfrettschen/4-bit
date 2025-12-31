# API Reference

## Emulator Interfaces
- Build/run: see README.
- Core crates: mcs4-core, mcs4-bus, mcs4-chips, mcs4-system, mcs4-gui.

### CLI
- mcs4-emu: run GUI; flags: --rom <path>, --speed <hz>.

### Public Rust API (summary)
- mcs4_chips::i4004::I4004
- mcs4_chips::i4040::I4040 (stub)
- mcs4_system::{Mcs4, Mcs40}

## Configuration
- Environment: MCS4_ROM, MCS4_RAM, LOG_LEVEL.
