//! 4004 Instruction Decoder
//!
//! The 4004 has 46 instructions encoded in 8 bits (OPR:OPA).
//! Two-byte instructions fetch a second byte in the following cycle.

/// All 4004 instructions
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    // ========== Machine Control (OPR=0x0) ==========
    /// No operation
    Nop,

    // ========== Conditional Jump (OPR=0x1) - 2 bytes ==========
    /// Jump if condition is true
    /// OPA encodes: C3=invert, C2=accumulator zero, C1=carry, C0=test pin
    Jcn { condition: u8, addr_low: u8 },

    // ========== Register Pair Operations (OPR=0x2, 0x3) ==========
    /// Fetch immediate to register pair - 2 bytes
    Fim { pair: u8, data: u8 },
    /// Send register control (select RAM address)
    Src { pair: u8 },
    /// Fetch indirect from ROM using pair 0
    Fin { pair: u8 },
    /// Jump indirect using pair
    Jin { pair: u8 },

    // ========== Unconditional Jumps (OPR=0x4, 0x5, 0x7) - 2 bytes ==========
    /// Jump unconditional (12-bit address)
    Jun { addr_high: u8, addr_low: u8 },
    /// Jump to subroutine (12-bit address)
    Jms { addr_high: u8, addr_low: u8 },
    /// Increment and skip if zero - 2 bytes
    Isz { reg: u8, addr_low: u8 },

    // ========== Index Register Operations (OPR=0x6, 0x8-0xC) ==========
    /// Increment register
    Inc { reg: u8 },
    /// Add register to accumulator with carry
    Add { reg: u8 },
    /// Subtract register from accumulator with borrow
    Sub { reg: u8 },
    /// Load register to accumulator
    Ld { reg: u8 },
    /// Exchange accumulator and register
    Xch { reg: u8 },
    /// Branch back and load (return from subroutine)
    Bbl { data: u8 },

    // ========== Immediate Operations (OPR=0xD) ==========
    /// Load immediate to accumulator
    Ldm { data: u8 },

    // ========== I/O and RAM Control (OPR=0xE) ==========
    /// Write memory (RAM data)
    Wrm,
    /// Write ROM port
    Wmp,
    /// Write RAM port
    Wrr,
    /// Write program RAM (status char 0)
    Wpm,
    /// Write RAM status character 0
    Wr0,
    /// Write RAM status character 1
    Wr1,
    /// Write RAM status character 2
    Wr2,
    /// Write RAM status character 3
    Wr3,
    /// Subtract memory from accumulator with borrow
    Sbm,
    /// Read memory (RAM data)
    Rdm,
    /// Read ROM port
    Rdr,
    /// Add memory to accumulator with carry
    Adm,
    /// Read RAM status character 0
    Rd0,
    /// Read RAM status character 1
    Rd1,
    /// Read RAM status character 2
    Rd2,
    /// Read RAM status character 3
    Rd3,

    // ========== Accumulator Group (OPR=0xF) ==========
    /// Clear both (accumulator and carry)
    Clb,
    /// Clear carry
    Clc,
    /// Increment accumulator
    Iac,
    /// Complement carry
    Cmc,
    /// Complement accumulator
    Cma,
    /// Rotate accumulator left through carry
    Ral,
    /// Rotate accumulator right through carry
    Rar,
    /// Transfer carry to accumulator and clear carry
    Tcc,
    /// Decrement accumulator
    Dac,
    /// Transfer carry subtract and clear
    Tcs,
    /// Set carry
    Stc,
    /// Decimal adjust accumulator
    Daa,
    /// Keyboard process
    Kbp,
    /// Designate command line (4040 extension, NOP on 4004)
    Dcl,

    /// Invalid/unknown instruction
    Invalid { opcode: u8 },
}

/// Instruction decoder for the 4004
#[derive(Clone, Debug, Default)]
pub struct InstructionDecoder {
    /// Current opcode (OPR) - upper nibble
    pub opr: u8,
    /// Current operand (OPA) - lower nibble
    pub opa: u8,
    /// Is this a two-byte instruction?
    pub two_byte: bool,
    /// Second byte (if two-byte instruction)
    pub operand: u8,
    /// Decoded instruction
    pub instruction: Option<Instruction>,
}

