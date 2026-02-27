//! Disassembler for 4004 and 4040 CPUs
//!
//! Provides instruction-level disassembly with symbol tables, automatic
//! jump target labeling, and formatted listing output.

use std::collections::{HashMap, HashSet};

use crate::i4004::instruction_decode::{Instruction, InstructionDecoder};

/// CPU type selector for disassembly
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuType {
    I4004,
    I4040,
}

/// A disassembled instruction line with metadata
#[derive(Clone, Debug)]
pub struct DisasmLine {
    pub address: u16,
    pub bytes: Vec<u8>,
    pub mnemonic: String,
    pub operands: String,
    pub comment: Option<String>,
    pub is_jump_target: bool,
}

/// Disassembler for MCS-4 instruction set
#[derive(Clone, Debug)]
pub struct Disassembler {
    /// Reserved for 4040 dispatch (4040-specific opcode decoding not yet implemented)
    _cpu_type: CpuType,
    symbols: HashMap<u16, String>,
    comments: HashMap<u16, String>,
}

impl Disassembler {
    /// Create a new disassembler for the specified CPU type
    pub fn new(cpu_type: CpuType) -> Self {
        Self {
            _cpu_type: cpu_type,
            symbols: HashMap::new(),
            comments: HashMap::new(),
        }
    }

    /// Add a symbol (label) at an address
    pub fn add_symbol(&mut self, addr: u16, label: String) {
        self.symbols.insert(addr, label);
    }

    /// Add a comment for an address
    pub fn add_comment(&mut self, addr: u16, comment: String) {
        self.comments.insert(addr, comment);
    }

    /// Disassemble a single instruction at the given address
    pub fn disasm_one(&self, rom: &[u8], addr: u16) -> Result<DisasmLine, String> {
        if addr as usize >= rom.len() {
            return Err(format!("Address 0x{:03X} out of bounds", addr));
        }

        let mut decoder = InstructionDecoder::new();
        let first_byte = rom[addr as usize];
        decoder.decode_first(first_byte);

        // Handle two-byte instructions
        let (bytes, instruction) = if decoder.two_byte {
            let next_addr = (addr as usize) + 1;
            if next_addr >= rom.len() {
                return Err(format!("Two-byte instruction at 0x{:03X} extends past ROM", addr));
            }
            let second_byte = rom[next_addr];
            decoder.decode_second(second_byte);
            (vec![first_byte, second_byte], decoder.get_instruction())
        } else {
            (vec![first_byte], decoder.get_instruction())
        };

        let instruction = instruction.ok_or("Failed to decode instruction".to_string())?;
        let (mnemonic, operands) = self.format_instruction(instruction);

        Ok(DisasmLine {
            address: addr,
            bytes,
            mnemonic,
            operands,
            comment: self.comments.get(&addr).cloned(),
            is_jump_target: self.symbols.contains_key(&addr),
        })
    }

    /// Disassemble a range of addresses
    pub fn disasm_range(&self, rom: &[u8], start: u16, end: u16) -> Vec<DisasmLine> {
        let mut lines = Vec::new();
        let mut addr = start;

        while addr < end && (addr as usize) < rom.len() {
            match self.disasm_one(rom, addr) {
                Ok(line) => {
                    addr += line.bytes.len() as u16;
                    lines.push(line);
                }
                Err(_) => break,
            }
        }

        lines
    }

    /// Disassemble entire ROM
    pub fn disasm_all(&self, rom: &[u8]) -> Vec<DisasmLine> {
        self.disasm_range(rom, 0, rom.len() as u16)
    }

    /// Automatically label all jump targets in ROM
    pub fn auto_label(&mut self, rom: &[u8]) {
        let mut targets = HashSet::new();

        // Scan for jump instructions
        let lines = self.disasm_all(rom);
        for line in lines {
            if let Some(target_addr) = self.extract_jump_target(&line) {
                targets.insert(target_addr);
            }
        }

        // Add labels for all jump targets
        for target in targets {
            if !self.symbols.contains_key(&target) {
                let label = format!("LAB_{:03X}", target);
                self.add_symbol(target, label);
            }
        }
    }

