//! 4004 Timing and I/O Control

/// Timing and I/O controller for the 4004
#[derive(Clone, Debug, Default)]
pub struct TimingIo {
    // Placeholder for timing state
    _state: u8,
}

impl TimingIo {
    pub fn new() -> Self {
        Self::default()
    }
}
