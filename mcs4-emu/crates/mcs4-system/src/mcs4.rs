//! MCS-4 System (4004-based)
//!
//! Complete system integration for Intel MCS-4 architecture.
//! Wires together CPU (4004), ROM (4001), and RAM (4002) chips
//! with proper bus protocol timing.

use mcs4_bus::prelude::*;
use mcs4_chips::{i4004::I4004, i4001::I4001, i4002::I4002};

/// Complete MCS-4 system
pub struct Mcs4System {
    /// 4004 CPU
    pub cpu: I4004,

    /// ROM chips (up to 16 x 4001 = 4KB)
    pub rom: Vec<I4001>,

    /// RAM chips (up to 4 banks x 4 chips = 16 x 4002)
    pub ram: Vec<I4002>,

    /// 4-bit bidirectional data bus
    pub bus: DataBus,

    /// Control signals (SYNC, CM-ROM, CM-RAM)
    pub control: ControlSignals,

    /// Two-phase clock generator
    pub clock: TwoPhaseClockTwoPhaseClock,

    /// Current bus cycle phase
    cycle: CycleState,

    /// Total machine cycles executed
    total_cycles: u64,

    /// Breakpoint addresses (stop when PC matches)
    breakpoints: Vec<u16>,
}

impl Mcs4System {
    /// Create a minimal MCS-4 system (1 ROM, 1 RAM)
    pub fn minimal() -> Self {
        Self {
            cpu: I4004::new(),
            rom: vec![I4001::new(0)],
            ram: vec![I4002::new(0, 0)],
            bus: DataBus::new(),
            control: ControlSignals::mcs4(),
            clock: TwoPhaseClockTwoPhaseClock::default_config(),
            cycle: CycleState::new(),
            total_cycles: 0,
            breakpoints: Vec::new(),
        }
    }

    /// Create a standard MCS-4 system (4 ROM, 2 RAM banks)
    pub fn standard() -> Self {
        Self {
            cpu: I4004::new(),
            rom: vec![
                I4001::new(0),
                I4001::new(1),
                I4001::new(2),
                I4001::new(3),
            ],
            ram: vec![
                // Bank 0
                I4002::new(0, 0),
                I4002::new(1, 0),
                I4002::new(2, 0),
                I4002::new(3, 0),
                // Bank 1
                I4002::new(0, 1),
                I4002::new(1, 1),
                I4002::new(2, 1),
                I4002::new(3, 1),
            ],
            bus: DataBus::new(),
            control: ControlSignals::mcs4(),
            clock: TwoPhaseClockTwoPhaseClock::default_config(),
            cycle: CycleState::new(),
            total_cycles: 0,
            breakpoints: Vec::new(),
        }
    }

    /// Create a maximal MCS-4 system (16 ROM, 4 RAM banks)
    pub fn maximal() -> Self {
        let mut rom = Vec::with_capacity(16);
        for i in 0..16 {
            rom.push(I4001::new(i));
        }

        let mut ram = Vec::with_capacity(16);
        for bank in 0..4 {
            for chip in 0..4 {
                ram.push(I4002::new(chip, bank));
            }
        }

        Self {
            cpu: I4004::new(),
            rom,
            ram,
            bus: DataBus::new(),
            control: ControlSignals::mcs4(),
            clock: TwoPhaseClockTwoPhaseClock::default_config(),
            cycle: CycleState::new(),
            total_cycles: 0,
            breakpoints: Vec::new(),
        }
    }

    /// Load program into ROM starting at address 0
    pub fn load_rom(&mut self, data: &[u8]) {
        // Distribute across ROM chips (256 bytes each)
        for (i, chunk) in data.chunks(256).enumerate() {
            if i < self.rom.len() {
                self.rom[i].load(chunk);
            }
        }
    }

    /// Load program at specific ROM address
    pub fn load_rom_at(&mut self, address: u16, data: &[u8]) {
        for (offset, &byte) in data.iter().enumerate() {
            let addr = address as usize + offset;
            let chip_id = (addr >> 8) & 0x0F;
            let chip_addr = addr & 0xFF;
            if let Some(rom) = self.rom.iter_mut().find(|r| r.chip_id == chip_id as u8) {
                rom.load_at(chip_addr, &[byte]);
            }
        }
    }

    /// Step one bus phase (1/8 of a machine cycle)
    ///
    /// Bus protocol timing:
    /// - A1-A3: CPU outputs address, ROM/RAM latch address
    /// - M1-M2: ROM outputs instruction data, CPU reads
    /// - X1-X3: CPU/RAM exchange data for I/O operations
    pub fn step(&mut self) {
        let phase = self.cycle.phase;

        match phase {
            // Address phases: CPU drives, memory latches
            BusCycle::A1 | BusCycle::A2 | BusCycle::A3 => {
                // CPU puts address on bus first
                self.cpu.tick(phase, &mut self.bus, &mut self.control);
                // ROM chips latch address
                for rom in &mut self.rom {
                    rom.tick_bus(phase, &mut self.bus, &self.control);
                }
                // RAM chips also see address phases (for SRC address)
                for ram in &mut self.ram {
                    ram.tick_bus(phase, &mut self.bus, &self.control);
                }
            }

            // Memory phases: ROM outputs data, CPU reads
            BusCycle::M1 | BusCycle::M2 => {
                // ROM puts instruction on bus first
                for rom in &mut self.rom {
                    rom.tick_bus(phase, &mut self.bus, &self.control);
                }
                // Then CPU reads from bus
                self.cpu.tick(phase, &mut self.bus, &mut self.control);
            }

            // Execute phases: bidirectional data exchange
            BusCycle::X1 | BusCycle::X2 | BusCycle::X3 => {
                // For X2 (writes): CPU puts data, RAM stores
                // For X3 (reads): RAM puts data, CPU reads
                // RAM responds first to handle reads
                for ram in &mut self.ram {
                    ram.tick_bus(phase, &mut self.bus, &self.control);
                }
                // ROM I/O ports also active during X phases
                for rom in &mut self.rom {
                    rom.tick_bus(phase, &mut self.bus, &self.control);
                }
                // CPU processes data
                self.cpu.tick(phase, &mut self.bus, &mut self.control);
            }
        }

        // Advance to next phase
        self.cycle.advance();

        // Track machine cycles (8 phases per cycle)
        if self.cycle.phase == BusCycle::A1 {
            self.total_cycles += 1;
        }
    }