impl InstructionDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode first byte of instruction
    pub fn decode_first(&mut self, byte: u8) {
        self.opr = (byte >> 4) & 0x0F;
        self.opa = byte & 0x0F;
        self.operand = 0;

        // Determine if two-byte instruction
        self.two_byte = matches!(self.opr, 0x1 | 0x2 | 0x4 | 0x5 | 0x7)
            && (self.opr != 0x2 || (self.opa & 0x01) == 0)  // FIM is 2-byte, SRC is 1-byte
            && (self.opr != 0x3);  // FIN/JIN are 1-byte

        if !self.two_byte {
            self.instruction = Some(self.decode_single_byte());
        } else {
            self.instruction = None;
        }
    }

    /// Decode second byte of two-byte instruction
    pub fn decode_second(&mut self, byte: u8) {
        self.operand = byte;
        self.instruction = Some(self.decode_two_byte());
    }

    /// Legacy decode method for compatibility
    pub fn decode(&mut self, instruction: u8) {
        self.decode_first(instruction);
    }

    /// Decode single-byte instructions
    fn decode_single_byte(&self) -> Instruction {
        match self.opr {
            0x0 => Instruction::Nop,

            0x2 => {
                // SRC (send register control) - OPA bit 0 = 1
                if (self.opa & 0x01) == 1 {
                    Instruction::Src { pair: self.opa >> 1 }
                } else {
                    // FIM starts here but is 2-byte
                    Instruction::Invalid { opcode: (self.opr << 4) | self.opa }
                }
            }

            0x3 => {
                if (self.opa & 0x01) == 0 {
                    Instruction::Fin { pair: self.opa >> 1 }
                } else {
                    Instruction::Jin { pair: self.opa >> 1 }
                }
            }

            0x6 => Instruction::Inc { reg: self.opa },
            0x8 => Instruction::Add { reg: self.opa },
            0x9 => Instruction::Sub { reg: self.opa },
            0xA => Instruction::Ld { reg: self.opa },
            0xB => Instruction::Xch { reg: self.opa },
            0xC => Instruction::Bbl { data: self.opa },
            0xD => Instruction::Ldm { data: self.opa },

            0xE => {
                match self.opa {
                    0x0 => Instruction::Wrm,
                    0x1 => Instruction::Wmp,
                    0x2 => Instruction::Wrr,
                    0x3 => Instruction::Wpm,
                    0x4 => Instruction::Wr0,
                    0x5 => Instruction::Wr1,
                    0x6 => Instruction::Wr2,
                    0x7 => Instruction::Wr3,
                    0x8 => Instruction::Sbm,
                    0x9 => Instruction::Rdm,
                    0xA => Instruction::Rdr,
                    0xB => Instruction::Adm,
                    0xC => Instruction::Rd0,
                    0xD => Instruction::Rd1,
                    0xE => Instruction::Rd2,
                    0xF => Instruction::Rd3,
                    _ => Instruction::Invalid { opcode: (self.opr << 4) | self.opa },
                }
            }

            0xF => {
                match self.opa {
                    0x0 => Instruction::Clb,
                    0x1 => Instruction::Clc,
                    0x2 => Instruction::Iac,
                    0x3 => Instruction::Cmc,
                    0x4 => Instruction::Cma,
                    0x5 => Instruction::Ral,
                    0x6 => Instruction::Rar,
                    0x7 => Instruction::Tcc,
                    0x8 => Instruction::Dac,
                    0x9 => Instruction::Tcs,
                    0xA => Instruction::Stc,
                    0xB => Instruction::Daa,
                    0xC => Instruction::Kbp,
                    0xD => Instruction::Dcl,
                    _ => Instruction::Invalid { opcode: (self.opr << 4) | self.opa },
                }
            }

            _ => Instruction::Invalid { opcode: (self.opr << 4) | self.opa },
        }
    }

    /// Decode two-byte instructions
    fn decode_two_byte(&self) -> Instruction {
        match self.opr {
            0x1 => Instruction::Jcn {
                condition: self.opa,
                addr_low: self.operand,
            },
            0x2 => Instruction::Fim {
                pair: self.opa >> 1,
                data: self.operand,
            },
            0x4 => Instruction::Jun {
                addr_high: self.opa,
                addr_low: self.operand,
            },
            0x5 => Instruction::Jms {
                addr_high: self.opa,
                addr_low: self.operand,
            },
            0x7 => Instruction::Isz {
                reg: self.opa,
                addr_low: self.operand,
            },
            _ => Instruction::Invalid { opcode: (self.opr << 4) | self.opa },
        }
    }

    /// Get the current decoded instruction
    pub fn get_instruction(&self) -> Option<Instruction> {
        self.instruction
    }

    /// Check if waiting for second byte
    pub fn needs_second_byte(&self) -> bool {
        self.two_byte && self.instruction.is_none()
    }
}

