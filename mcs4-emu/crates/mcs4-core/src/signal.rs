//! Signal types and signal history tracking

use crate::timing::Time;
use smallvec::SmallVec;

/// Unique identifier for a signal in the simulation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SignalId(pub u32);

impl SignalId {
    pub const INVALID: SignalId = SignalId(u32::MAX);
}

/// Logic level of a digital signal
///
/// Note: The Intel 4004 uses negative logic (pMOS):
/// - Logic HIGH = Vss = 0V (ground)
/// - Logic LOW = Vdd = -15V
///
/// We abstract this to standard positive logic in the simulator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum SignalLevel {
    /// Logic low (0)
    Low = 0,
    /// Logic high (1)
    High = 1,
    /// High impedance (floating, not driven)
    #[default]
    Z = 2,
    /// Unknown/undefined (contention or uninitialized)
    X = 3,
}

impl SignalLevel {
    /// Returns true if the signal is a defined logic level (not Z or X)
    #[inline]
    pub fn is_defined(self) -> bool {
        matches!(self, SignalLevel::Low | SignalLevel::High)
    }

    /// Returns true if signal is logic high
    #[inline]
    pub fn is_high(self) -> bool {
        self == SignalLevel::High
    }

    /// Returns true if signal is logic low
    #[inline]
    pub fn is_low(self) -> bool {
        self == SignalLevel::Low
    }

    /// Logical NOT
    #[inline]
    pub fn invert(self) -> SignalLevel {
        match self {
            SignalLevel::Low => SignalLevel::High,
            SignalLevel::High => SignalLevel::Low,
            SignalLevel::Z => SignalLevel::X,
            SignalLevel::X => SignalLevel::X,
        }
    }

    /// Logical AND
    #[inline]
    pub fn and(self, other: SignalLevel) -> SignalLevel {
        match (self, other) {
            (SignalLevel::Low, _) | (_, SignalLevel::Low) => SignalLevel::Low,
            (SignalLevel::High, SignalLevel::High) => SignalLevel::High,
            _ => SignalLevel::X,
        }
    }

    /// Logical OR
    #[inline]
    pub fn or(self, other: SignalLevel) -> SignalLevel {
        match (self, other) {
            (SignalLevel::High, _) | (_, SignalLevel::High) => SignalLevel::High,
            (SignalLevel::Low, SignalLevel::Low) => SignalLevel::Low,
            _ => SignalLevel::X,
        }
    }

    /// Resolve bus contention between multiple drivers
    pub fn resolve(drivers: &[SignalLevel]) -> SignalLevel {
        let mut has_high = false;
        let mut has_low = false;

        for &level in drivers {
            match level {
                SignalLevel::High => has_high = true,
                SignalLevel::Low => has_low = true,
                SignalLevel::X => return SignalLevel::X,
                SignalLevel::Z => {}
            }
        }

        match (has_high, has_low) {
            (true, true) => SignalLevel::X,   // Bus fight!
            (true, false) => SignalLevel::High,
            (false, true) => SignalLevel::Low,
            (false, false) => SignalLevel::Z, // No drivers
        }
    }
}

impl From<bool> for SignalLevel {
    fn from(b: bool) -> Self {
        if b { SignalLevel::High } else { SignalLevel::Low }
    }
}

impl From<SignalLevel> for bool {
    fn from(s: SignalLevel) -> Self {
        s == SignalLevel::High
    }
}

/// A signal with transition history for waveform display
#[derive(Clone, Debug)]
pub struct Signal {
    /// Human-readable name (e.g., "PHI1", "D0", "SYNC")
    pub name: String,

    /// Current value
    pub current: SignalLevel,

    /// Transition history: (time, new_value)
    /// Uses SmallVec to avoid allocation for signals with few transitions
    history: SmallVec<[(Time, SignalLevel); 16]>,

    /// Maximum history length (for memory management)
    max_history: usize,
}

impl Signal {
    /// Create a new signal with the given name and initial value
    pub fn new(name: impl Into<String>, initial: SignalLevel) -> Self {
        Self {
            name: name.into(),
            current: initial,
            history: SmallVec::new(),
            max_history: 10_000,
        }
    }

    /// Create a signal with custom history limit
    pub fn with_history_limit(name: impl Into<String>, initial: SignalLevel, limit: usize) -> Self {
        Self {
            name: name.into(),
            current: initial,
            history: SmallVec::new(),
            max_history: limit,
        }
    }

    /// Update signal value at the given time
    pub fn update(&mut self, time: Time, value: SignalLevel) {
        if value != self.current {
            // Trim history if at limit
            if self.history.len() >= self.max_history {
                // Remove oldest quarter of history
                let remove_count = self.max_history / 4;
                self.history.drain(0..remove_count);
            }

            self.history.push((time, value));
            self.current = value;
        }
    }

