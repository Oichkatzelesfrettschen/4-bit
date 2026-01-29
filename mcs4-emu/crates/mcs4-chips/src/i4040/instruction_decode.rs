//! 4040 Instruction Decoder
//!
//! Extends 4004 with 14 new instructions (60 total)

use crate::i4004::instruction_decode::Instruction as I4004Instruction;

/// 4040-specific instructions (14 new opcodes)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum I4040Instruction {
    /// Halt (0x01) - enter low-power mode
    Hlt,
    /// Branch Back from interrupt (0x02) - restore SRC and return
    Bbs,
    /// Load Command RAM (0x03) - read ROM into RAM
    Lcr,
    /// OR accumulator with R4 (0x04)
    Or4,
    /// OR accumulator with R5 (0x05)
    Or5,
    /// AND accumulator with R6 (0x06)
    An6,
    /// AND accumulator with R7 (0x07)
    An7,
    /// Designate Bank 0 (0x08) - switch to register bank 0
    Db0,
    /// Designate Bank 1 (0x09) - switch to register bank 1
    Db1,
    /// Select RAM Bank 0 (0x0A)
    Sb0,
    /// Select RAM Bank 1 (0x0B)
    Sb1,
    /// Enable Interrupts (0x0C)
    Ein,
    /// Disable Interrupts (0x0D)
    Din,
    /// Read Program Memory (0x0E) - read ROM byte to accumulator
    Rpm,
}

/// Combined instruction set (4004 + 4040)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    /// 4004 instruction (backward compatible)
    I4004(I4004Instruction),
    /// 4040-specific instruction
    I4040(I4040Instruction),
}

/// Decode 4040 instruction from opcode
pub fn decode_4040_specific(opcode: u8) -> Option<I4040Instruction> {
    match opcode {
        0x01 => Some(I4040Instruction::Hlt),
        0x02 => Some(I4040Instruction::Bbs),
        0x03 => Some(I4040Instruction::Lcr),
        0x04 => Some(I4040Instruction::Or4),
        0x05 => Some(I4040Instruction::Or5),
        0x06 => Some(I4040Instruction::An6),
        0x07 => Some(I4040Instruction::An7),
        0x08 => Some(I4040Instruction::Db0),
        0x09 => Some(I4040Instruction::Db1),
        0x0A => Some(I4040Instruction::Sb0),
        0x0B => Some(I4040Instruction::Sb1),
        0x0C => Some(I4040Instruction::Ein),
        0x0D => Some(I4040Instruction::Din),
        0x0E => Some(I4040Instruction::Rpm),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_4040_opcodes() {
        assert_eq!(decode_4040_specific(0x01), Some(I4040Instruction::Hlt));
        assert_eq!(decode_4040_specific(0x02), Some(I4040Instruction::Bbs));
        assert_eq!(decode_4040_specific(0x0C), Some(I4040Instruction::Ein));
        assert_eq!(decode_4040_specific(0x0D), Some(I4040Instruction::Din));
        assert_eq!(decode_4040_specific(0x00), None); // NOP is 4004
        assert_eq!(decode_4040_specific(0x0F), None); // Not a 4040-specific opcode
    }
}
