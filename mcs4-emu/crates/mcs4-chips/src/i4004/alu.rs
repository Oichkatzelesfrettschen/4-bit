//! 4004 ALU (Arithmetic Logic Unit)

/// 4-bit ALU with accumulator and carry
#[derive(Clone, Debug)]
pub struct Alu {
    /// 4-bit accumulator
    acc: u8,

    /// Carry flag
    carry: bool,

    /// Temporary register (for two-step operations)
    temp: u8,
}

impl Alu {
    pub fn new() -> Self {
        Self {
            acc: 0,
            carry: false,
            temp: 0,
        }
    }

    /// Get accumulator value
    pub fn accumulator(&self) -> u8 {
        self.acc & 0x0F
    }

    /// Set accumulator value
    pub fn set_accumulator(&mut self, value: u8) {
        self.acc = value & 0x0F;
    }

    /// Get carry flag
    pub fn carry(&self) -> bool {
        self.carry
    }

    /// Set carry flag
    pub fn set_carry(&mut self, carry: bool) {
        self.carry = carry;
    }

    /// Clear accumulator
    pub fn clb(&mut self) {
        self.acc = 0;
        self.carry = false;
    }

    /// Complement accumulator
    pub fn cma(&mut self) {
        self.acc = (!self.acc) & 0x0F;
    }

    /// Complement carry
    pub fn cmc(&mut self) {
        self.carry = !self.carry;
    }

    /// Set carry
    pub fn stc(&mut self) {
        self.carry = true;
    }

    /// Increment accumulator
    pub fn iac(&mut self) {
        let result = self.acc.wrapping_add(1);
        self.carry = result > 0x0F;
        self.acc = result & 0x0F;
    }

    /// Decrement accumulator
    pub fn dac(&mut self) {
        let result = self.acc.wrapping_sub(1);
        self.carry = self.acc != 0; // Borrow (inverted)
        self.acc = result & 0x0F;
    }

    /// Rotate left through carry
    pub fn ral(&mut self) {
        let new_carry = (self.acc & 0x08) != 0;
        self.acc = ((self.acc << 1) | (self.carry as u8)) & 0x0F;
        self.carry = new_carry;
    }

    /// Rotate right through carry
    pub fn rar(&mut self) {
        let new_carry = (self.acc & 0x01) != 0;
        self.acc = ((self.acc >> 1) | ((self.carry as u8) << 3)) & 0x0F;
        self.carry = new_carry;
    }

    /// Add with carry
    pub fn add(&mut self, value: u8) {
        let result = (self.acc as u16) + (value as u16) + (self.carry as u16);
        self.carry = result > 0x0F;
        self.acc = (result & 0x0F) as u8;
    }

    /// Subtract with borrow
    pub fn sub(&mut self, value: u8) {
        // 4004 subtract: ACC = ACC + ~value + carry
        let complement = (!value) & 0x0F;
        let result = (self.acc as u16) + (complement as u16) + (self.carry as u16);
        self.carry = result > 0x0F;
        self.acc = (result & 0x0F) as u8;
    }

    /// Load accumulator
    pub fn load(&mut self, value: u8) {
        self.acc = value & 0x0F;
    }

    /// Exchange accumulator with temp
    pub fn xch(&mut self, value: u8) -> u8 {
        let old = self.acc;
        self.acc = value & 0x0F;
        old
    }

    /// Decimal adjust accumulator
    pub fn daa(&mut self) {
        if self.carry || self.acc > 9 {
            let result = self.acc + 6;
            if result > 0x0F {
                self.carry = true;
            }
            self.acc = result & 0x0F;
        }
    }

    /// Transfer carry to accumulator and clear
    pub fn tcc(&mut self) {
        self.acc = self.carry as u8;
        self.carry = false;
    }

    /// Keyboard process (convert to BCD)
    pub fn kbp(&mut self) {
        self.acc = match self.acc {
            0x00 => 0x00,
            0x01 => 0x01,
            0x02 => 0x02,
            0x04 => 0x03,
            0x08 => 0x04,
            _ => 0x0F, // Invalid
        };
    }
}

impl Default for Alu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let mut alu = Alu::new();
        alu.set_accumulator(5);
        alu.add(3);
        assert_eq!(alu.accumulator(), 8);
        assert!(!alu.carry());

        // Test carry
        alu.set_accumulator(15);
        alu.add(1);
        assert_eq!(alu.accumulator(), 0);
        assert!(alu.carry());
    }

    #[test]
    fn test_rotate() {
        let mut alu = Alu::new();
        alu.set_accumulator(0b1010);
        alu.set_carry(false);

        alu.ral();
        assert_eq!(alu.accumulator(), 0b0100);
        assert!(alu.carry());
    }

    #[test]
    fn test_kbp() {
        let mut alu = Alu::new();

        alu.set_accumulator(0x01);
        alu.kbp();
        assert_eq!(alu.accumulator(), 1);

        alu.set_accumulator(0x08);
        alu.kbp();
        assert_eq!(alu.accumulator(), 4);

        alu.set_accumulator(0x03); // Invalid
        alu.kbp();
        assert_eq!(alu.accumulator(), 0x0F);
    }
}
