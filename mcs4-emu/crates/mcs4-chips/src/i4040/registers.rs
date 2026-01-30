//! 4040 Register File
//!
//! Backward-compatible extension of 4004 registers with:
//! - 24 4-bit index registers (R0-R23) with bank switching
//! - 7-level stack (vs 3-level in 4004)
//! - Interrupt support with SRC save/restore

/// Register file for the 4040
///
/// Contains:
/// - 24 4-bit index registers (R0-R23)
///   - Bank 0 (DB0): R0-R7 are primary, R8-R15 always accessible
///   - Bank 1 (DB1): R0-R7 map to R16-R23 (shadow), R8-R15 always accessible
/// - 12-bit program counter
/// - 7-level stack (12-bit entries)
/// - Interrupt controller state
#[derive(Clone, Debug)]
pub struct Registers {
    /// Index registers R0-R23 (4-bit each)
    index: [u8; 24],

    /// Program counter (12-bit)
    pc: u16,

    /// Stack (7 levels of 12-bit addresses)
    stack: [u16; 7],

    /// Stack pointer (0-6, wraps)
    sp: u8,

    /// Register bank selector (0 or 1)
    /// Bank 0: R0-R7 = index[0..8]
    /// Bank 1: R0-R7 = index[16..24]
    /// R8-R15 always map to index[8..16]
    bank: u8,

    /// Saved SRC register value (for interrupt restore)
    src_save: u8,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            index: [0; 24],
            pc: 0,
            stack: [0; 7],
            sp: 0,
            bank: 0,
            src_save: 0,
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

    /// Get current register bank (0 or 1)
    pub fn bank(&self) -> u8 {
        self.bank
    }

    /// Set register bank (DB0 or DB1 instruction)
    pub fn set_bank(&mut self, bank: u8) {
        self.bank = bank & 0x01;
    }

    /// Get saved SRC register (for interrupt restore)
    pub fn src_save(&self) -> u8 {
        self.src_save
    }

    /// Save SRC register (on interrupt entry)
    pub fn save_src(&mut self, value: u8) {
        self.src_save = value;
    }

    /// Map logical register index (0-23) to physical index based on bank
    ///
    /// Bank 0: R0-R7  -> index[0..8],  R8-R15 -> index[8..16]
    /// Bank 1: R0-R7  -> index[16..24], R8-R15 -> index[8..16]
    /// R16-R23 never directly accessible (only via bank switching)
    fn physical_index(&self, logical: u8) -> usize {
        let logical = (logical & 0x0F) as usize;
        if logical < 8 {
            // R0-R7: affected by bank bit
            if self.bank == 0 {
                logical
            } else {
                logical + 16
            }
        } else {
            // R8-R15: always map to index[8..16]
            logical
        }
    }

    /// Get index register (R0-R15, bank-aware for R0-R7)
    pub fn get_r(&self, index: u8) -> u8 {
        let phys = self.physical_index(index);
        self.index[phys] & 0x0F
    }

    /// Set index register (bank-aware for R0-R7)
    pub fn set_r(&mut self, index: u8, value: u8) {
        let phys = self.physical_index(index);
        self.index[phys] = value & 0x0F;
    }

    /// Get register pair as 8-bit value
    /// P0 = R0:R1, P1 = R2:R3, etc.
    pub fn get_pair(&self, pair: u8) -> u8 {
        let base = (pair & 0x07) * 2;
        let high = self.get_r(base);
        let low = self.get_r(base + 1);
        (high << 4) | low
    }

    /// Set register pair
    pub fn set_pair(&mut self, pair: u8, value: u8) {
        let base = (pair & 0x07) * 2;
        self.set_r(base, (value >> 4) & 0x0F);
        self.set_r(base + 1, value & 0x0F);
    }

    /// Push return address to stack and set new PC (for JMS)
    pub fn call(&mut self, return_addr: u16, target: u16) {
        self.stack[self.sp as usize] = return_addr & 0x0FFF;
        self.sp = (self.sp + 1) % 7;
        self.pc = target & 0x0FFF;
    }

    /// Pop PC from stack (for BBL)
    pub fn ret(&mut self) {
        self.sp = if self.sp == 0 { 6 } else { self.sp - 1 };
        self.pc = self.stack[self.sp as usize];
    }

    /// Pop PC from stack and restore SRC (for BBS - interrupt return)
    pub fn ret_from_interrupt(&mut self) -> u8 {
        self.sp = if self.sp == 0 { 6 } else { self.sp - 1 };
        self.pc = self.stack[self.sp as usize];
        self.src_save
    }

    /// Push return address onto stack (for interrupt service)
    pub fn push_return(&mut self, addr: u16) {
        self.stack[self.sp as usize] = addr & 0x0FFF;
        self.sp = (self.sp + 1) % 7;
    }

    /// Get current stack depth (number of entries pushed)
    pub fn stack_depth(&self) -> u8 {
        self.sp
    }

    /// Check if stack is full (7 levels occupied)
    pub fn stack_full(&self) -> bool {
        self.sp == 0 // After 7 pushes, sp wraps to 0
    }

    /// Get stack entry at level (0 = most recent, 6 = oldest)
    pub fn stack_at(&self, level: u8) -> u16 {
        if level >= 7 {
            return 0;
        }
        let idx = if self.sp > level {
            self.sp - level - 1
        } else {
            7 + self.sp - level - 1
        };
        self.stack[idx as usize % 7]
    }

    /// Increment register pair (for ISZ)
    pub fn inc_pair(&mut self, pair: u8) -> bool {
        let value = self.get_pair(pair).wrapping_add(1);
        self.set_pair(pair, value);
        value == 0
    }

