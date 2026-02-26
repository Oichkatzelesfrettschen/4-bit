use std::collections::VecDeque;

use mcs4_bus::prelude::*;

/// Maximum number of samples to keep
const MAX_SAMPLES: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sample {
    pub tick: u64,
    pub phi1: bool,
    pub phi2: bool,
    pub sync: bool,
    pub data: u8,   // 4-bit data bus
    pub cm_rom: u8, // 4-bit ROM select
    pub cm_ram: u8, // 4-bit RAM select
    pub phase: BusCycle,
}

pub struct SignalTrace {
    samples: VecDeque<Sample>,
}

impl SignalTrace {
    pub fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(MAX_SAMPLES),
        }
    }

    pub fn push(&mut self, tick: u64, bus: &DataBus, ctrl: &ControlSignals, phase: BusCycle, clock: &TwoPhaseClock) {
        if self.samples.len() >= MAX_SAMPLES {
            self.samples.pop_front();
        }

        self.samples.push_back(Sample {
            tick,
            phi1: clock.phi1_high(),
            phi2: clock.phi2_high(),
            sync: ctrl.sync.current == mcs4_core::signal::SignalLevel::High,
            data: bus.read(),
            cm_rom: ctrl.cm_rom(),
            cm_ram: ctrl.cm_ram(),
            phase,
        });
    }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<'_, Sample> {
        self.samples.iter()
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }
}

impl Default for SignalTrace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_trace_is_empty() {
        let trace = SignalTrace::new();
        assert!(trace.is_empty());
        assert_eq!(trace.len(), 0);
    }

    #[test]
    fn push_increments_len() {
        let mut trace = SignalTrace::new();
        let bus = DataBus::new();
        let ctrl = ControlSignals::mcs4();
        let clock = TwoPhaseClock::default_config();
        trace.push(0, &bus, &ctrl, BusCycle::A1, &clock);
        assert_eq!(trace.len(), 1);
        assert!(!trace.is_empty());
    }

    #[test]
    fn clear_empties_trace() {
        let mut trace = SignalTrace::new();
        let bus = DataBus::new();
        let ctrl = ControlSignals::mcs4();
        let clock = TwoPhaseClock::default_config();
        for i in 0..10 {
            trace.push(i, &bus, &ctrl, BusCycle::A1, &clock);
        }
        assert_eq!(trace.len(), 10);
        trace.clear();
        assert!(trace.is_empty());
    }

    #[test]
    fn iter_yields_pushed_samples() {
        let mut trace = SignalTrace::new();
        let bus = DataBus::new();
        let ctrl = ControlSignals::mcs4();
        let clock = TwoPhaseClock::default_config();
        for i in 0..5u64 {
            trace.push(i, &bus, &ctrl, BusCycle::A1, &clock);
        }
        let ticks: Vec<u64> = trace.iter().map(|s| s.tick).collect();
        assert_eq!(ticks, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn default_is_new() {
        let trace = SignalTrace::default();
        assert!(trace.is_empty());
    }

    #[test]
    fn push_records_phase() {
        let mut trace = SignalTrace::new();
        let bus = DataBus::new();
        let ctrl = ControlSignals::mcs4();
        let clock = TwoPhaseClock::default_config();

        let phases = [
            BusCycle::A1,
            BusCycle::A2,
            BusCycle::A3,
            BusCycle::M1,
            BusCycle::M2,
            BusCycle::X1,
            BusCycle::X2,
            BusCycle::X3,
        ];
        for (i, &phase) in phases.iter().enumerate() {
            trace.push(i as u64, &bus, &ctrl, phase, &clock);
        }

        let recorded: Vec<BusCycle> = trace.iter().map(|s| s.phase).collect();
        assert_eq!(recorded, phases.to_vec());
    }

    #[test]
    fn push_records_bus_data() {
        let mut trace = SignalTrace::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs4();
        let clock = TwoPhaseClock::default_config();

        bus.write(0xA);
        trace.push(0, &bus, &ctrl, BusCycle::M1, &clock);

        let sample = trace.iter().next().expect("trace should have one sample");
        assert_eq!(sample.data, 0xA);
    }

    #[test]
    fn overflow_evicts_oldest() {
        let mut trace = SignalTrace::new();
        let bus = DataBus::new();
        let ctrl = ControlSignals::mcs4();
        let clock = TwoPhaseClock::default_config();

        // Push MAX_SAMPLES + 10 to trigger eviction
        for i in 0..(MAX_SAMPLES + 10) as u64 {
            trace.push(i, &bus, &ctrl, BusCycle::A1, &clock);
        }

        assert_eq!(trace.len(), MAX_SAMPLES);
        // Oldest tick should be 10 (first 10 were evicted)
        let first = trace.iter().next().expect("trace should not be empty after overflow");
        assert_eq!(first.tick, 10);
    }

    #[test]
    fn iter_count_matches_len() {
        let mut trace = SignalTrace::new();
        let bus = DataBus::new();
        let ctrl = ControlSignals::mcs4();
        let clock = TwoPhaseClock::default_config();

        for i in 0..50u64 {
            trace.push(i, &bus, &ctrl, BusCycle::A1, &clock);
        }
        assert_eq!(trace.iter().count(), trace.len());
    }
}