impl Instruction {
    /// Get instruction mnemonic
    pub fn mnemonic(&self) -> &'static str {
        match self {
            Instruction::Nop => "NOP",
            Instruction::Jcn { .. } => "JCN",
            Instruction::Fim { .. } => "FIM",
            Instruction::Src { .. } => "SRC",
            Instruction::Fin { .. } => "FIN",
            Instruction::Jin { .. } => "JIN",
            Instruction::Jun { .. } => "JUN",
            Instruction::Jms { .. } => "JMS",
            Instruction::Isz { .. } => "ISZ",
            Instruction::Inc { .. } => "INC",
            Instruction::Add { .. } => "ADD",
            Instruction::Sub { .. } => "SUB",
            Instruction::Ld { .. } => "LD",
            Instruction::Xch { .. } => "XCH",
            Instruction::Bbl { .. } => "BBL",
            Instruction::Ldm { .. } => "LDM",
            Instruction::Wrm => "WRM",
            Instruction::Wmp => "WMP",
            Instruction::Wrr => "WRR",
            Instruction::Wpm => "WPM",
            Instruction::Wr0 => "WR0",
            Instruction::Wr1 => "WR1",
            Instruction::Wr2 => "WR2",
            Instruction::Wr3 => "WR3",
            Instruction::Sbm => "SBM",
            Instruction::Rdm => "RDM",
            Instruction::Rdr => "RDR",
            Instruction::Adm => "ADM",
            Instruction::Rd0 => "RD0",
            Instruction::Rd1 => "RD1",
            Instruction::Rd2 => "RD2",
            Instruction::Rd3 => "RD3",
            Instruction::Clb => "CLB",
            Instruction::Clc => "CLC",
            Instruction::Iac => "IAC",
            Instruction::Cmc => "CMC",
            Instruction::Cma => "CMA",
            Instruction::Ral => "RAL",
            Instruction::Rar => "RAR",
            Instruction::Tcc => "TCC",
            Instruction::Dac => "DAC",
            Instruction::Tcs => "TCS",
            Instruction::Stc => "STC",
            Instruction::Daa => "DAA",
            Instruction::Kbp => "KBP",
            Instruction::Dcl => "DCL",
            Instruction::Invalid { .. } => "???",
        }
    }

    /// Get instruction length in bytes
    pub fn length(&self) -> u8 {
        match self {
            Instruction::Jcn { .. }
            | Instruction::Fim { .. }
            | Instruction::Jun { .. }
            | Instruction::Jms { .. }
            | Instruction::Isz { .. } => 2,
            _ => 1,
        }
    }

    /// Get number of machine cycles
    pub fn cycles(&self) -> u8 {
        match self {
            Instruction::Jcn { .. }
            | Instruction::Fim { .. }
            | Instruction::Jun { .. }
            | Instruction::Jms { .. }
            | Instruction::Isz { .. }
            | Instruction::Fin { .. }
            | Instruction::Jin { .. } => 2,
            _ => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_nop() {
        let mut decoder = InstructionDecoder::new();
        decoder.decode_first(0x00);
        assert_eq!(decoder.get_instruction(), Some(Instruction::Nop));
        assert!(!decoder.two_byte);
    }

    #[test]
    fn test_decode_ldm() {
        let mut decoder = InstructionDecoder::new();
        decoder.decode_first(0xD5); // LDM 5
        assert_eq!(decoder.get_instruction(), Some(Instruction::Ldm { data: 5 }));
    }

    #[test]
    fn test_decode_add() {
        let mut decoder = InstructionDecoder::new();
        decoder.decode_first(0x8A); // ADD R10
        assert_eq!(decoder.get_instruction(), Some(Instruction::Add { reg: 10 }));
    }

    #[test]
    fn test_decode_jun() {
        let mut decoder = InstructionDecoder::new();
        decoder.decode_first(0x42); // JUN 2xx
        assert!(decoder.two_byte);
        assert!(decoder.needs_second_byte());

        decoder.decode_second(0xAB);
        assert_eq!(
            decoder.get_instruction(),
            Some(Instruction::Jun { addr_high: 2, addr_low: 0xAB })
        );
    }

    #[test]
    fn test_decode_fim() {
        let mut decoder = InstructionDecoder::new();
        decoder.decode_first(0x20); // FIM P0
        assert!(decoder.two_byte);

        decoder.decode_second(0x42);
        assert_eq!(
            decoder.get_instruction(),
            Some(Instruction::Fim { pair: 0, data: 0x42 })
        );
    }

    #[test]
    fn test_decode_src() {
        let mut decoder = InstructionDecoder::new();
        decoder.decode_first(0x21); // SRC P0
        assert!(!decoder.two_byte);
        assert_eq!(decoder.get_instruction(), Some(Instruction::Src { pair: 0 }));
    }

    #[test]
    fn test_decode_accumulator_group() {
        let mut decoder = InstructionDecoder::new();

        decoder.decode_first(0xF0);
        assert_eq!(decoder.get_instruction(), Some(Instruction::Clb));

        decoder.decode_first(0xF5);
        assert_eq!(decoder.get_instruction(), Some(Instruction::Ral));

        decoder.decode_first(0xFB);
        assert_eq!(decoder.get_instruction(), Some(Instruction::Daa));
    }

    #[test]
    fn test_decode_io() {
        let mut decoder = InstructionDecoder::new();

        decoder.decode_first(0xE0);
        assert_eq!(decoder.get_instruction(), Some(Instruction::Wrm));

        decoder.decode_first(0xE9);
        assert_eq!(decoder.get_instruction(), Some(Instruction::Rdm));
    }

    #[test]
    fn test_instruction_metadata() {
        assert_eq!(Instruction::Nop.length(), 1);
        assert_eq!(Instruction::Nop.cycles(), 1);
        assert_eq!(Instruction::Nop.mnemonic(), "NOP");

        let jun = Instruction::Jun { addr_high: 0, addr_low: 0 };
        assert_eq!(jun.length(), 2);
        assert_eq!(jun.cycles(), 2);
        assert_eq!(jun.mnemonic(), "JUN");
    }
}
