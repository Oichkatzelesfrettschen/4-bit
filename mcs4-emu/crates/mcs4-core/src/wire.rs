//! Wire and net modeling for interconnect delay simulation

use crate::signal::SignalId;
use crate::timing::{Delay, PICOSECOND};

/// Fanout information for a net
#[derive(Clone, Debug, Default)]
pub struct Fanout {
    /// Number of gate inputs this wire drives
    pub count: usize,
    /// Total capacitive load in femtofarads (fF)
    pub capacitance: f64,
}

impl Fanout {
    pub fn new(count: usize) -> Self {
        // Typical gate input capacitance ~10 fF for this era
        Self {
            count,
            capacitance: count as f64 * 10.0,
        }
    }

    /// Calculate wire delay based on fanout
    pub fn delay(&self) -> Delay {
        // Simple RC delay model: delay proportional to fanout
        // ~0.5 ns per fanout for this technology
        (self.count as Delay) * 500 * PICOSECOND
    }
}

/// A wire connecting gate outputs to gate inputs
#[derive(Clone, Debug)]
pub struct Wire {
    /// Source signal (driver)
    pub source: SignalId,

    /// Destination signals (loads)
    pub destinations: Vec<SignalId>,

    /// Calculated fanout information
    pub fanout: Fanout,

    /// Wire resistance (ohms) - for future transistor-level sim
    pub resistance: f64,

    /// Wire capacitance (femtofarads) - for future transistor-level sim
    pub capacitance: f64,
}

impl Wire {
    pub fn new(source: SignalId, destinations: Vec<SignalId>) -> Self {
        let fanout = Fanout::new(destinations.len());
        Self {
            source,
            destinations,
            fanout,
            resistance: 10.0,   // Default 10 ohms
            capacitance: 5.0,   // Default 5 fF base
        }
    }

    /// Propagation delay through this wire
    pub fn delay(&self) -> Delay {
        self.fanout.delay()
    }
}

/// A net groups multiple wires that are electrically connected
#[derive(Clone, Debug)]
pub struct Net {
    /// Human-readable name
    pub name: String,

    /// All signal IDs that are part of this net
    pub signals: Vec<SignalId>,

    /// Driver signal (if single driver)
    pub driver: Option<SignalId>,

    /// Total capacitance on the net
    pub total_capacitance: f64,
}

impl Net {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            signals: Vec::new(),
            driver: None,
            total_capacitance: 0.0,
        }
    }

    pub fn add_signal(&mut self, signal: SignalId, capacitance: f64) {
        self.signals.push(signal);
        self.total_capacitance += capacitance;
    }

    pub fn set_driver(&mut self, driver: SignalId) {
        self.driver = Some(driver);
    }
}

/// Wire delay model parameters
pub mod wire_model {
    use super::*;

    /// Resistance per unit length (ohms/um)
    pub const R_PER_UM: f64 = 0.05;

    /// Capacitance per unit length (fF/um)
    pub const C_PER_UM: f64 = 0.1;

    /// Calculate RC delay for a wire
    pub fn rc_delay(length_um: f64, fanout: usize) -> Delay {
        let r = R_PER_UM * length_um;
        let c = C_PER_UM * length_um + (fanout as f64 * 10.0); // 10 fF per gate input

        // Elmore delay: tau = R * C
        // Convert to picoseconds (RC product is in seconds for Ohms * Farads)
        let tau_seconds = r * c * 1e-15; // fF to F
        let tau_ps = tau_seconds * 1e12;

        tau_ps as Delay
    }

    /// Estimate wire length from chip coordinates
    pub fn estimate_length(x1: i32, y1: i32, x2: i32, y2: i32) -> f64 {
        // Manhattan distance in micrometers
        let dx = (x2 - x1).unsigned_abs();
        let dy = (y2 - y1).unsigned_abs();
        (dx + dy) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fanout_delay() {
        let f0 = Fanout::new(0);
        let f1 = Fanout::new(1);
        let f10 = Fanout::new(10);

        assert_eq!(f0.delay(), 0);
        assert_eq!(f1.delay(), 500);
        assert_eq!(f10.delay(), 5000);
    }

    #[test]
    fn test_wire_delay() {
        let wire = Wire::new(SignalId(0), vec![SignalId(1), SignalId(2), SignalId(3)]);
        assert_eq!(wire.delay(), 1500); // 3 * 500 ps
    }

    #[test]
    fn test_rc_delay() {
        // Short wire with small fanout
        let d1 = wire_model::rc_delay(10.0, 1);
        assert!(d1 > 0);

        // Longer wire with more fanout should have more delay
        let d2 = wire_model::rc_delay(100.0, 10);
        assert!(d2 > d1);
    }
}
