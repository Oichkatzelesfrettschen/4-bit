//! 4040 Instruction Decoder
//!
//! The 4040 supports all 46 instructions of the 4004 plus 14 new instructions.
//! Total: 60 instructions.

/// All 4040 instructions
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    // ========== 4004 Instructions ==========
    Nop,
    Jcn {
        condition: u8,
        addr_low: u8,
    },
    Fim {
        pair: u8,
        data: u8,
    },
    Src {
        pair: u8,
    },
    Fin {
        pair: u8,
    },
    Jin {
        pair: u8,
    },
    Jun {
        addr_high: u8,
        addr_low: u8,
    },
    Jms {
        addr_high: u8,
        addr_low: u8,
    },
    Isz {
        reg: u8,
        addr_low: u8,
    },
    Inc {
        reg: u8,
    },
    Add {
        reg: u8,
    },
    Sub {
        reg: u8,
    },
    Ld {
        reg: u8,
    },
    Xch {
        reg: u8,
    },
    Bbl {
        data: u8,
    },
    Ldm {
        data: u8,
    },
    Wrm,
    Wmp,
    Wrr,
    Wpm,
    Wr0,
    Wr1,
    Wr2,
    Wr3,
    Sbm,
    Rdm,
    Rdr,
    Adm,
    Rd0,
    Rd1,
    Rd2,
    Rd3,
    Clb,
    Clc,
    Iac,
    Cmc,
    Cma,
    Ral,
    Rar,
    Tcc,
    Dac,
    Tcs,
    Stc,
    Daa,
    Kbp,
    Dcl,

    // ========== 4040 Extended Instructions ==========
    /// Halt
    Hlt,
    /// Branch Back from Interrupt (and restore SRC)
    Bbs,
    /// Load Command Register (read ROM to RAM)
    Lcr,
    /// OR accumulator with Register 4
    Or4,
    /// OR accumulator with Register 5
    Or5,
    /// AND accumulator with Register 6
    An6,
    /// AND accumulator with Register 7
    An7,
    /// Designate Bank 0
    Db0,
    /// Designate Bank 1
    Db1,
    /// Select RAM Bank 0
    Sb0,
    /// Select RAM Bank 1
    Sb1,
    /// Enable Interrupts
    Ein,
    /// Disable Interrupts
    Din,
    /// Read Program Memory (ROM to Accumulator)
    Rpm,

    /// Invalid opcode
    Invalid {
        opcode: u8,
    },
}

/// Instruction decoder for the 4040
#[derive(Clone, Debug, Default)]
pub struct InstructionDecoder {
    pub opr: u8,
    pub opa: u8,
    pub two_byte: bool,
    pub operand: u8,
    pub instruction: Option<Instruction>,
}

impl InstructionDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn decode_first(&mut self, byte: u8) {
        self.opr = (byte >> 4) & 0x0F;
        self.opa = byte & 0x0F;
        self.operand = 0;

        // Determine if two-byte instruction (same as 4004)
        self.two_byte = matches!(self.opr, 0x1 | 0x2 | 0x4 | 0x5 | 0x7)
            && (self.opr != 0x2 || (self.opa & 0x01) == 0)
            && (self.opr != 0x3);

        if !self.two_byte {
            self.instruction = Some(self.decode_single_byte());
        } else {
            self.instruction = None;
        }
    }

    pub fn decode_second(&mut self, byte: u8) {
        self.operand = byte;
        self.instruction = Some(self.decode_two_byte());
    }

    pub fn decode(&mut self, instruction: u8) {
        self.decode_first(instruction);
    }

    fn decode_single_byte(&self) -> Instruction {
        match self.opr {
            0x0 => {
                match self.opa {
                    0x0 => Instruction::Nop,
                    0x1 => Instruction::Hlt,
                    0x2 => Instruction::Bbs,
                    0x3 => Instruction::Lcr,
                    0x4 => Instruction::Or4,
                    0x5 => Instruction::Or5,
                    0x6 => Instruction::An6,
                    0x7 => Instruction::An7,
                    0x8 => Instruction::Db0,
                    0x9 => Instruction::Db1,
                    0xA => Instruction::Sb0,
                    0xB => Instruction::Sb1,
                    0xC => Instruction::Ein,
                    0xD => Instruction::Din,
                    0xE => Instruction::Rpm,
                    _ => Instruction::Invalid { opcode: self.opa }, // 0x0F is undefined
                }
            }

            0x2 => {
                // SRC
                if (self.opa & 0x01) == 1 {
                    Instruction::Src { pair: self.opa >> 1 }
                } else {
                    Instruction::Invalid {
                        opcode: (self.opr << 4) | self.opa,
                    }
                }
            }

            0x3 => {
                // FIN/JIN
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

            0xE => match self.opa {
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
                _ => Instruction::Invalid {
                    opcode: (self.opr << 4) | self.opa,
                },
            },

            0xF => match self.opa {
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
                _ => Instruction::Invalid {
                    opcode: (self.opr << 4) | self.opa,
                },
            },

            _ => Instruction::Invalid {
                opcode: (self.opr << 4) | self.opa,
            },
        }
    }

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
            _ => Instruction::Invalid {
                opcode: (self.opr << 4) | self.opa,
            },
        }
    }

    pub fn get_instruction(&self) -> Option<Instruction> {
        self.instruction
    }

    pub fn needs_second_byte(&self) -> bool {
        self.two_byte && self.instruction.is_none()
    }
}

impl Instruction {
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

// Remove the old Opcode4040 enum and decode_ext function
// to avoid confusion and enforce use of the full decoder.
