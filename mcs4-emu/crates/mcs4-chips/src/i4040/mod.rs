//! 4040 CPU core scaffold integrating stack and interrupts.

mod registers;
mod stack;
mod interrupt;

use registers::RegFile;
use stack::CallStack;
use interrupt::InterruptCtrl;

#[derive(Default)]
pub struct I4040 {
    pub regs: RegFile,
    pub acc: u8,
    pub carry: bool,
    pub pc: u16,
    pub stack: CallStack,
    pub intr: InterruptCtl,
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
        // TODO: fetch/decode/execute here
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
