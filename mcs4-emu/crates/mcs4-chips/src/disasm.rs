//! Disassembler for MCS-4 (4004) and MCS-40 (4040) instructions

use std::fmt;

use crate::i4004::{Instruction, InstructionDecoder};

/// Represents a single line of disassembled code
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisasmLine {
    /// Memory address
    pub address: u16,
    /// Raw bytes (1 or 2)
    pub bytes: Vec<u8>,
    /// Instruction mnemonic (e.g., "FIM")
    pub mnemonic: &'static str,
    /// Formatted operands (e.g., "P0, 42H")
    pub operands: String,
    /// Comment (optional)
    pub comment: Option<String>,
    /// CPU cycles
    pub cycles: u8,
}

impl fmt::Display for DisasmLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format: ADDR  BYTES     MNEMONIC  OPERANDS
        // Example: 000:  20 42     FIM       P0, 42H

        let bytes_str = self
            .bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        write!(
            f,
            "{:03X}:  {:<5}     {:<8}  {}",
            self.address, bytes_str, self.mnemonic, self.operands
        )
    }
}

/// MCS-4/MCS-40 Disassembler
#[derive(Default)]
pub struct Disassembler;

impl Disassembler {
    pub fn new() -> Self {
        Self
    }

    /// Disassemble a single instruction at the given address
    pub fn disassemble_one(&self, data: &[u8], offset: usize, address: u16) -> Option<DisasmLine> {
        if offset >= data.len() {
            return None;
        }

        let byte1 = data[offset];
        let mut decoder = InstructionDecoder::new();
        decoder.decode_first(byte1);

        let mut bytes = vec![byte1];
        let mut instruction = decoder.get_instruction();

        // Handle 2-byte instructions
        if decoder.needs_second_byte() {
            if offset + 1 < data.len() {
                let byte2 = data[offset + 1];
                bytes.push(byte2);
                decoder.decode_second(byte2);
                instruction = decoder.get_instruction();
            } else {
                // Incomplete instruction at end of ROM
                return Some(DisasmLine {
                    address,
                    bytes,
                    mnemonic: "???",
                    operands: "Incomplete".to_string(),
                    comment: None,
                    cycles: 0,
                });
            }
        }

        match instruction {
            Some(instr) => Some(DisasmLine {
                address,
                bytes,
                mnemonic: instr.mnemonic(),
                operands: self.format_operands(&instr),
                comment: None,
                cycles: instr.cycles(),
            }),
            None => Some(DisasmLine {
                address,
                bytes,
                mnemonic: "???",
                operands: "Invalid".to_string(),
                comment: None,
                cycles: 0,
            }),
        }
    }

    /// Disassemble a range of memory
    pub fn disassemble_range(&self, data: &[u8], start_addr: u16) -> Vec<DisasmLine> {
        let mut lines = Vec::new();
        let mut offset = 0;
        let mut addr = start_addr;

        while offset < data.len() {
            if let Some(line) = self.disassemble_one(data, offset, addr) {
                offset += line.bytes.len();
                addr += line.bytes.len() as u16;
                lines.push(line);
            } else {
                break;
            }
        }

        lines
    }

    /// Format operands for an instruction
    fn format_operands(&self, instr: &Instruction) -> String {
        match instr {
            Instruction::Nop => String::new(),
            Instruction::Jcn { condition, addr_low } => {
                let cond_str = self.format_condition(*condition);
                format!("{}, {:02X}H", cond_str, addr_low)
            }
            Instruction::Fim { pair, data } => format!("P{}, {:02X}H", pair, data),
            Instruction::Src { pair } => format!("P{}", pair),
            Instruction::Fin { pair } => format!("P{}", pair),
            Instruction::Jin { pair } => format!("P{}", pair),
            Instruction::Jun { addr_high, addr_low } => {
                let addr = ((*addr_high as u16) << 8) | (*addr_low as u16);
                format!("L_{:03X}", addr)
            }
            Instruction::Jms { addr_high, addr_low } => {
                let addr = ((*addr_high as u16) << 8) | (*addr_low as u16);
                format!("L_{:03X}", addr)
            }
            Instruction::Isz { reg, addr_low } => format!("R{}, {:02X}H", reg, addr_low),
            Instruction::Inc { reg } => format!("R{}", reg),
            Instruction::Add { reg } => format!("R{}", reg),
            Instruction::Sub { reg } => format!("R{}", reg),
            Instruction::Ld { reg } => format!("R{}", reg),
            Instruction::Xch { reg } => format!("R{}", reg),
            Instruction::Bbl { data } => format!("{}", data),
            Instruction::Ldm { data } => format!("{}", data),
            Instruction::Dcl => String::new(),
            _ => String::new(),
        }
    }

    /// Format condition codes for JCN
    fn format_condition(&self, c: u8) -> String {
        // C3=Inv, C2=Zero, C1=Carry, C0=Test
        let mut parts = Vec::new();
        if c & 0x08 != 0 {
            parts.push("INV");
        }
        if c & 0x04 != 0 {
            parts.push("Z");
        }
        if c & 0x02 != 0 {
            parts.push("C");
        }
        if c & 0x01 != 0 {
            parts.push("T");
        }
        if parts.is_empty() {
            "0".to_string()
        } else {
            parts.join("")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disasm_simple() {
        let code = [0xD5, 0x20, 0x42]; // LDM 5, FIM P0, 42H
        let disasm = Disassembler::new();
        let lines = disasm.disassemble_range(&code, 0x100);

        assert_eq!(lines.len(), 2);

        // Check LDM 5
        assert_eq!(lines[0].address, 0x100);
        assert_eq!(lines[0].mnemonic, "LDM");
        assert_eq!(lines[0].operands, "5");

        // Check FIM P0, 42H
        assert_eq!(lines[1].address, 0x101);
        assert_eq!(lines[1].mnemonic, "FIM");
        assert_eq!(lines[1].operands, "P0, 42H");
    }
}