    /// Extract jump target address from a disassembled line if applicable
    fn extract_jump_target(&self, line: &DisasmLine) -> Option<u16> {
        if line.mnemonic.starts_with("J") {
            // Parse operands like "0x123" or "LAB_123"
            if line.operands.starts_with("0x") {
                if let Ok(addr) = u16::from_str_radix(&line.operands[2..], 16) {
                    return Some(addr);
                }
            } else if line.operands.starts_with("LAB_") {
                if let Ok(addr) = u16::from_str_radix(&line.operands[4..], 16) {
                    return Some(addr);
                }
            }
        }
        None
    }

    /// Format disassembled lines as an assembly listing
    pub fn format_listing(&self, lines: &[DisasmLine]) -> String {
        let mut output = String::new();
        output.push_str("Addr  Bytes       Mnemonic Operands\n");
        output.push_str("----  ----------  -------- --------\n");

        for line in lines {
            // Address
            output.push_str(&format!("{:04X}  ", line.address));

            // Bytes
            let bytes_str = line
                .bytes
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" ");
            output.push_str(&format!("{:<11} ", bytes_str));

            // Mnemonic
            output.push_str(&format!("{:<8} ", line.mnemonic));

            // Operands
            output.push_str(&line.operands);

            // Comment
            if let Some(comment) = &line.comment {
                output.push_str(&format!(" ; {}", comment));
            }

            output.push('\n');
        }

        output
    }

    /// Format an instruction into mnemonic and operands
    fn format_instruction(&self, instruction: Instruction) -> (String, String) {
        match instruction {
            // Machine Control
            Instruction::Nop => ("NOP".to_string(), String::new()),

            // Conditional Jump (2-byte)
            Instruction::Jcn { condition, addr_low } => {
                let cond_str = self.format_condition(condition);
                ("JCN".to_string(), format!("{}, 0x{:03X}", cond_str, addr_low))
            }

            // Register Pair Operations
            Instruction::Fim { pair, data } => ("FIM".to_string(), format!("P{}, 0x{:02X}", pair, data)),
            Instruction::Src { pair } => ("SRC".to_string(), format!("P{}", pair)),
            Instruction::Fin { pair } => ("FIN".to_string(), format!("P{}", pair)),
            Instruction::Jin { pair } => ("JIN".to_string(), format!("P{}", pair)),

            // Unconditional Jumps (2-byte)
            Instruction::Jun { addr_high, addr_low } => {
                let addr = ((addr_high as u16) << 8) | (addr_low as u16);
                ("JUN".to_string(), format!("0x{:03X}", addr))
            }
            Instruction::Jms { addr_high, addr_low } => {
                let addr = ((addr_high as u16) << 8) | (addr_low as u16);
                ("JMS".to_string(), format!("0x{:03X}", addr))
            }
            Instruction::Isz { reg, addr_low } => ("ISZ".to_string(), format!("R{}, 0x{:02X}", reg, addr_low)),

            // Index Register Operations
            Instruction::Inc { reg } => ("INC".to_string(), format!("R{}", reg)),
            Instruction::Add { reg } => ("ADD".to_string(), format!("R{}", reg)),
            Instruction::Sub { reg } => ("SUB".to_string(), format!("R{}", reg)),
            Instruction::Ld { reg } => ("LD".to_string(), format!("R{}", reg)),
            Instruction::Xch { reg } => ("XCH".to_string(), format!("R{}", reg)),
            Instruction::Bbl { data } => ("BBL".to_string(), format!("0x{:X}", data)),

            // Immediate Operations
            Instruction::Ldm { data } => ("LDM".to_string(), format!("0x{:X}", data)),

            // I/O and RAM Control
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

            // Accumulator Group
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

            Instruction::Invalid { opcode } => ("???".to_string(), format!("0x{:02X}", opcode)),
        }
    }

    /// Format JCN condition code
    fn format_condition(&self, condition: u8) -> String {
        match condition {
            0x0 => "JNT".to_string(), // Jump if test pin = 0
            0x1 => "T".to_string(),   // Jump if test pin = 1
            0x2 => "JNZ".to_string(), // Jump if accumulator != 0
            0x3 => "JZ".to_string(),  // Jump if accumulator == 0
            0x4 => "JNC".to_string(), // Jump if carry = 0
            0x5 => "JC".to_string(),  // Jump if carry = 1
            0x6 => "JZC".to_string(), // Jump if A==0 or carry = 1
            0x7 => "JNZ".to_string(), // Jump if A != 0 and carry = 0
            0x8 => "T!".to_string(),  // Jump if test pin = 0 (invert)
            0x9 => "!T".to_string(),  // Jump if test pin = 1 (invert)
            0xA => "A!".to_string(),  // Jump if A != 0 (invert)
            0xB => "!A".to_string(),  // Jump if A == 0 (invert)
            0xC => "C!".to_string(),  // Jump if carry = 0 (invert)
            0xD => "!C".to_string(),  // Jump if carry = 1 (invert)
            0xE => "CZ!".to_string(), // Jump if C==0 and A==0 (invert)
            0xF => "!ZC".to_string(), // Jump if A != 0 or carry = 1 (invert)
            _ => format!("COND{}", condition),
        }
    }
}

