//! Event-driven digital simulation engine

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use crate::gate::Gate;
use crate::signal::{Signal, SignalId, SignalLevel};
use crate::timing::Time;

/// A simulation event
#[derive(Clone, Debug)]
pub struct Event {
    /// Time at which this event occurs
    pub time: Time,

    /// Signal being changed
    pub target: SignalId,

    /// New value for the signal
    pub value: SignalLevel,

    /// Source of the event (for debugging)
    pub source: EventSource,
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for Event {}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

/// Source of a simulation event
#[derive(Clone, Debug)]
pub enum EventSource {
    /// Event from external stimulus
    Stimulus,
    /// Event from a gate output
    Gate(usize),
    /// Event from clock generator
    Clock,
    /// Event from reset logic
    Reset,
}

/// Configuration for the simulator
#[derive(Clone, Debug)]
pub struct SimulatorConfig {
    /// Maximum simulation time (0 = unlimited)
    pub max_time: Time,

    /// Enable signal history recording
    pub record_history: bool,

    /// Maximum history entries per signal
    pub max_history: usize,

    /// Enable delta-cycle limiting (prevent infinite loops)
    pub max_delta_cycles: usize,
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            max_time: 0,
            record_history: true,
            max_history: 10_000,
            max_delta_cycles: 1000,
        }
    }
}

/// Statistics from simulation run
#[derive(Clone, Debug, Default)]
pub struct SimulatorStats {
    /// Total events processed
    pub events_processed: u64,

    /// Total simulation time elapsed
    pub time_elapsed: Time,

    /// Number of clock cycles completed
    pub clock_cycles: u64,

    /// Peak event queue depth
    pub peak_queue_depth: usize,
}

/// Event-driven digital simulator
pub struct Simulator {
    /// Current simulation time
    current_time: Time,

    /// Event queue (min-heap by time)
    events: BinaryHeap<Reverse<Event>>,

    /// All signals in the simulation
    signals: HashMap<SignalId, Signal>,

    /// All gates in the simulation
    gates: Vec<Box<dyn Gate>>,

    /// Mapping from signal ID to gates that depend on it
    signal_to_gates: HashMap<SignalId, Vec<usize>>,

    /// Configuration
    config: SimulatorConfig,

    /// Statistics
    stats: SimulatorStats,

    /// Next available signal ID
    next_signal_id: u32,
}

impl Simulator {
    /// Create a new simulator with default configuration
    pub fn new() -> Self {
        Self::with_config(SimulatorConfig::default())
    }

    /// Create a new simulator with custom configuration
    pub fn with_config(config: SimulatorConfig) -> Self {
        Self {
            current_time: 0,
            events: BinaryHeap::new(),
            signals: HashMap::new(),
            gates: Vec::new(),
            signal_to_gates: HashMap::new(),
            config,
            stats: SimulatorStats::default(),
            next_signal_id: 0,
        }
    }

    /// Get current simulation time
    pub fn time(&self) -> Time {
        self.current_time
    }

    /// Get simulation statistics
    pub fn stats(&self) -> &SimulatorStats {
        &self.stats
    }

    /// Allocate a new signal ID
    pub fn alloc_signal(&mut self, name: impl Into<String>, initial: SignalLevel) -> SignalId {
        let id = SignalId(self.next_signal_id);
        self.next_signal_id += 1;

        let signal = if self.config.record_history {
            Signal::with_history_limit(name, initial, self.config.max_history)
        } else {
            Signal::new(name, initial)
        };

        self.signals.insert(id, signal);
        id
    }

    /// Add a gate to the simulation
    pub fn add_gate(&mut self, gate: Box<dyn Gate>) -> usize {
        let gate_id = self.gates.len();

        // Register this gate as dependent on its inputs
        for &input in gate.inputs() {
            self.signal_to_gates
                .entry(input)
                .or_default()
                .push(gate_id);
        }

        self.gates.push(gate);
        gate_id
    }

    /// Schedule an event
    pub fn schedule(&mut self, time: Time, target: SignalId, value: SignalLevel, source: EventSource) {
        let event = Event {
            time,
            target,
            value,
            source,
        };
        self.events.push(Reverse(event));
    }

    /// Schedule an event relative to current time
    pub fn schedule_delta(&mut self, delay: Time, target: SignalId, value: SignalLevel, source: EventSource) {
        self.schedule(self.current_time + delay, target, value, source);
    }

    /// Get current value of a signal
    pub fn get_signal(&self, id: SignalId) -> SignalLevel {
        self.signals
            .get(&id)
            .map(|s| s.current)
            .unwrap_or(SignalLevel::X)
    }

    /// Get signal by ID
    pub fn signal(&self, id: SignalId) -> Option<&Signal> {
        self.signals.get(&id)
    }

    /// Get mutable signal by ID
    pub fn signal_mut(&mut self, id: SignalId) -> Option<&mut Signal> {
        self.signals.get_mut(&id)
    }

