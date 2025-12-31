// Disassembler core scaffolding
pub enum CpuType { I4004, I4040 }

pub struct DisasmLine {
    pub address: u16,
    pub bytes: Vec<u8>,
    pub mnemonic: String,
    pub operands: String,
}

pub struct Disassembler { pub cpu_type: CpuType }

impl Disassembler {
    pub fn new(cpu_type: CpuType) -> Self { Self { cpu_type } }
    pub fn disasm_one(&self, rom: &[u8], addr: u16) -> DisasmLine {
        let op = rom.get(addr as usize).copied().unwrap_or(0x00);
        DisasmLine { address: addr, bytes: vec![op], mnemonic: format!("OP{:02X}", op), operands: String::new() }
    }
    pub fn disasm_range(&self, rom: &[u8], start: u16, end: u16) -> Vec<DisasmLine> {
        (start..=end).map(|a| self.disasm_one(rom, a)).collect()
    }
}
