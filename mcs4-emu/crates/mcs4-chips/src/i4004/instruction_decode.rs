//! 4004 Instruction Decoder

/// Instruction decoder for the 4004
#[derive(Clone, Debug, Default)]
pub struct InstructionDecoder {
    /// Current opcode (OPR)
    pub opr: u8,
    /// Current operand (OPA)
    pub opa: u8,
    /// Is this a two-byte instruction?
    pub two_byte: bool,
}

impl InstructionDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode an instruction byte
    pub fn decode(&mut self, instruction: u8) {
        self.opr = (instruction >> 4) & 0x0F;
        self.opa = instruction & 0x0F;
        self.two_byte = matches!(self.opr, 0x1 | 0x2 | 0x4 | 0x5 | 0x7);
    }
}
