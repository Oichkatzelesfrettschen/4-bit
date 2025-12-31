//! 4040 call stack (7-level) with push/pop invariants.

#[derive(Default, Debug, Clone)]
pub struct CallStack {
    stack: [u16; 7],
    sp: usize, // points to next free slot (0..7)
}

impl CallStack {
    pub fn new() -> Self { Self::default() }
    #[inline]
    pub fn depth(&self) -> usize { self.sp }
    #[inline]
    pub fn is_full(&self) -> bool { self.sp >= 7 }
    #[inline]
    pub fn is_empty(&self) -> bool { self.sp == 0 }

    pub fn push(&mut self, addr: u16) -> Result<(), &'static str> {
        if self.is_full() { return Err("stack_overflow"); }
        self.stack[self.sp] = addr & 0x0FFF; // 12-bit PC
        self.sp += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Result<u16, &'static str> {
        if self.is_empty() { return Err("stack_underflow"); }
        self.sp -= 1;
        Ok(self.stack[self.sp])
    }

    pub fn peek(&self) -> Option<u16> { if self.is_empty() { None } else { Some(self.stack[self.sp-1]) } }
}

#[cfg(test)]
mod tests {
    use super::CallStack;

    #[test]
    fn push_pop_invariants() {
        let mut s = CallStack::new();
        for i in 0..7 { assert!(s.push(i).is_ok()); }
        assert!(s.is_full());
        assert!(s.push(7).is_err());
        for i in (0..7).rev() { assert_eq!(s.pop().unwrap(), i); }
        assert!(s.is_empty());
        assert!(s.pop().is_err());
    }
}
