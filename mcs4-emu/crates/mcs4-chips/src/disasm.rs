//! Disassembler for MCS-4 (4004) and MCS-40 (4040) instruction sets
//!
//! Converts binary ROM images to human-readable assembly listings.

use crate::i4004::instruction_decode::{Instruction, InstructionDecoder};
use std::collections::HashMap;

/// CPU type affects instruction decode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuType {
    /// Intel 4004 (46 instructions)
    I4004,
    /// Intel 4040 (60 instructions, backward compatible)
    I4040,
}

/// Single disassembled line
#[derive(Clone, Debug)]
pub struct DisasmLine {
    /// Address of instruction
    pub address: u16,
    /// Raw bytes (1 or 2)
    pub bytes: Vec<u8>,
    /// Mnemonic (e.g., "NOP", "JUN", "FIM")
    pub mnemonic: String,
    /// Operands (e.g., "P0, 42H", "L_123")
    pub operands: String,
    /// Optional comment
    pub comment: Option<String>,
    /// Is this a jump target?
    pub is_jump_target: bool,
}

/// Disassembler for MCS-4/MCS-40
pub struct Disassembler {
    /// CPU type
    #[allow(dead_code)]
    cpu_type: CpuType,
    /// Symbol table (address -> label)
    symbols: HashMap<u16, String>,
    /// Comments (address -> comment)
    comments: HashMap<u16, String>,
}

impl Disassembler {
    /// Create new disassembler
    pub fn new(cpu_type: CpuType) -> Self {
        Self {
            cpu_type,
            symbols: HashMap::new(),
            comments: HashMap::new(),
        }
    }

    /// Add symbol at address
    pub fn add_symbol(&mut self, addr: u16, name: String) {
        self.symbols.insert(addr, name);
    }

    /// Add comment at address
    pub fn add_comment(&mut self, addr: u16, comment: String) {
        self.comments.insert(addr, comment);
    }

    /// Disassemble single instruction at address
    pub fn disasm_one(&self, rom: &[u8], addr: u16) -> DisasmLine {
        let addr_usize = (addr & 0x0FFF) as usize;
        
        if addr_usize >= rom.len() {
            return DisasmLine {
                address: addr,
                bytes: vec![],
                mnemonic: "???".to_string(),
                operands: String::new(),
                comment: Some("Address out of range".to_string()),
                is_jump_target: self.symbols.contains_key(&addr),
            };
        }

        let mut decoder = InstructionDecoder::new();
        let first_byte = rom[addr_usize];
        decoder.decode_first(first_byte);

        let mut bytes = vec![first_byte];
        
        if decoder.two_byte {
            let second_addr = (addr + 1) & 0x0FFF;
            if (second_addr as usize) < rom.len() {
                let second_byte = rom[second_addr as usize];
                decoder.decode_second(second_byte);
                bytes.push(second_byte);
            }
        }

        let instruction = decoder.instruction.unwrap_or(Instruction::Invalid { opcode: first_byte });
        let (mnemonic, operands) = self.format_instruction(&instruction, addr);
        
        DisasmLine {
            address: addr,
            bytes,
            mnemonic,
            operands,
            comment: self.comments.get(&addr).cloned(),
            is_jump_target: self.symbols.contains_key(&addr),
        }
    }

    /// Disassemble range of addresses
    pub fn disasm_range(&self, rom: &[u8], start: u16, end: u16) -> Vec<DisasmLine> {
        let mut lines = Vec::new();
        let mut addr = start & 0x0FFF;
        let end = end & 0x0FFF;

        while addr <= end && (addr as usize) < rom.len() {
            let line = self.disasm_one(rom, addr);
            let length = line.bytes.len() as u16;
            lines.push(line);
            
            if length == 0 {
                break;
            }
            addr = (addr + length) & 0x0FFF;
            
            if addr == 0 && end != 0 {
                break;
            }
        }

        lines
    }