    /// Run for N machine cycles
    pub fn run_cycles(&mut self, cycles: usize) {
        for _ in 0..(cycles * 8) {
            self.step();
        }
    }

    /// Run until a breakpoint or cycle limit is reached
    /// Returns true if breakpoint hit, false if limit reached
    pub fn run_until_breakpoint(&mut self, max_cycles: u64) -> bool {
        let start = self.total_cycles;
        while self.total_cycles - start < max_cycles {
            self.step();

            // Check breakpoints at start of each instruction fetch
            if self.cycle.phase == BusCycle::A1 {
                let pc = self.cpu.pc();
                if self.breakpoints.contains(&pc) {
                    return true;
                }
            }
        }
        false
    }

    /// Add a breakpoint at the given address
    pub fn add_breakpoint(&mut self, addr: u16) {
        if !self.breakpoints.contains(&addr) {
            self.breakpoints.push(addr);
        }
    }

    /// Remove a breakpoint
    pub fn remove_breakpoint(&mut self, addr: u16) {
        self.breakpoints.retain(|&a| a != addr);
    }

    /// Clear all breakpoints
    pub fn clear_breakpoints(&mut self) {
        self.breakpoints.clear();
    }

    /// Reset the system to initial state
    pub fn reset(&mut self) {
        self.cpu = I4004::new();
        self.bus = DataBus::new();
        self.control = ControlSignals::mcs4();
        self.cycle = CycleState::new();
        // Note: ROM contents preserved, RAM and registers cleared
        for ram in &mut self.ram {
            *ram = I4002::new(ram.chip_id, ram.bank_id);
        }
    }

    /// Set the CPU test pin
    pub fn set_test_pin(&mut self, state: bool) {
        self.cpu.set_test_pin(state);
    }

    /// Get current program counter
    pub fn pc(&self) -> u16 {
        self.cpu.pc()
    }

    /// Get accumulator value
    pub fn accumulator(&self) -> u8 {
        self.cpu.accumulator()
    }

    /// Get carry flag
    pub fn carry(&self) -> bool {
        self.cpu.carry()
    }

    /// Get total machine cycles executed
    pub fn cycles(&self) -> u64 {
        self.total_cycles
    }

    /// Get current bus cycle phase
    pub fn phase(&self) -> BusCycle {
        self.cycle.phase
    }

    /// Read a register pair (0-7)
    pub fn register_pair(&self, pair: u8) -> u8 {
        self.cpu.registers.get_pair(pair)
    }

    /// Read a single register (0-15)
    pub fn register(&self, r: u8) -> u8 {
        self.cpu.registers.get_r(r)
    }

    /// Read ROM at given address
    pub fn read_rom(&self, addr: u16) -> Option<u8> {
        let chip_id = ((addr >> 8) & 0x0F) as u8;
        let chip_addr = (addr & 0xFF) as u8;
        self.rom
            .iter()
            .find(|r| r.chip_id == chip_id)
            .map(|r| r.read_direct(chip_addr))
    }

    /// Read RAM at given address (bank, chip, register, character)
    pub fn read_ram(&self, bank: u8, chip: u8, reg: u8, char_addr: u8) -> Option<u8> {
        self.ram
            .iter()
            .find(|r| r.bank_id == bank && r.chip_id == chip)
            .map(|r| r.read_direct(reg, char_addr))
    }
}

impl Default for Mcs4System {
    fn default() -> Self {
        Self::minimal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_system() {
        let sys = Mcs4System::minimal();
        assert_eq!(sys.rom.len(), 1);
        assert_eq!(sys.ram.len(), 1);
        assert_eq!(sys.pc(), 0);
    }

    #[test]
    fn test_standard_system() {
        let sys = Mcs4System::standard();
        assert_eq!(sys.rom.len(), 4);
        assert_eq!(sys.ram.len(), 8);
    }

    #[test]
    fn test_load_and_run() {
        let mut sys = Mcs4System::minimal();

        // Simple program: NOP, NOP, NOP
        sys.load_rom(&[0x00, 0x00, 0x00]);

        // Run 3 instruction cycles (each is 8 phases, 2 cycles per instruction)
        sys.run_cycles(6);

        // PC should have advanced
        assert!(sys.pc() > 0);
    }

    #[test]
    fn test_ldm_instruction() {
        let mut sys = Mcs4System::minimal();

        // LDM 5 (load immediate 5 into accumulator)
        sys.load_rom(&[0xD5]);

        // Run one instruction (2 machine cycles)
        sys.run_cycles(2);

        assert_eq!(sys.accumulator(), 5);
    }

    #[test]
    fn test_breakpoint() {
        let mut sys = Mcs4System::minimal();

        // NOP, NOP, NOP, ...
        sys.load_rom(&[0x00; 16]);
        sys.add_breakpoint(4);

        let hit = sys.run_until_breakpoint(100);
        assert!(hit);
        assert_eq!(sys.pc(), 4);
    }
}
