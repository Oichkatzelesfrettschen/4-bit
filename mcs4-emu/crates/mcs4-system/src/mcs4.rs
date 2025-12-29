//! MCS-4 System (4004-based)

use mcs4_bus::prelude::*;
use mcs4_chips::{i4004::I4004, i4001::I4001, i4002::I4002};

/// Complete MCS-4 system
pub struct Mcs4System {
    pub cpu: I4004,
    pub rom: Vec<I4001>,
    pub ram: Vec<I4002>,
    pub bus: DataBus,
    pub control: ControlSignals,
    pub clock: TwoPhaseClockTwoPhaseClock,
    cycle: CycleState,
}

impl Mcs4System {
    /// Create a minimal MCS-4 system (1 ROM, 1 RAM)
    pub fn minimal() -> Self {
        Self {
            cpu: I4004::new(),
            rom: vec![I4001::new(0)],
            ram: vec![I4002::new(0)],
            bus: DataBus::new(),
            control: ControlSignals::mcs4(),
            clock: TwoPhaseClockTwoPhaseClock::default_config(),
            cycle: CycleState::new(),
        }
    }

    /// Load program into ROM
    pub fn load_rom(&mut self, data: &[u8]) {
        if let Some(rom) = self.rom.first_mut() {
            rom.load(data);
        }
    }

    /// Step one bus phase
    pub fn step(&mut self) {
        let phase = self.cycle.phase;
        self.cpu.tick(phase, &mut self.bus, &mut self.control);
        self.cycle.advance();
    }

    /// Run for N machine cycles
    pub fn run_cycles(&mut self, cycles: usize) {
        for _ in 0..(cycles * 8) {
            self.step();
        }
    }
}