    /// Format instruction as mnemonic and operands
    fn format_instruction(&self, instr: &Instruction, addr: u16) -> (String, String) {
        match instr {
            Instruction::Nop => ("NOP".to_string(), String::new()),
            
            Instruction::Jcn { condition, addr_low } => {
                let target = ((addr & 0x0F00) | (*addr_low as u16)) & 0x0FFF;
                let label = self.symbols.get(&target)
                    .cloned()
                    .unwrap_or_else(|| format!("L_{:03X}", target));
                ("JCN".to_string(), format!("{:X}H, {}", condition, label))
            }
            
            Instruction::Fim { pair, data } => {
                ("FIM".to_string(), format!("P{}, {:02X}H", pair, data))
            }
            
            Instruction::Src { pair } => {
                ("SRC".to_string(), format!("P{}", pair))
            }
            
            Instruction::Fin { pair } => {
                ("FIN".to_string(), format!("P{}", pair))
            }
            
            Instruction::Jin { pair } => {
                ("JIN".to_string(), format!("P{}", pair))
            }
            
            Instruction::Jun { addr_high, addr_low } => {
                let target = (((*addr_high as u16) << 8) | (*addr_low as u16)) & 0x0FFF;
                let label = self.symbols.get(&target)
                    .cloned()
                    .unwrap_or_else(|| format!("L_{:03X}", target));
                ("JUN".to_string(), label)
            }
            
            Instruction::Jms { addr_high, addr_low } => {
                let target = (((*addr_high as u16) << 8) | (*addr_low as u16)) & 0x0FFF;
                let label = self.symbols.get(&target)
                    .cloned()
                    .unwrap_or_else(|| format!("L_{:03X}", target));
                ("JMS".to_string(), label)
            }
            
            Instruction::Isz { reg, addr_low } => {
                let target = ((addr & 0x0F00) | (*addr_low as u16)) & 0x0FFF;
                let label = self.symbols.get(&target)
                    .cloned()
                    .unwrap_or_else(|| format!("L_{:03X}", target));
                ("ISZ".to_string(), format!("R{}, {}", reg, label))
            }
            
            Instruction::Inc { reg } => ("INC".to_string(), format!("R{}", reg)),
            Instruction::Add { reg } => ("ADD".to_string(), format!("R{}", reg)),
            Instruction::Sub { reg } => ("SUB".to_string(), format!("R{}", reg)),
            Instruction::Ld { reg } => ("LD".to_string(), format!("R{}", reg)),
            Instruction::Xch { reg } => ("XCH".to_string(), format!("R{}", reg)),
            Instruction::Bbl { data } => ("BBL".to_string(), format!("{:X}H", data)),
            Instruction::Ldm { data } => ("LDM".to_string(), format!("{:X}H", data)),
            
            Instruction::Wrm => ("WRM".to_string(), String::new()),
            Instruction::Wmp => ("WMP".to_string(), String::new()),
            Instruction::Wrr => ("WRR".to_string(), String::new()),
            Instruction::Wpm => ("WPM".to_string(), String::new()),
            Instruction::Wr0 => ("WR0".to_string(), String::new()),
            Instruction::Wr1 => ("WR1".to_string(), String::new()),
            Instruction::Wr2 => ("WR2".to_string(), String::new()),
            Instruction::Wr3 => ("WR3".to_string(), String::new()),
            Instruction::Sbm => ("SBM".to_string(), String::new()),
            Instruction::Rdm => ("RDM".to_string(), String::new()),
            Instruction::Rdr => ("RDR".to_string(), String::new()),
            Instruction::Adm => ("ADM".to_string(), String::new()),
            Instruction::Rd0 => ("RD0".to_string(), String::new()),
            Instruction::Rd1 => ("RD1".to_string(), String::new()),
            Instruction::Rd2 => ("RD2".to_string(), String::new()),
            Instruction::Rd3 => ("RD3".to_string(), String::new()),
            
            Instruction::Clb => ("CLB".to_string(), String::new()),
            Instruction::Clc => ("CLC".to_string(), String::new()),
            Instruction::Iac => ("IAC".to_string(), String::new()),
            Instruction::Cmc => ("CMC".to_string(), String::new()),
            Instruction::Cma => ("CMA".to_string(), String::new()),
            Instruction::Ral => ("RAL".to_string(), String::new()),
            Instruction::Rar => ("RAR".to_string(), String::new()),
            Instruction::Tcc => ("TCC".to_string(), String::new()),
            Instruction::Dac => ("DAC".to_string(), String::new()),
            Instruction::Tcs => ("TCS".to_string(), String::new()),
            Instruction::Stc => ("STC".to_string(), String::new()),
            Instruction::Daa => ("DAA".to_string(), String::new()),
            Instruction::Kbp => ("KBP".to_string(), String::new()),
            Instruction::Dcl => ("DCL".to_string(), String::new()),
            
            Instruction::Invalid { opcode } => {
                ("???".to_string(), format!("{:02X}H", opcode))
            }
        }
    }

    /// Auto-generate labels for jump targets
    pub fn auto_label(&mut self, rom: &[u8]) {
        let lines = self.disasm_range(rom, 0, rom.len() as u16 - 1);
        
        for line in &lines {
            if let Some(target) = self.extract_jump_target(&line.mnemonic, &line.operands, line.address) {
                if !self.symbols.contains_key(&target) {
                    self.symbols.insert(target, format!("L_{:03X}", target));
                }
            }
        }
    }