/// Cached disassembly for efficient PC-relative lookup.
///
/// Caches the full disassembly of a ROM image and provides O(1) lookup
/// for windowed views around the current PC. The cache is invalidated
/// when the ROM contents change (detected by length + first/last byte check).
#[derive(Clone, Debug)]
pub struct DisasmCache {
    /// All disassembled lines, sorted by address
    lines: Vec<DisasmLine>,
    /// Address-to-index map for O(1) lookup
    addr_to_idx: HashMap<u16, usize>,
    /// Fingerprint of the ROM used to build this cache (len, first, last bytes)
    fingerprint: (usize, u8, u8),
}

impl DisasmCache {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            addr_to_idx: HashMap::new(),
            fingerprint: (0, 0, 0),
        }
    }

    /// Rebuild the cache from ROM data. Only rebuilds if ROM has changed.
    pub fn rebuild_if_needed(&mut self, disasm: &Disassembler, rom: &[u8]) {
        let fp = Self::make_fingerprint(rom);
        if fp == self.fingerprint && !self.lines.is_empty() {
            return; // Cache is still valid
        }

        self.fingerprint = fp;
        self.lines = disasm.disasm_range(rom, 0, rom.len() as u16);
        self.addr_to_idx.clear();
        for (i, line) in self.lines.iter().enumerate() {
            self.addr_to_idx.insert(line.address, i);
        }
    }

    /// Return a window of disassembled lines centered on the given PC.
    ///
    /// `before` and `after` control how many lines before/after the PC to include.
    /// Returns `(lines, pc_index)` where `pc_index` is the index of the PC line
    /// within the returned slice, or None if the PC address is not found.
    pub fn window(&self, pc: u16, before: usize, after: usize) -> (&[DisasmLine], Option<usize>) {
        if self.lines.is_empty() {
            return (&[], None);
        }

        let pc_idx = match self.addr_to_idx.get(&pc) {
            Some(&idx) => idx,
            None => {
                // PC not at an instruction boundary -- find nearest
                match self.lines.binary_search_by_key(&pc, |l| l.address) {
                    Ok(idx) => idx,
                    Err(idx) => idx.saturating_sub(1),
                }
            }
        };

        let start = pc_idx.saturating_sub(before);
        let end = (pc_idx + after + 1).min(self.lines.len());
        let pc_offset = pc_idx - start;

        (&self.lines[start..end], Some(pc_offset))
    }

    /// Get all cached lines.
    pub fn all_lines(&self) -> &[DisasmLine] {
        &self.lines
    }

    /// Number of cached instructions.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    fn make_fingerprint(rom: &[u8]) -> (usize, u8, u8) {
        let first = rom.first().copied().unwrap_or(0);
        let last = rom.last().copied().unwrap_or(0);
        (rom.len(), first, last)
    }
}

