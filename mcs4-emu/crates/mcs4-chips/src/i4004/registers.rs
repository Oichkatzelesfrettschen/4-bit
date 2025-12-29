//! 4004 Register File

/// Register file for the 4004
///
/// Contains:
/// - 16 4-bit index registers (R0-R15), also addressable as 8 pairs (P0-P7)
/// - 12-bit program counter
/// - 3-level stack (12-bit entries)
#[derive(Clone, Debug)]
pub struct Registers {
    /// Index registers R0-R15 (4-bit each)
    index: [u8; 16],

    /// Program counter (12-bit)
    pc: u16,

    /// Stack (3 levels of 12-bit addresses)
    stack: [u16; 3],

    /// Stack pointer (0-2, wraps)
    sp: u8,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            index: [0; 16],
            pc: 0,
            stack: [0; 3],
            sp: 0,
        }
    }

    /// Get program counter
    pub fn pc(&self) -> u16 {
        self.pc & 0x0FFF
    }

    /// Set program counter
    pub fn set_pc(&mut self, addr: u16) {
        self.pc = addr & 0x0FFF;
    }

    /// Increment program counter
    pub fn increment_pc(&mut self) {
        self.pc = (self.pc + 1) & 0x0FFF;
    }

    /// Get index register (R0-R15)
    pub fn get_r(&self, index: u8) -> u8 {
        self.index[(index & 0x0F) as usize] & 0x0F
    }

    /// Set index register
    pub fn set_r(&mut self, index: u8, value: u8) {
        self.index[(index & 0x0F) as usize] = value & 0x0F;
    }

    /// Get register pair as 8-bit value
    /// P0 = R0:R1, P1 = R2:R3, etc.
    pub fn get_pair(&self, pair: u8) -> u8 {
        let base = (pair & 0x07) as usize * 2;
        let high = self.index[base] & 0x0F;
        let low = self.index[base + 1] & 0x0F;
        (high << 4) | low
    }

    /// Set register pair
    pub fn set_pair(&mut self, pair: u8, value: u8) {
        let base = (pair & 0x07) as usize * 2;
        self.index[base] = (value >> 4) & 0x0F;
        self.index[base + 1] = value & 0x0F;
    }

    /// Push PC to stack and set new PC (for JMS)
    pub fn call(&mut self, addr: u16) {
        self.stack[self.sp as usize] = self.pc;
        self.sp = (self.sp + 1) % 3;
        self.pc = addr & 0x0FFF;
    }

    /// Pop PC from stack (for BBL)
    pub fn ret(&mut self) {
        self.sp = if self.sp == 0 { 2 } else { self.sp - 1 };
        self.pc = self.stack[self.sp as usize];
    }

    /// Increment register pair (for ISZ)
    pub fn inc_pair(&mut self, pair: u8) -> bool {
        let value = self.get_pair(pair).wrapping_add(1);
        self.set_pair(pair, value);
        value == 0
    }

    /// Increment single register, return true if wrapped to 0
    pub fn inc_r(&mut self, index: u8) -> bool {
        let value = (self.index[(index & 0x0F) as usize] + 1) & 0x0F;
        self.index[(index & 0x0F) as usize] = value;
        value == 0
    }
}

impl Default for Registers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pc() {
        let mut regs = Registers::new();
        assert_eq!(regs.pc(), 0);

        regs.increment_pc();
        assert_eq!(regs.pc(), 1);

        regs.set_pc(0xFFF);
        regs.increment_pc();
        assert_eq!(regs.pc(), 0); // Wrap
    }

    #[test]
    fn test_index_registers() {
        let mut regs = Registers::new();

        regs.set_r(5, 0xA);
        assert_eq!(regs.get_r(5), 0xA);

        // Test masking
        regs.set_r(5, 0xFF);
        assert_eq!(regs.get_r(5), 0x0F);
    }

    #[test]
    fn test_pairs() {
        let mut regs = Registers::new();

        regs.set_pair(0, 0xAB);
        assert_eq!(regs.get_r(0), 0xA);
        assert_eq!(regs.get_r(1), 0xB);
        assert_eq!(regs.get_pair(0), 0xAB);
    }

    #[test]
    fn test_stack() {
        let mut regs = Registers::new();

        regs.set_pc(0x100);
        regs.call(0x200);
        assert_eq!(regs.pc(), 0x200);

        regs.call(0x300);
        assert_eq!(regs.pc(), 0x300);

        regs.ret();
        assert_eq!(regs.pc(), 0x200);

        regs.ret();
        assert_eq!(regs.pc(), 0x100);
    }
}
