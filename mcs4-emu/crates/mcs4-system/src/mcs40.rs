//! MCS-40 System (4040-based) - stub

use mcs4_chips::i4040::I4040;

/// Complete MCS-40 system (stub)
pub struct Mcs40System {
    pub cpu: I4040,
}

impl Mcs40System {
    pub fn new() -> Self {
        Self { cpu: I4040::new() }
    }
}

impl Default for Mcs40System {
    fn default() -> Self {
        Self::new()
    }
}
