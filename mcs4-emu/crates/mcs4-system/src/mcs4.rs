//! MCS-4 System (4004-based)
//!
//! Complete system integration for Intel MCS-4 architecture.
//! Wires together CPU (4004), ROM (4001), and RAM (4002) chips
//! with proper bus protocol timing.

use mcs4_bus::prelude::*;
use mcs4_chips::{i4001::I4001, i4002::I4002, i4003::I4003, i4004::I4004};
use mcs4_core::{FidelityManager, process::ProcessParams};
use cosmic_scheduler::PhaseScheduler;

use crate::fixture::load_hex_bytes;

#[derive(Clone, Debug)]
struct AttachedShiftReg {
    bank: u8,
    chip: u8,
    dev: I4003,
}

/// Complete MCS-4 system
pub struct Mcs4System {
    /// 4004 CPU
    pub cpu: I4004,

    /// ROM chips (up to 16 x 4001 = 4KB)
    pub rom: Vec<I4001>,

    /// RAM chips (up to 4 banks x 4 chips = 16 x 4002)
    pub ram: Vec<I4002>,

    /// Fidelity manager for multi-resolution simulation
    pub fidelity: FidelityManager,

    /// Optional attached 4003 shift registers (wired to a RAM output port, best-effort).
    shift_regs: Vec<AttachedShiftReg>,

    /// 4-bit bidirectional data bus
    pub bus: DataBus,

    /// Control signals (SYNC, CM-ROM, CM-RAM)
    pub control: ControlSignals,

    /// Two-phase clock generator
    pub clock: TwoPhaseClock,

    /// Current bus cycle phase
    cycle: CycleState,

    /// Total machine cycles executed
    total_cycles: u64,

    /// Breakpoint addresses (stop when PC matches)
    breakpoints: Vec<u16>,
}

impl PhaseScheduler for Mcs4System {
    type State = ();
    type Error = ();

    fn execute_phi1(&self, _state: &mut Self::State) -> Result<(), Self::Error> {
        // Execute logic interaction (behavioral or analog)
        Ok(())
    }

    fn execute_phi2(&self, _state: &mut Self::State) -> Result<(), Self::Error> {
        // Execute propagation
        Ok(())
    }
}

impl Mcs4System {
    /// Create a minimal MCS-4 system (1 ROM, 1 RAM)
    pub fn minimal() -> Self {
        Self {
            cpu: I4004::new(),
            rom: vec![I4001::new(0)],
            ram: vec![I4002::new(0, 0)],
            fidelity: FidelityManager::new(ProcessParams::default()),
            shift_regs: Vec::new(),
            bus: DataBus::new(),
            control: ControlSignals::mcs4(),
            clock: TwoPhaseClock::default_config(),
            cycle: CycleState::new(),
            total_cycles: 0,
            breakpoints: Vec::new(),
        }
    }

    /// Create a standard MCS-4 system (4 ROM, 2 RAM banks)
    pub fn standard() -> Self {
        Self {
            cpu: I4004::new(),
            rom: vec![I4001::new(0), I4001::new(1), I4001::new(2), I4001::new(3)],
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
            fidelity: FidelityManager::new(ProcessParams::default()),
            shift_regs: Vec::new(),
            bus: DataBus::new(),
            control: ControlSignals::mcs4(),
            clock: TwoPhaseClock::default_config(),
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
            fidelity: FidelityManager::new(ProcessParams::default()),
            shift_regs: Vec::new(),
            bus: DataBus::new(),
            control: ControlSignals::mcs4(),
            clock: TwoPhaseClock::default_config(),
            cycle: CycleState::new(),
            total_cycles: 0,
            breakpoints: Vec::new(),
        }
    }

    /// Attach a 4003 shift register to a specific RAM output port (bank + chip).
    ///
    /// Best-effort wiring model: whenever `WMP` updates the selected RAM output port, we shift in
    /// the output port bit0 into the 4003.
    pub fn attach_i4003(&mut self, bank: u8, chip: u8) {
        self.shift_regs.push(AttachedShiftReg {
            bank: bank & 0x03,
            chip: chip & 0x03,
            dev: I4003::new(),
        });
    }

    pub fn i4003_parallel_out(&self, index: usize) -> Option<u16> {
        self.shift_regs.get(index).map(|s| s.dev.parallel_out())
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

    /// Load program from a file using memory mapping
    pub fn load_rom_file(&mut self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };
        self.load_rom(&mmap);
        Ok(())
    }