    /// Extract jump target from instruction if it's a jump/call
    fn extract_jump_target(&self, mnemonic: &str, operands: &str, addr: u16) -> Option<u16> {
        match mnemonic {
            "JUN" | "JMS" => {
                // Format: "L_XXX" or "XXX"
                if let Some(stripped) = operands.strip_prefix("L_") {
                    u16::from_str_radix(stripped, 16).ok()
                } else {
                    u16::from_str_radix(operands, 16).ok()
                }
            }
            "JCN" | "ISZ" => {
                // Format: "cond, L_XXX" or "reg, L_XXX"
                if let Some(comma_pos) = operands.rfind(',') {
                    let target_str = operands[comma_pos + 1..].trim();
                    if let Some(stripped) = target_str.strip_prefix("L_") {
                        u16::from_str_radix(stripped, 16).ok()
                    } else {
                        // Calculate same-page target
                        if let Some(val) = target_str.strip_suffix('H') {
                            if let Ok(low) = u16::from_str_radix(val, 16) {
                                Some((addr & 0x0F00) | low)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Format as assembly listing
    pub fn format_listing(&self, lines: &[DisasmLine]) -> String {
        let mut output = String::new();
        
        output.push_str("; MCS-4 Disassembly\n\n");
        
        for line in lines {
            // Label if this is a jump target
            if line.is_jump_target {
                if let Some(label) = self.symbols.get(&line.address) {
                    output.push_str(&format!("{}:\n", label));
                }
            }
            
            // Address
            output.push_str(&format!("{:03X}:  ", line.address));
            
            // Bytes
            for byte in &line.bytes {
                output.push_str(&format!("{:02X} ", byte));
            }
            for _ in line.bytes.len()..2 {
                output.push_str("   ");
            }
            
            // Instruction
            output.push_str(&format!("{:<6}", line.mnemonic));
            if !line.operands.is_empty() {
                output.push_str(&format!(" {}", line.operands));
            }
            
            // Comment
            if let Some(comment) = &line.comment {
                output.push_str(&format!("  ; {}", comment));
            }
            
            output.push('\n');
        }
        
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disasm_nop() {
        let disasm = Disassembler::new(CpuType::I4004);
        let rom = vec![0x00];
        let line = disasm.disasm_one(&rom, 0);
        
        assert_eq!(line.mnemonic, "NOP");
        assert_eq!(line.operands, "");
        assert_eq!(line.bytes.len(), 1);
    }

    #[test]
    fn test_disasm_ldm() {
        let disasm = Disassembler::new(CpuType::I4004);
        let rom = vec![0xD5];
        let line = disasm.disasm_one(&rom, 0);
        
        assert_eq!(line.mnemonic, "LDM");
        assert_eq!(line.operands, "5H");
    }

    #[test]
    fn test_disasm_fim() {
        let disasm = Disassembler::new(CpuType::I4004);
        let rom = vec![0x20, 0x42];
        let line = disasm.disasm_one(&rom, 0);
        
        assert_eq!(line.mnemonic, "FIM");
        assert_eq!(line.operands, "P0, 42H");
        assert_eq!(line.bytes.len(), 2);
    }

    #[test]
    fn test_disasm_jun() {
        let disasm = Disassembler::new(CpuType::I4004);
        let rom = vec![0x41, 0x23];
        let line = disasm.disasm_one(&rom, 0);
        
        assert_eq!(line.mnemonic, "JUN");
        assert_eq!(line.operands, "L_123");
    }

    #[test]
    fn test_auto_label() {
        let mut disasm = Disassembler::new(CpuType::I4004);
        let rom = vec![
            0xD5,       // LDM 5
            0x41, 0x23, // JUN 0x123
            0x00,       // NOP
        ];
        
        disasm.auto_label(&rom);
        assert!(disasm.symbols.contains_key(&0x123));
        assert_eq!(disasm.symbols.get(&0x123), Some(&"L_123".to_string()));
    }

    #[test]
    fn test_format_listing() {
        let mut disasm = Disassembler::new(CpuType::I4004);
        let rom = vec![0xD5, 0x00, 0x20, 0x42];
        
        disasm.auto_label(&rom);
        let lines = disasm.disasm_range(&rom, 0, 3);
        let listing = disasm.format_listing(&lines);
        
        assert!(listing.contains("LDM"));
        assert!(listing.contains("NOP"));
        assert!(listing.contains("FIM"));
    }
}