    /// Process the next event
    ///
    /// Returns the time of the processed event, or None if queue is empty.
    pub fn step(&mut self) -> Option<Time> {
        let Reverse(event) = self.events.pop()?;

        // Track queue depth
        if self.events.len() > self.stats.peak_queue_depth {
            self.stats.peak_queue_depth = self.events.len();
        }

        // Advance time
        self.current_time = event.time;
        self.stats.events_processed += 1;
        self.stats.time_elapsed = self.current_time;

        // Check max time
        if self.config.max_time > 0 && self.current_time > self.config.max_time {
            return None;
        }

        // Apply the event
        self.apply_event(&event);

        Some(self.current_time)
    }

    /// Run simulation until a specific time
    pub fn run_until(&mut self, end_time: Time) {
        while let Some(time) = self.step() {
            if time >= end_time {
                break;
            }
        }
    }

    /// Run simulation for a number of events
    pub fn run_events(&mut self, count: usize) {
        for _ in 0..count {
            if self.step().is_none() {
                break;
            }
        }
    }

    /// Apply an event and propagate changes
    fn apply_event(&mut self, event: &Event) {
        // Get current signal value
        let signal = match self.signals.get_mut(&event.target) {
            Some(s) => s,
            None => return,
        };

        // Check if value actually changed
        let old_value = signal.current;
        if old_value == event.value {
            return;
        }

        // Update signal
        signal.update(event.time, event.value);

        // Propagate to dependent gates
        let dependent_gates: Vec<usize> = self
            .signal_to_gates
            .get(&event.target)
            .cloned()
            .unwrap_or_default();

        for gate_id in dependent_gates {
            self.evaluate_gate(gate_id);
        }
    }

    /// Evaluate a gate and schedule output event if changed
    fn evaluate_gate(&mut self, gate_id: usize) {
        let gate = &self.gates[gate_id];

        // Gather current input values
        let inputs: Vec<SignalLevel> = gate
            .inputs()
            .iter()
            .map(|&id| self.get_signal(id))
            .collect();

        // Evaluate gate
        let new_output = gate.evaluate(&inputs);
        let output_id = gate.output();
        let delay = gate.propagation_delay();

        // Get current output value
        let current_output = self.get_signal(output_id);

        // Schedule event if output will change
        if new_output != current_output {
            self.schedule(
                self.current_time + delay,
                output_id,
                new_output,
                EventSource::Gate(gate_id),
            );
        }
    }

    /// Reset simulation to initial state
    pub fn reset(&mut self) {
        self.current_time = 0;
        self.events.clear();
        self.stats = SimulatorStats::default();

        for signal in self.signals.values_mut() {
            signal.clear_history();
        }
    }

    /// Get all signal IDs
    pub fn signal_ids(&self) -> impl Iterator<Item = SignalId> + '_ {
        self.signals.keys().copied()
    }

    /// Check if simulation is complete (no more events)
    pub fn is_done(&self) -> bool {
        self.events.is_empty()
    }

    /// Get number of pending events
    pub fn pending_events(&self) -> usize {
        self.events.len()
    }
}

impl Default for Simulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::Inverter;
    use crate::timing::NANOSECOND;

    #[test]
    fn test_basic_simulation() {
        let mut sim = Simulator::new();

        // Create signals
        let input = sim.alloc_signal("IN", SignalLevel::Low);
        let output = sim.alloc_signal("OUT", SignalLevel::High);

        // Add inverter
        let inv = Box::new(Inverter::new(input, output, 1));
        sim.add_gate(inv);

        // Schedule input change
        sim.schedule(100 * NANOSECOND, input, SignalLevel::High, EventSource::Stimulus);

        // Run until after gate delay
        sim.run_until(200 * NANOSECOND);

        // Output should have changed to Low (inverted)
        assert_eq!(sim.get_signal(output), SignalLevel::Low);
    }

    #[test]
    fn test_signal_history() {
        let mut sim = Simulator::new();

        let sig = sim.alloc_signal("test", SignalLevel::Low);

        sim.schedule(100, sig, SignalLevel::High, EventSource::Stimulus);
        sim.schedule(200, sig, SignalLevel::Low, EventSource::Stimulus);
        sim.schedule(300, sig, SignalLevel::High, EventSource::Stimulus);

        sim.run_until(400);

        let signal = sim.signal(sig).unwrap();
        assert_eq!(signal.history().len(), 3);
    }

    #[test]
    fn test_event_ordering() {
        let mut sim = Simulator::new();

        let sig = sim.alloc_signal("test", SignalLevel::Low);

        // Schedule events out of order
        sim.schedule(300, sig, SignalLevel::High, EventSource::Stimulus);
        sim.schedule(100, sig, SignalLevel::High, EventSource::Stimulus);
        sim.schedule(200, sig, SignalLevel::Low, EventSource::Stimulus);

        // First event should be at time 100
        let time = sim.step().unwrap();
        assert_eq!(time, 100);

        let time = sim.step().unwrap();
        assert_eq!(time, 200);

        let time = sim.step().unwrap();
        assert_eq!(time, 300);
    }
}
