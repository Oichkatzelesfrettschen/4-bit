//! MCS-40 System (4040-based)
//!
//! Complete system integration for Intel MCS-40 architecture.
//! Wires together 4040 CPU, 4201 Clock, and optional MCS-40 support chips.

use mcs4_bus::prelude::*;
use mcs4_chips::{i4001::I4001, i4002::I4002, i4040::I4040, i4201::I4201, i4289::I4289};

/// Complete MCS-40 system
pub struct Mcs40System {
    /// 4040 CPU
    pub cpu: I4040,

    /// 4201 Clock Generator
    pub clock_gen: I4201,

    /// 4289 Standard Memory Interface
    pub smi: I4289,

    /// ROM chips
    pub rom: Vec<I4001>,

    /// RAM chips
    pub ram: Vec<I4002>,

    /// 4-bit bidirectional data bus
    pub bus: DataBus,

    /// Control signals
    pub control: ControlSignals,

    /// Current bus cycle phase
    cycle: CycleState,

    /// Total machine cycles
    total_cycles: u64,
}

impl Mcs40System {
    pub fn new() -> Self {
        Self {
            cpu: I4040::new(),
            clock_gen: I4201::new(),
            smi: I4289::new(),
            rom: vec![I4001::new(0)],
            ram: vec![I4002::new(0, 0)],
            bus: DataBus::new(),
            control: ControlSignals::mcs40(),
            cycle: CycleState::new(),
            total_cycles: 0,
        }
    }

    /// Step one bus phase
    pub fn step(&mut self) {
        let phase = self.cycle.phase;

        // 4201 generates clock (simulated here by BusCycle from CycleState)

        match phase {
            BusCycle::A1 | BusCycle::A2 | BusCycle::A3 => {
                self.cpu.tick(phase, &mut self.bus, &mut self.control);
                self.smi.tick_bus(phase, &mut self.bus, &self.control);
                for r in &mut self.rom {
                    r.tick_bus(phase, &mut self.bus, &self.control);
                }
                for r in &mut self.ram {
                    r.tick_bus(phase, &mut self.bus, &self.control);
                }
            }
            BusCycle::M1 | BusCycle::M2 => {
                for r in &mut self.rom {
                    r.tick_bus(phase, &mut self.bus, &self.control);
                }
                self.cpu.tick(phase, &mut self.bus, &mut self.control);
            }
            BusCycle::X1 | BusCycle::X2 | BusCycle::X3 => {
                for r in &mut self.ram {
                    r.tick_bus(phase, &mut self.bus, &self.control);
                }
                for r in &mut self.rom {
                    r.tick_bus(phase, &mut self.bus, &self.control);
                }
                self.cpu.tick(phase, &mut self.bus, &mut self.control);
            }
        }

        self.cycle.advance();
        if self.cycle.phase == BusCycle::A1 {
            self.total_cycles += 1;
        }
    }

    pub fn load_rom(&mut self, data: &[u8]) {
        for (i, chunk) in data.chunks(256).enumerate() {
            if i < self.rom.len() {
                self.rom[i].load(chunk);
            }
        }
    }

    pub fn pc(&self) -> u16 {
        self.cpu.pc()
    }
}

impl Default for Mcs40System {
    fn default() -> Self {
        Self::new()
    }
}
