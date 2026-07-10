//! Census tests backing the documented instruction-set size claims.
//!
//! The MCS-4 datasheet defines 46 instructions for the 4004; the 4040
//! datasheet adds 14 new opcodes for a total of 60. These tests derive the
//! counts from the decoders themselves so the claim is enforced against the
//! implementation, not restated beside it.

use std::{collections::HashSet, mem::discriminant};

use mcs4_chips::{
    i4004::{Instruction, InstructionDecoder},
    i4040::instruction_decode::decode_4040_specific,
};

/// Every distinct instruction the 4004 decoder can produce across the whole
/// first-byte opcode space. Two-byte instructions vary only their operand
/// payload with the second byte, so one representative second byte suffices
/// for a mnemonic census.
fn i4004_census() -> usize {
    let mut seen = HashSet::new();
    for first_byte in 0..=255u8 {
        let mut decoder = InstructionDecoder::new();
        decoder.decode_first(first_byte);
        if decoder.two_byte {
            decoder.decode_second(0x00);
        }
        match decoder.get_instruction() {
            // The Invalid sentinel marks undecodable opcodes; it is not an
            // instruction and stays out of the census.
            Some(Instruction::Invalid { .. }) | None => {}
            Some(instruction) => {
                seen.insert(discriminant(&instruction));
            }
        }
    }
    seen.len()
}

/// Every distinct 4040-specific instruction across the opcode space.
fn i4040_new_census() -> usize {
    let mut seen = HashSet::new();
    for opcode in 0..=255u8 {
        if let Some(instruction) = decode_4040_specific(opcode) {
            seen.insert(discriminant(&instruction));
        }
    }
    seen.len()
}

#[test]
fn i4004_decoder_yields_46_instructions() {
    assert_eq!(i4004_census(), 46, "4004 datasheet instruction count");
}

#[test]
fn i4040_adds_14_instructions_for_60_total() {
    let new_ops = i4040_new_census();
    assert_eq!(new_ops, 14, "4040 datasheet: 14 new instructions");
    assert_eq!(i4004_census() + new_ops, 60, "4040 total instruction count");
}