    /// Get the signal value at a specific time
    pub fn value_at(&self, time: Time) -> SignalLevel {
        // Binary search for the latest transition before or at `time`
        match self.history.binary_search_by_key(&time, |&(t, _)| t) {
            Ok(idx) => self.history[idx].1,
            Err(0) => {
                // Before first transition - assume initial value
                if let Some(&(_, first_val)) = self.history.first() {
                    // Return opposite of first recorded value as "initial"
                    first_val.invert()
                } else {
                    self.current
                }
            }
            Err(idx) => self.history[idx - 1].1,
        }
    }

    /// Get history for waveform display
    pub fn history(&self) -> &[(Time, SignalLevel)] {
        &self.history
    }

    /// Get transitions in a time range for waveform display
    pub fn transitions_in_range(&self, start: Time, end: Time) -> Vec<(Time, SignalLevel)> {
        self.history
            .iter()
            .filter(|&&(t, _)| t >= start && t <= end)
            .copied()
            .collect()
    }

    /// Clear history (for reset)
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

/// 4-bit bus signal (common in MCS-4)
#[derive(Clone, Debug)]
pub struct Bus4 {
    pub bits: [Signal; 4],
}

impl Bus4 {
    pub fn new(name_prefix: &str) -> Self {
        Self {
            bits: [
                Signal::new(format!("{name_prefix}0"), SignalLevel::Z),
                Signal::new(format!("{name_prefix}1"), SignalLevel::Z),
                Signal::new(format!("{name_prefix}2"), SignalLevel::Z),
                Signal::new(format!("{name_prefix}3"), SignalLevel::Z),
            ],
        }
    }

    /// Get current 4-bit value
    pub fn value(&self) -> u8 {
        let mut v = 0u8;
        for (i, bit) in self.bits.iter().enumerate() {
            if bit.current == SignalLevel::High {
                v |= 1 << i;
            }
        }
        v
    }

    /// Update all bits at once
    pub fn update(&mut self, time: Time, value: u8) {
        for (i, bit) in self.bits.iter_mut().enumerate() {
            let level = if (value >> i) & 1 == 1 {
                SignalLevel::High
            } else {
                SignalLevel::Low
            };
            bit.update(time, level);
        }
    }

    /// Set all bits to high-Z
    pub fn tristate(&mut self, time: Time) {
        for bit in &mut self.bits {
            bit.update(time, SignalLevel::Z);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_logic() {
        assert_eq!(SignalLevel::High.invert(), SignalLevel::Low);
        assert_eq!(SignalLevel::Low.invert(), SignalLevel::High);

        assert_eq!(SignalLevel::High.and(SignalLevel::High), SignalLevel::High);
        assert_eq!(SignalLevel::High.and(SignalLevel::Low), SignalLevel::Low);
        assert_eq!(SignalLevel::Low.and(SignalLevel::Low), SignalLevel::Low);

        assert_eq!(SignalLevel::High.or(SignalLevel::Low), SignalLevel::High);
        assert_eq!(SignalLevel::Low.or(SignalLevel::Low), SignalLevel::Low);
    }

    #[test]
    fn test_bus_resolution() {
        // No drivers
        assert_eq!(SignalLevel::resolve(&[SignalLevel::Z, SignalLevel::Z]), SignalLevel::Z);

        // Single driver
        assert_eq!(SignalLevel::resolve(&[SignalLevel::High, SignalLevel::Z]), SignalLevel::High);

        // Bus fight
        assert_eq!(SignalLevel::resolve(&[SignalLevel::High, SignalLevel::Low]), SignalLevel::X);
    }

    #[test]
    fn test_signal_history() {
        let mut sig = Signal::new("test", SignalLevel::Low);

        sig.update(100, SignalLevel::High);
        sig.update(200, SignalLevel::Low);
        sig.update(300, SignalLevel::High);

        assert_eq!(sig.value_at(50), SignalLevel::High); // Before first transition
        assert_eq!(sig.value_at(150), SignalLevel::High);
        assert_eq!(sig.value_at(250), SignalLevel::Low);
        assert_eq!(sig.value_at(350), SignalLevel::High);
    }

    #[test]
    fn test_bus4() {
        let mut bus = Bus4::new("D");
        assert_eq!(bus.value(), 0);

        bus.update(100, 0b1010);
        assert_eq!(bus.value(), 0b1010);

        bus.update(200, 0b0101);
        assert_eq!(bus.value(), 0b0101);
    }
}
