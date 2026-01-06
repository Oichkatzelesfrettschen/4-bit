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
