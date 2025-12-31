//! 4040 CPU core scaffold integrating stack and interrupts.

mod registers;
mod stack;
mod interrupt;
mod instruction_decode;

use registers::RegFile;
use stack::CallStack;
use interrupt::InterruptCtrl;

use crate::i4040::instruction_decode::decode_ext as decode_4040;
// use mcs4_bus::BusCycle; // unused until tick() implemented

#[derive(Default)]
pub struct I4040 {
    pub regs: RegFile,
    pub acc: u8,
    pub carry: bool,
    pub pc: u16,
    pub stack: CallStack,
    pub intr: InterruptCtrl,
    pub halted: bool,
}

impl I4040 {
    pub fn new() -> Self { Self::default() }

    /// Execute one instruction boundary: handle pending interrupts and HLT.
    pub fn step(&mut self) {
        if self.halted { return; }
        if let Some(vec) = self.intr.service(self.current_src()) {
            let _ = self.stack.push(self.pc);
            self.pc = vec;
        }
        // Minimal executor: handle control ops fetched from a byte at PC (stub)
        let opcode: u8 = 0; // TODO fetch
        if let Some(op) = decode_4040(opcode) {
            use crate::i4040::instruction_decode::Opcode4040 as Op;
            match op {
                Op::Hlt => self.hlt(),
                Op::Db0 => self.regs.db0(),
                Op::Db1 => self.regs.db1(),
                Op::Ein => self.intr.ein(),
                Op::Din => self.intr.din(),
                _ => {}
            }
        }
    }

    #[inline]
    fn current_src(&self) -> u8 {
        // SRC register encoding from current pair selection (placeholder)
        0
    }

    #[inline]
    pub fn hlt(&mut self) { self.halted = true; }
    #[inline]
    pub fn resume(&mut self) { self.halted = false; }
}

#[cfg(test)]
mod tests {
    use super::I4040;

    #[test]
    fn interrupt_vectors_and_bbs_restore() {
        let mut cpu = I4040::new();
        cpu.pc = 0x100;
        cpu.intr.ein();
        cpu.intr.request();
        cpu.step();
        assert_eq!(cpu.pc, 0x003);
        // Simulate BBS restore
        let saved = cpu.intr.bbs_restore();
        assert_eq!(saved, 0);
    }
}