    /// Increment single register, return true if wrapped to 0
    pub fn inc_r(&mut self, index: u8) -> bool {
        let value = (self.get_r(index) + 1) & 0x0F;
        self.set_r(index, value);
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
    fn test_register_bank_switching() {
        let mut regs = Registers::new();

        // Bank 0: R0-R7 map to index[0..8]
        assert_eq!(regs.bank(), 0);
        regs.set_r(5, 0xA);
        assert_eq!(regs.get_r(5), 0xA);
        assert_eq!(regs.index[5], 0xA);

        // Switch to Bank 1: R0-R7 now map to index[16..24]
        regs.set_bank(1);
        assert_eq!(regs.bank(), 1);
        assert_eq!(regs.get_r(5), 0x0); // Different physical location
        regs.set_r(5, 0xB);
        assert_eq!(regs.get_r(5), 0xB);
        assert_eq!(regs.index[21], 0xB); // Physical index 16 + 5 = 21

        // Switch back to Bank 0: original value still there
        regs.set_bank(0);
        assert_eq!(regs.get_r(5), 0xA);
    }

    #[test]
    fn test_r8_r15_always_accessible() {
        let mut regs = Registers::new();

        // R8-R15 always map to index[8..16] regardless of bank
        regs.set_r(10, 0x7);
        assert_eq!(regs.get_r(10), 0x7);
        assert_eq!(regs.index[10], 0x7);

        regs.set_bank(1);
        assert_eq!(regs.get_r(10), 0x7); // Still the same
        assert_eq!(regs.index[10], 0x7);
    }

    #[test]
    fn test_pairs_with_bank_switching() {
        let mut regs = Registers::new();

        // P0 = R0:R1 in bank 0
        regs.set_bank(0);
        regs.set_pair(0, 0xAB);
        assert_eq!(regs.get_r(0), 0xA);
        assert_eq!(regs.get_r(1), 0xB);
        assert_eq!(regs.get_pair(0), 0xAB);

        // Switch to bank 1, P0 now different
        regs.set_bank(1);
        assert_eq!(regs.get_pair(0), 0x00);
        regs.set_pair(0, 0xCD);
        assert_eq!(regs.get_pair(0), 0xCD);

        // Switch back, original value preserved
        regs.set_bank(0);
        assert_eq!(regs.get_pair(0), 0xAB);
    }

    #[test]
    fn test_stack_7_levels() {
        let mut regs = Registers::new();

        // Push 7 addresses
        regs.set_pc(0x100);
        regs.call(0x101, 0x200);
        assert_eq!(regs.pc(), 0x200);
        assert_eq!(regs.stack_depth(), 1);

        regs.call(0x201, 0x300);
        assert_eq!(regs.stack_depth(), 2);

        regs.call(0x301, 0x400);
        assert_eq!(regs.stack_depth(), 3);

        regs.call(0x401, 0x500);
        assert_eq!(regs.stack_depth(), 4);

        regs.call(0x501, 0x600);
        assert_eq!(regs.stack_depth(), 5);

        regs.call(0x601, 0x700);
        assert_eq!(regs.stack_depth(), 6);

        regs.call(0x701, 0x800);
        assert_eq!(regs.pc(), 0x800);
        assert_eq!(regs.stack_depth(), 0); // Wrapped

        // Pop all 7 levels
        regs.ret();
        assert_eq!(regs.pc(), 0x701);

        regs.ret();
        assert_eq!(regs.pc(), 0x601);

        regs.ret();
        assert_eq!(regs.pc(), 0x501);

        regs.ret();
        assert_eq!(regs.pc(), 0x401);

        regs.ret();
        assert_eq!(regs.pc(), 0x301);

        regs.ret();
        assert_eq!(regs.pc(), 0x201);

        regs.ret();
        assert_eq!(regs.pc(), 0x101);
    }

    #[test]
    fn test_stack_overflow() {
        let mut regs = Registers::new();

        // Fill all 7 levels
        for i in 0..7 {
            regs.call(0x100 + i, 0x200 + i);
        }

        // After 7 calls, sp wraps to 0
        assert_eq!(regs.stack_depth(), 0);
        assert!(regs.stack_full());

        // 8th call overwrites oldest entry (index 0)
        regs.call(0x107, 0x207);
        assert_eq!(regs.stack_depth(), 1);

        // Oldest entry (0x100) was overwritten
        assert_eq!(regs.stack[0], 0x107);
    }

    #[test]
    fn test_src_save_restore() {
        let mut regs = Registers::new();

        // Save SRC value (simulating interrupt entry)
        regs.save_src(0x42);
        assert_eq!(regs.src_save(), 0x42);

        // Push return address (interrupt vectors)
        regs.call(0x123, 0x003);

        // Restore SRC on interrupt return
        let saved_src = regs.ret_from_interrupt();
        assert_eq!(saved_src, 0x42);
        assert_eq!(regs.pc(), 0x123);
    }

    #[test]
    fn test_stack_at() {
        let mut regs = Registers::new();

        regs.call(0x100, 0x200);
        regs.call(0x200, 0x300);
        regs.call(0x300, 0x400);

        assert_eq!(regs.stack_at(0), 0x300); // Most recent
        assert_eq!(regs.stack_at(1), 0x200);
        assert_eq!(regs.stack_at(2), 0x100); // Oldest
    }

    #[test]
    fn test_masking() {
        let mut regs = Registers::new();

        // 4-bit masking
        regs.set_r(3, 0xFF);
        assert_eq!(regs.get_r(3), 0x0F);

        // Bank masking
        regs.set_bank(0xFF);
        assert_eq!(regs.bank(), 0x01);
    }
}
