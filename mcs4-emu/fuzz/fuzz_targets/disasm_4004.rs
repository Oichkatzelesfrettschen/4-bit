//! Fuzz target: 4004 disassembler.
//!
//! Invariant: any byte sequence fed as a ROM image must not cause a panic.
//! Invalid opcodes, truncated two-byte instructions, and all-zeros ROMs are
//! all legal inputs for this contract.

#![no_main]

use libfuzzer_sys::fuzz_target;
use mcs4_chips::disasm::{CpuType, Disassembler};

fuzz_target!(|data: &[u8]| {
    // Clamp to 4 KiB -- the maximum addressable 4001 ROM space.
    let rom = if data.len() > 4096 { &data[..4096] } else { data };

    let disasm = Disassembler::new(CpuType::I4004);
    // disasm_all iterates the entire ROM; must not panic on any content.
    let _lines = disasm.disasm_all(rom);
});