impl Default for DisasmCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disasm_nop() {
        let disasm = Disassembler::new(CpuType::I4004);
        let rom = vec![0x00];
        let line = disasm.disasm_one(&rom, 0).expect("NOP disasm");
        assert_eq!(line.mnemonic, "NOP");
        assert_eq!(line.operands, "");
        assert_eq!(line.address, 0);
        assert_eq!(line.bytes.len(), 1);
    }

    #[test]
    fn test_disasm_ldm() {
        let disasm = Disassembler::new(CpuType::I4004);
        let rom = vec![0xD5]; // LDM 5
        let line = disasm.disasm_one(&rom, 0).expect("LDM disasm");
        assert_eq!(line.mnemonic, "LDM");
        assert_eq!(line.operands, "0x5");
    }

    #[test]
    fn test_disasm_two_byte_jim() {
        let disasm = Disassembler::new(CpuType::I4004);
        let rom = vec![0x40, 0xAB]; // JUN 0x0AB
        let line = disasm.disasm_one(&rom, 0).expect("JUN disasm");
        assert_eq!(line.mnemonic, "JUN");
        assert_eq!(line.operands, "0x0AB");
        assert_eq!(line.bytes.len(), 2);
    }

    #[test]
    fn test_auto_label() {
        let mut disasm = Disassembler::new(CpuType::I4004);
        let rom = vec![
            0x40, 0x05, // JUN 0x005
            0x00, // NOP
            0x00, // NOP
            0x00, // NOP
        ];
        disasm.auto_label(&rom);
        assert!(disasm.symbols.contains_key(&0x005));
    }

    #[test]
    fn test_disasm_range() {
        let disasm = Disassembler::new(CpuType::I4004);
        let rom = vec![0x00, 0xD5, 0xF0]; // NOP, LDM, CLB
        let lines = disasm.disasm_range(&rom, 0, 3);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].mnemonic, "NOP");
        assert_eq!(lines[1].mnemonic, "LDM");
        assert_eq!(lines[2].mnemonic, "CLB");
    }

    #[test]
    fn test_format_condition() {
        let disasm = Disassembler::new(CpuType::I4004);
        assert_eq!(disasm.format_condition(0x0), "JNT");
        assert_eq!(disasm.format_condition(0x1), "T");
        assert_eq!(disasm.format_condition(0x4), "JNC");
        assert_eq!(disasm.format_condition(0x5), "JC");
    }

    #[test]
    fn test_format_listing() {
        let disasm = Disassembler::new(CpuType::I4004);
        let rom = vec![0x00, 0xD5];
        let lines = disasm.disasm_range(&rom, 0, 2);
        let listing = disasm.format_listing(&lines);
        assert!(listing.contains("NOP"));
        assert!(listing.contains("LDM"));
    }

    // --- DisasmCache tests ---

    #[test]
    fn cache_rebuild_and_lookup() {
        let disasm = Disassembler::new(CpuType::I4004);
        let rom = vec![0x00, 0xD5, 0xF2, 0x00]; // NOP, LDM 5, IAC, NOP
        let mut cache = DisasmCache::new();

        assert!(cache.is_empty());
        cache.rebuild_if_needed(&disasm, &rom);
        assert!(!cache.is_empty());
        assert_eq!(cache.len(), 4);
    }

    #[test]
    fn cache_window_centered_on_pc() {
        let disasm = Disassembler::new(CpuType::I4004);
        let rom = vec![0x00, 0xD5, 0xF2, 0xF0, 0x00]; // NOP, LDM, IAC, CLB, NOP
        let mut cache = DisasmCache::new();
        cache.rebuild_if_needed(&disasm, &rom);

        // Window around address 2 (IAC), 1 before, 1 after
        let (window, pc_idx) = cache.window(2, 1, 1);
        assert_eq!(pc_idx, Some(1)); // PC is at index 1 in the window
        assert_eq!(window.len(), 3); // LDM, IAC, CLB
        assert_eq!(window[0].address, 1); // LDM at addr 1
        assert_eq!(window[1].address, 2); // IAC at addr 2
        assert_eq!(window[2].address, 3); // CLB at addr 3
    }

    #[test]
    fn cache_window_at_start() {
        let disasm = Disassembler::new(CpuType::I4004);
        let rom = vec![0x00, 0xD5, 0xF2];
        let mut cache = DisasmCache::new();
        cache.rebuild_if_needed(&disasm, &rom);

        // Window at address 0 (start), 5 before (saturates), 1 after
        let (window, pc_idx) = cache.window(0, 5, 1);
        assert_eq!(pc_idx, Some(0)); // PC is at index 0
        assert_eq!(window.len(), 2); // NOP, LDM (only 2 lines from start)
    }

    #[test]
    fn cache_window_at_end() {
        let disasm = Disassembler::new(CpuType::I4004);
        let rom = vec![0x00, 0xD5, 0xF2];
        let mut cache = DisasmCache::new();
        cache.rebuild_if_needed(&disasm, &rom);

        // Window at last address, 1 before, 5 after (saturates)
        let (window, pc_idx) = cache.window(2, 1, 5);
        assert_eq!(pc_idx, Some(1)); // PC is at index 1
        assert_eq!(window.len(), 2); // LDM, IAC
    }

    #[test]
    fn cache_skips_rebuild_for_same_rom() {
        let disasm = Disassembler::new(CpuType::I4004);
        let rom = vec![0x00, 0xD5];
        let mut cache = DisasmCache::new();

        cache.rebuild_if_needed(&disasm, &rom);
        let first_len = cache.len();

        // Second rebuild with same ROM should be a no-op
        cache.rebuild_if_needed(&disasm, &rom);
        assert_eq!(cache.len(), first_len);
    }

    #[test]
    fn cache_rebuilds_for_changed_rom() {
        let disasm = Disassembler::new(CpuType::I4004);
        let rom1 = vec![0x00, 0xD5];
        let rom2 = vec![0x00, 0xD5, 0xF2]; // Different length

        let mut cache = DisasmCache::new();
        cache.rebuild_if_needed(&disasm, &rom1);
        assert_eq!(cache.len(), 2);

        cache.rebuild_if_needed(&disasm, &rom2);
        assert_eq!(cache.len(), 3); // Rebuilt with new ROM
    }

    #[test]
    fn cache_handles_two_byte_instructions() {
        let disasm = Disassembler::new(CpuType::I4004);
        // JUN 0x005 (2 bytes) + NOP + NOP + NOP + NOP
        let rom = vec![0x40, 0x05, 0x00, 0x00, 0x00, 0x00];
        let mut cache = DisasmCache::new();
        cache.rebuild_if_needed(&disasm, &rom);

        // JUN is 2 bytes, then 4 NOPs = 5 instructions
        assert_eq!(cache.len(), 5);

        // Address 1 is the second byte of JUN, not a separate instruction
        // Window at addr 0 should show JUN
        let (window, pc_idx) = cache.window(0, 0, 2);
        assert_eq!(pc_idx, Some(0));
        assert_eq!(window[0].mnemonic, "JUN");
        assert_eq!(window[0].bytes.len(), 2);
    }

    #[test]
    fn cache_empty_rom() {
        let disasm = Disassembler::new(CpuType::I4004);
        let mut cache = DisasmCache::new();
        cache.rebuild_if_needed(&disasm, &[]);
        assert!(cache.is_empty());

        let (window, pc_idx) = cache.window(0, 5, 5);
        assert!(window.is_empty());
        assert_eq!(pc_idx, None);
    }
}