    /// Load a ROM fixture from a whitespace-separated hex text file.
    pub fn load_rom_hex_file(&mut self, path: impl AsRef<std::path::Path>) -> Result<(), crate::FixtureError> {
        let bytes = load_hex_bytes(path)?;
        self.load_rom(&bytes);
        Ok(())
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

            // Execute phases: bidirectional data exchange.
            //
            // Ordering matters:
            // - X2 is write-oriented (CPU drives, peripherals latch).
            // - X3 is read-oriented (peripherals drive, CPU latches), except SRC which uses the
            //   bus in X3 for the second nibble of the address latch.
            BusCycle::X1 => {
                // No data transfer yet; keep RAM bank select inactive to avoid "always-on" behavior.
                self.control.deselect_ram(0);
                self.cpu.tick(phase, &mut self.bus, &mut self.control);
                for ram in &mut self.ram {
                    ram.tick_bus(phase, &mut self.bus, &self.control);
                }
                for rom in &mut self.rom {
                    rom.tick_bus(phase, &mut self.bus, &self.control);
                }
            }
            BusCycle::X2 => {
                self.cpu.tick(phase, &mut self.bus, &mut self.control);
                let bus_after_cpu = self.bus.read();
                // X2 is write-oriented; enable RAM bank select only for write/SRC operations.
                if self.cpu.x2_ram_bank_select() {
                    self.control.select_ram(self.cpu.ram_bank(), 0);
                } else {
                    self.control.deselect_ram(0);
                }
                for ram in &mut self.ram {
                    ram.tick_bus(phase, &mut self.bus, &self.control);
                }
                if self.control.io_op == Some(IoOp::RamPortWrite) {
                    for sr in &mut self.shift_regs {
                        if let Some(ram) = self
                            .ram
                            .iter()
                            .find(|r| r.bank_id() == sr.bank && r.chip_id() == sr.chip && r.is_selected())
                        {
                            let bit = (ram.output() & 0x01) != 0;
                            sr.dev.shift_in(bit);
                        }
                    }
                }
                for rom in &mut self.rom {
                    rom.tick_bus(phase, &mut self.bus, &self.control);
                }

                // In X2, the CPU is the bus driver for write-oriented ops; peripherals should not alter the value.
                if self.control.is_io_write() {
                    debug_assert_eq!(
                        self.bus.read(),
                        bus_after_cpu,
                        "bus changed during X2 write; io_op={:?} ram_sel={:?} rom_sel={:?}",
                        self.control.io_op,
                        self.control.selected_ram(),
                        self.control.selected_rom(),
                    );
                }
            }
            BusCycle::X3 => {
                // X3 is read-oriented (peripherals drive, CPU latches), except SRC which uses the
                // bus in X3 for the second nibble of the address latch.
                if self.cpu.x3_cpu_drives_first() {
                    // SRC uses the bus in X3, so CPU must drive before RAM latches.
                    self.cpu.tick(phase, &mut self.bus, &mut self.control);
                    self.control.select_ram(self.cpu.ram_bank(), 0);
                    for ram in &mut self.ram {
                        ram.tick_bus(phase, &mut self.bus, &self.control);
                    }
                    for rom in &mut self.rom {
                        rom.tick_bus(phase, &mut self.bus, &self.control);
                    }
                } else {
                    // For read-oriented ops, peripherals must see the control op before they drive the bus in X3.
                    self.control.clear_io_op();
                    if let Some(op) = self.cpu.x3_peripheral_io_op() {
                        self.control.set_io_op(op);
                    }
                    if self.cpu.x3_ram_bank_select() {
                        self.control.select_ram(self.cpu.ram_bank(), 0);
                    } else {
                        self.control.deselect_ram(0);
                    }
                    for ram in &mut self.ram {
                        ram.tick_bus(phase, &mut self.bus, &self.control);
                    }
                    for rom in &mut self.rom {
                        rom.tick_bus(phase, &mut self.bus, &self.control);
                    }
                    let bus_before_cpu = self.bus.read();
                    self.cpu.tick(phase, &mut self.bus, &mut self.control);

                    // In X3, peripherals are the bus driver for read-oriented ops; the CPU should not alter the value.
                    if self.control.is_io_read() {
                        debug_assert_eq!(
                            self.bus.read(),
                            bus_before_cpu,
                            "bus changed during X3 read; io_op={:?} ram_sel={:?} rom_sel={:?}",
                            self.control.io_op,
                            self.control.selected_ram(),
                            self.control.selected_rom(),
                        );
                    }
                }
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

    /// Get the CPU test pin
    pub fn test_pin(&self) -> bool {
        self.cpu.test_pin()
    }

    /// Read the 4-bit ROM I/O port output latch for the given ROM chip.
    pub fn read_rom_port(&self, chip_id: u8) -> Option<u8> {
        self.rom
            .iter()
            .find(|rom| rom.chip_id == (chip_id & 0x0F))
            .map(|rom| rom.io_output())
    }

    /// Set the 4-bit ROM I/O port input latch for the given ROM chip.
    pub fn write_rom_port_input(&mut self, chip_id: u8, value: u8) {
        if let Some(rom) = self.rom.iter_mut().find(|rom| rom.chip_id == (chip_id & 0x0F)) {
            rom.set_io_input(value);
        }
    }

    /// Read the 4-bit RAM output port latch for the given bank/chip.
    pub fn read_ram_port(&self, bank_id: u8, chip_id: u8) -> Option<u8> {
        self.ram
            .iter()
            .find(|ram| ram.bank_id == (bank_id & 0x03) && ram.chip_id == (chip_id & 0x03))
            .map(|ram| ram.output())
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

impl crate::System for Mcs4System {
    fn step(&mut self) {
        self.step();
    }

    fn step_analog(&mut self, dt: f64) {
        self.fidelity.step_analog(dt);
    }

    fn fidelity(&self) -> &FidelityManager {
        &self.fidelity
    }

    fn fidelity_mut(&mut self) -> &mut FidelityManager {
        &mut self.fidelity
    }

    fn reset(&mut self) {
        self.reset();
    }

    fn set_pc(&mut self, addr: u16) {
        self.cpu.registers.set_pc(addr);
    }

    fn pc(&self) -> u16 {
        self.pc()
    }

    fn accumulator(&self) -> u8 {
        self.accumulator()
    }

    fn load_rom(&mut self, data: &[u8]) {
        self.load_rom(data);
    }

    fn read_rom(&self, addr: u16) -> Option<u8> {
        self.read_rom(addr)
    }

    fn roms(&self) -> &[I4001] {
        &self.rom
    }

    fn roms_mut(&mut self) -> &mut [I4001] {
        &mut self.rom
    }

    fn rams(&self) -> &[I4002] {
        &self.ram
    }

    fn rams_mut(&mut self) -> &mut [I4002] {
        &mut self.ram
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
    fn test_i4003_shifts_from_ram_output_port() {
        let mut sys = Mcs4System::minimal();
        sys.attach_i4003(0, 0);

        // Select chip=0, reg=0, char=1 once, then do WMP ten times.
        // Each WMP shifts in ACC bit0 (via RAM output bit0).
        let mut program = vec![
            0x20, 0x01, // FIM P0, 0x01
            0x21, // SRC P0
        ];
        for bit in [true, false, true, false, true, false, true, false, true, false] {
            program.push(if bit { 0xD1 } else { 0xD0 }); // LDM 0/1
            program.push(0xE1); // WMP
        }
        program.push(0x00); // NOP
        sys.load_rom(&program);

        sys.run_cycles(30);

        assert_eq!(sys.i4003_parallel_out(0), Some(0x2AA));
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

    #[test]
    fn test_two_byte_jun_jumps_to_target() {
        let mut sys = Mcs4System::minimal();

        // JUN 0x005 (two-byte instruction): 0x40 0x05
        sys.load_rom(&[0x40, 0x05, 0x00, 0x00, 0x00, 0x00]);

        // First machine cycle fetches opcode, then advances PC to fetch operand.
        sys.run_cycles(1);
        assert_eq!(sys.pc(), 1);

        // Second machine cycle fetches operand and executes the jump.
        sys.run_cycles(1);
        assert_eq!(sys.pc(), 0x005);
    }

    #[test]
    fn test_two_byte_jms_and_bbl_return_address() {
        let mut sys = Mcs4System::minimal();

        // JMS 0x004 (two-byte instruction): 0x50 0x04
        // At 0x004: BBL 0xA
        sys.load_rom(&[0x50, 0x04, 0x00, 0x00, 0x00, 0x00]);
        sys.load_rom_at(0x004, &[0xCA]);

        sys.run_cycles(1);
        assert_eq!(sys.pc(), 1);

        // Execute call; next fetch should begin at subroutine target (0x004).
        sys.run_cycles(1);
        assert_eq!(sys.pc(), 0x004);

        // Execute BBL 0xA, which returns to 0x002 (next instruction after JMS operand).
        sys.run_cycles(1);
        assert_eq!(sys.accumulator(), 0xA);
        assert_eq!(sys.pc(), 0x002);
    }

    #[test]
    fn test_wrr_writes_rom_io_port() {
        let mut sys = Mcs4System::minimal();

        // LDM 0xA; WRR; NOP
        sys.load_rom(&[0xDA, 0xE2, 0x00]);

        sys.run_cycles(2);

        assert_eq!(sys.rom[0].io_output(), 0xA);
    }

    #[test]
    fn test_wpm_has_no_side_effects_by_default() {
        let mut sys = Mcs4System::minimal();

        // Seed ROM with a known byte; WPM is currently treated as "bus write with no modeled target".
        sys.rom[0].write_direct(0x10, 0x99);

        // LDM 0xA; WPM; NOP
        sys.load_rom(&[0xDA, 0xE3, 0x00]);

        sys.run_cycles(4);

        // ROM contents should not have been modified.
        assert_eq!(sys.rom[0].read_direct(0x10), 0x99);
        // ROM I/O port should not have been modified (that's WRR).
        assert_eq!(sys.rom[0].io_output(), 0x0);
    }

    #[test]
    fn test_end_to_end_src_wrm_rdm_roundtrip() {
        let mut sys = Mcs4System::minimal();

        // ACC=0xA
        // P0 = 0x01 -> chip=0, reg=0, char=1 (hi nibble encodes reg:chip as (reg<<2)|chip)
        // SRC P0 (latch address)
        // WRM (write ACC to RAM[reg][char])
        // ACC=0x0, then RDM to read back into ACC
        sys.load_rom(&[
            0xDA, // LDM 0xA
            0x20, 0x01, // FIM P0, 0x01
            0x21, // SRC P0
            0xE0, // WRM
            0xD0, // LDM 0x0
            0xE9, // RDM
            0x00, // NOP
        ]);

        // 1 + 2 + 1 + 1 + 1 + 1 + 1 = 8 machine cycles (approx; run a bit extra for safety).
        sys.run_cycles(10);

        assert_eq!(sys.accumulator(), 0xA);
        assert_eq!(sys.read_ram(0, 0, 0, 1), Some(0xA));
    }

    #[test]
    fn test_wrm_without_src_does_not_write() {
        let mut sys = Mcs4System::minimal();

        // LDM 0xA; WRM; NOP
        sys.load_rom(&[0xDA, 0xE0, 0x00]);

        // Run a few cycles; WRM should be ignored without a preceding SRC.
        sys.run_cycles(6);

        assert_eq!(sys.ram[0].read_direct(0, 0), 0x0);
    }

    #[test]
    fn test_src_drives_bus_over_x2_x3() {
        let mut sys = Mcs4System::minimal();

        // FIM P0, 0x68 ; SRC P0 ; NOP
        // 0x68 encodes chip_reg=0x6 (chip=2, reg=1) and char=0x8.
        sys.load_rom(&[0x20, 0x68, 0x21, 0x00]);

        let mut saw_x2 = false;
        let mut saw_x3 = false;

        for _ in 0..(8 * 10) {
            let phase = sys.phase();
            sys.step();
            let is_src = sys.control.io_op == Some(IoOp::Src);

            if is_src && phase == BusCycle::X2 {
                assert_eq!(sys.bus.read(), 0x06);
                saw_x2 = true;
            }
            if is_src && phase == BusCycle::X3 {
                assert_eq!(sys.bus.read(), 0x08);
                saw_x3 = true;
            }

            if saw_x2 && saw_x3 {
                break;
            }
        }

        assert!(saw_x2);
        assert!(saw_x3);
    }

    #[test]
    fn test_fixture_src_wrm_rdm_hex_executes() {
        let mut sys = Mcs4System::minimal();

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("src_wrm_rdm.hex");
        sys.load_rom_hex_file(path).expect("load fixture");

        sys.run_cycles(10);

        assert_eq!(sys.accumulator(), 0xA);
        assert_eq!(sys.read_ram(0, 0, 0, 1), Some(0xA));
    }

    #[test]
    fn test_fixture_rom_port_wrr_rdr_hex_executes() {
        let mut sys = Mcs4System::minimal();

        sys.rom[0].set_io_input(0xC);

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("rom_port_wrr_rdr.hex");
        sys.load_rom_hex_file(path).expect("load fixture");

        sys.run_cycles(12);

        assert_eq!(sys.rom[0].io_output(), 0xA);
        assert_eq!(sys.accumulator(), 0xC);
    }

    #[test]
    fn test_rom_io_op_is_phase_accurate() {
        let mut sys = Mcs4System::minimal();

        // LDM 0xA ; WRR ; LDM 0x0 ; RDR ; NOP
        sys.load_rom(&[0xDA, 0xE2, 0xD0, 0xEA, 0x00]);

        let mut saw_wrr = false;
        let mut saw_rdr = false;

        for _ in 0..(8 * 20) {
            let phase = sys.phase();
            sys.step();

            match sys.control.io_op {
                Some(IoOp::RomPortWrite) => {
                    assert_eq!(phase, BusCycle::X2);
                    // ROM port ops should not require CM-RAM.
                    assert_eq!(sys.control.selected_ram(), None);
                    saw_wrr = true;
                }
                Some(IoOp::RomPortRead) => {
                    assert_eq!(phase, BusCycle::X3);
                    assert_eq!(sys.control.selected_ram(), None);
                    saw_rdr = true;
                }
                _ => {}
            }

            if saw_wrr && saw_rdr {
                break;
            }
        }

        assert!(saw_wrr);
        assert!(saw_rdr);
    }

    #[test]
    fn test_io_op_not_asserted_in_x1() {
        let mut sys = Mcs4System::minimal();

        // SRC P0 ; WRM ; RDM ; NOP
        // (values don't matter; we just want to observe the decode/execute boundary)
        sys.load_rom(&[0x21, 0xE0, 0xE9, 0x00]);

        // Find an X1 phase and ensure io_op is not asserted there (phase-accurate gating).
        let mut saw_x1 = false;
        for _ in 0..(8 * 10) {
            let phase = sys.phase();
            sys.step();
            if phase == BusCycle::X1 {
                assert_eq!(sys.control.io_op, None);
                saw_x1 = true;
                break;
            }
        }
        assert!(saw_x1);
    }

    #[test]
    fn test_cm_ram_only_asserted_during_transfer_phases() {
        let mut sys = Mcs4System::minimal();

        // FIM P0, 0x68 ; SRC P0 ; WRM ; RDM ; NOP
        sys.load_rom(&[0x20, 0x68, 0x21, 0xE0, 0xE9, 0x00]);

        let mut saw_x1 = false;
        let mut saw_x2 = false;
        let mut saw_x3 = false;

        for _ in 0..(8 * 20) {
            let phase = sys.phase();
            sys.step();

            match phase {
                BusCycle::X1 => {
                    // X1 is decode phase; CM-RAM should be off.
                    assert_eq!(sys.control.selected_ram(), None);
                    saw_x1 = true;
                }
                BusCycle::X2 => {
                    // SRC/WRM can assert CM-RAM.
                    if sys.control.is_io_write() {
                        assert!(sys.control.selected_ram().is_some());
                        saw_x2 = true;
                    }
                }
                BusCycle::X3 => {
                    // SRC/RDM/RDn can assert CM-RAM.
                    if sys.control.io_op == Some(IoOp::Src) || sys.control.is_io_read() {
                        assert!(sys.control.selected_ram().is_some());
                        saw_x3 = true;
                    }
                }
                _ => {}
            }

            if saw_x1 && saw_x2 && saw_x3 {
                break;
            }
        }

        assert!(saw_x1);
        assert!(saw_x2);
        assert!(saw_x3);
    }

    #[test]
    fn test_io_op_is_phase_accurate() {
        let mut sys = Mcs4System::minimal();

        // FIM P0, 0x68 ; SRC P0 ; LDM 0xA ; WRM ; LDM 0x0 ; RDM ; NOP
        sys.load_rom(&[0x20, 0x68, 0x21, 0xDA, 0xE0, 0xD0, 0xE9, 0x00]);

        let mut saw_wrm = false;
        let mut saw_rdm = false;

        for _ in 0..(8 * 20) {
            let phase = sys.phase();
            sys.step();

            match sys.control.io_op {
                Some(IoOp::RamMainWrite) => {
                    assert_eq!(phase, BusCycle::X2);
                    assert!(sys.control.selected_ram().is_some());
                    saw_wrm = true;
                }
                Some(IoOp::RamMainRead) => {
                    assert_eq!(phase, BusCycle::X3);
                    assert!(sys.control.selected_ram().is_some());
                    saw_rdm = true;
                }
                _ => {}
            }

            if saw_wrm && saw_rdm {
                break;
            }
        }

        assert!(saw_wrm);
        assert!(saw_rdm);
    }

    #[test]
    fn test_fixture_ram_status_wr1_rd1_hex_executes() {
        let mut sys = Mcs4System::minimal();

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("ram_status_wr1_rd1.hex");
        sys.load_rom_hex_file(path).expect("load fixture");

        sys.run_cycles(12);

        assert_eq!(sys.accumulator(), 0xB);
        assert_eq!(sys.ram[0].read_status(1), 0xB);
    }
}
