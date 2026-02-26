//! MCS-40 integration test: 4040 CPU fetching instructions from 4308 ROM (D.3.1)
//!
//! Validates that the 4040 CPU can fetch and execute instructions through the
//! 4308 ROM's tick_bus bus protocol. This exercises the full address/data bus
//! cycle: CPU drives address (A1-A3), 4308 responds with data (M1-M2),
//! CPU decodes and executes (X1-X3).

use mcs4_bus::prelude::*;
use mcs4_chips::{i4002::I4002, i4040::I4040, i4308::I4308};

/// Minimal MCS-40 system: 4040 CPU + 4308 ROM + 4002 RAM
struct Mini4040System {
    cpu: I4040,
    rom: I4308,
    ram: I4002,
    bus: DataBus,
    ctrl: ControlSignals,
}

impl Mini4040System {
    fn new(program: &[u8]) -> Self {
        let mut rom = I4308::new(0);
        rom.load(program);
        Self {
            cpu: I4040::new(),
            rom,
            ram: I4002::new(0, 0),
            bus: DataBus::new(),
            ctrl: ControlSignals::mcs40(),
        }
    }

    /// Execute one bus phase through all chips in correct order.
    fn step_phase(&mut self, phase: BusCycle) {
        match phase {
            BusCycle::A1 | BusCycle::A2 | BusCycle::A3 => {
                // CPU drives address, ROM/RAM latch it
                self.cpu.tick(phase, &mut self.bus, &mut self.ctrl);
                self.rom.tick_bus(phase, &mut self.bus, &self.ctrl);
                self.ram.tick_bus(phase, &mut self.bus, &self.ctrl);
            }
            BusCycle::M1 | BusCycle::M2 => {
                // ROM drives data, CPU reads it
                self.rom.tick_bus(phase, &mut self.bus, &self.ctrl);
                self.cpu.tick(phase, &mut self.bus, &mut self.ctrl);
            }
            BusCycle::X1 => {
                self.ctrl.deselect_ram(0);
                self.cpu.tick(phase, &mut self.bus, &mut self.ctrl);
                self.ram.tick_bus(phase, &mut self.bus, &self.ctrl);
                self.rom.tick_bus(phase, &mut self.bus, &self.ctrl);
            }
            BusCycle::X2 => {
                self.cpu.tick(phase, &mut self.bus, &mut self.ctrl);
                if self.cpu.x2_ram_bank_select() {
                    self.ctrl.select_ram(self.cpu.ram_bank(), 0);
                } else {
                    self.ctrl.deselect_ram(0);
                }
                self.ram.tick_bus(phase, &mut self.bus, &self.ctrl);
                self.rom.tick_bus(phase, &mut self.bus, &self.ctrl);
            }
            BusCycle::X3 => {
                if self.cpu.x3_cpu_drives_first() {
                    self.cpu.tick(phase, &mut self.bus, &mut self.ctrl);
                    self.ctrl.select_ram(self.cpu.ram_bank(), 0);
                    self.ram.tick_bus(phase, &mut self.bus, &self.ctrl);
                    self.rom.tick_bus(phase, &mut self.bus, &self.ctrl);
                } else {
                    self.ctrl.clear_io_op();
                    if let Some(op) = self.cpu.x3_peripheral_io_op() {
                        self.ctrl.set_io_op(op);
                    }
                    if self.cpu.x3_ram_bank_select() {
                        self.ctrl.select_ram(self.cpu.ram_bank(), 0);
                    } else {
                        self.ctrl.deselect_ram(0);
                    }
                    self.ram.tick_bus(phase, &mut self.bus, &self.ctrl);
                    self.rom.tick_bus(phase, &mut self.bus, &self.ctrl);
                    self.cpu.tick(phase, &mut self.bus, &mut self.ctrl);
                }
            }
        }
    }

    /// Run one complete machine cycle (8 bus phases).
    fn run_cycle(&mut self) {
        for phase in [
            BusCycle::A1,
            BusCycle::A2,
            BusCycle::A3,
            BusCycle::M1,
            BusCycle::M2,
            BusCycle::X1,
            BusCycle::X2,
            BusCycle::X3,
        ] {
            self.step_phase(phase);
        }
    }

    /// Run N machine cycles.
    fn run_cycles(&mut self, n: usize) {
        for _ in 0..n {
            self.run_cycle();
        }
    }

    fn pc(&self) -> u16 {
        self.cpu.pc()
    }

    fn accumulator(&self) -> u8 {
        self.cpu.alu.accumulator()
    }
}

// --- Integration tests ---

#[test]
fn nop_advances_pc() {
    let mut sys = Mini4040System::new(&[0x00, 0x00, 0x00]);

    assert_eq!(sys.pc(), 0);
    sys.run_cycles(1);
    assert_eq!(sys.pc(), 1);
    sys.run_cycles(1);
    assert_eq!(sys.pc(), 2);
}

#[test]
fn ldm_loads_accumulator() {
    // LDM 0x5 = 0xD5
    let mut sys = Mini4040System::new(&[0xD5, 0x00]);

    sys.run_cycles(1);
    assert_eq!(sys.accumulator(), 0x5);
    assert_eq!(sys.pc(), 1);
}

#[test]
fn jun_jumps_to_target() {
    // JUN 0x010 = 0x40 0x10 (two-byte instruction)
    let mut program = vec![0u8; 1024];
    program[0] = 0x40; // JUN (high nibble=4, address high=0)
    program[1] = 0x10; // address low byte = 0x10
    program[0x10] = 0xD7; // LDM 0x7 at target

    let mut sys = Mini4040System::new(&program);

    // First cycle: fetch JUN opcode at 0x000
    sys.run_cycles(1);
    assert_eq!(sys.pc(), 1);

    // Second cycle: fetch JUN operand at 0x001, then jump
    sys.run_cycles(1);
    assert_eq!(sys.pc(), 0x010);

    // Third cycle: execute LDM 0x7 at 0x010
    sys.run_cycles(1);
    assert_eq!(sys.accumulator(), 0x7);
}

#[test]
fn jms_and_bbl_call_return() {
    // JMS 0x004 = 0x50 0x04 (call subroutine)
    // At 0x004: BBL 0xA = 0xCA (return with accumulator = 0xA)
    let mut sys = Mini4040System::new(&[0x50, 0x04, 0x00, 0x00, 0xCA]);

    // Fetch JMS opcode
    sys.run_cycles(1);
    assert_eq!(sys.pc(), 1);

    // Fetch JMS operand + jump to 0x004
    sys.run_cycles(1);
    assert_eq!(sys.pc(), 0x004);

    // Execute BBL 0xA: return to 0x002, acc=0xA
    sys.run_cycles(1);
    assert_eq!(sys.pc(), 0x002);
    assert_eq!(sys.accumulator(), 0xA);
}

#[test]
fn inc_register_via_4308() {
    // FIM P0, 0x05 (loads R0=0, R1=5); INC R1; NOP
    // FIM = 0x20 0x05, INC R1 = 0x61
    let mut sys = Mini4040System::new(&[0x20, 0x05, 0x61, 0x00]);

    // FIM P0, 0x05 (2 cycles)
    sys.run_cycles(2);
    assert_eq!(sys.pc(), 2);

    // INC R1 (1 cycle)
    sys.run_cycles(1);
    assert_eq!(sys.pc(), 3);
}

#[test]
fn rom_io_port_wrr_rdr_roundtrip() {
    // LDM 0xA; WRR; LDM 0x0; RDR; NOP
    // WRR = 0xE2, RDR = 0xEA
    let mut sys = Mini4040System::new(&[0xDA, 0xE2, 0xD0, 0xEA, 0x00]);

    // LDM 0xA
    sys.run_cycles(1);
    assert_eq!(sys.accumulator(), 0xA);

    // WRR: write accumulator to ROM I/O port
    sys.run_cycles(1);

    // Check ROM port got the value
    assert_eq!(sys.rom.read_port(0), 0xA);

    // LDM 0x0: clear accumulator
    sys.run_cycles(1);
    assert_eq!(sys.accumulator(), 0x0);

    // RDR: read ROM I/O port back into accumulator
    sys.run_cycles(1);
    assert_eq!(sys.accumulator(), 0xA);
}

#[test]
fn src_wrm_rdm_ram_roundtrip() {
    // FIM P0, 0x01; SRC P0; LDM 0xA; WRM; LDM 0x0; RDM; NOP
    let mut sys = Mini4040System::new(&[0x20, 0x01, 0x21, 0xDA, 0xE0, 0xD0, 0xE9, 0x00]);

    // Run enough cycles for the full sequence
    sys.run_cycles(10);

    assert_eq!(sys.accumulator(), 0xA);
    assert_eq!(sys.ram.read_direct(0, 1), 0xA);
}

#[test]
fn multi_instruction_sequence() {
    // Test a longer sequence to verify sustained operation
    // LDM 3; IAC; IAC; IAC; NOP (accumulator: 3 -> 4 -> 5 -> 6)
    // IAC = 0xF2
    let mut sys = Mini4040System::new(&[0xD3, 0xF2, 0xF2, 0xF2, 0x00]);

    sys.run_cycles(1); // LDM 3
    assert_eq!(sys.accumulator(), 3);

    sys.run_cycles(1); // IAC
    assert_eq!(sys.accumulator(), 4);

    sys.run_cycles(1); // IAC
    assert_eq!(sys.accumulator(), 5);

    sys.run_cycles(1); // IAC
    assert_eq!(sys.accumulator(), 6);
}

#[test]
fn rom_chip_select_prevents_bus_contention() {
    // Two ROMs on the bus, only chip_id=0 should respond for low addresses
    let mut cpu = I4040::new();
    let mut rom0 = I4308::new(0);
    let mut rom1 = I4308::new(1);

    rom0.load(&[0xD5]); // LDM 5
    rom1.load(&[0xDA]); // LDM A (should NOT be fetched)

    let mut bus = DataBus::new();
    let mut ctrl = ControlSignals::mcs40();

    // A1: CPU drives address low nibble
    cpu.tick(BusCycle::A1, &mut bus, &mut ctrl);
    rom0.tick_bus(BusCycle::A1, &mut bus, &ctrl);
    rom1.tick_bus(BusCycle::A1, &mut bus, &ctrl);

    // A2: CPU drives address mid nibble
    cpu.tick(BusCycle::A2, &mut bus, &mut ctrl);
    rom0.tick_bus(BusCycle::A2, &mut bus, &ctrl);
    rom1.tick_bus(BusCycle::A2, &mut bus, &ctrl);

    // A3: CPU drives address high nibble + selects ROM bank 0
    cpu.tick(BusCycle::A3, &mut bus, &mut ctrl);
    rom0.tick_bus(BusCycle::A3, &mut bus, &ctrl);
    rom1.tick_bus(BusCycle::A3, &mut bus, &ctrl);

    // Only ROM 0 should be selected
    assert!(rom0.is_selected());
    assert!(!rom1.is_selected());

    // M1: ROM drives low nibble of instruction
    rom0.tick_bus(BusCycle::M1, &mut bus, &ctrl);
    rom1.tick_bus(BusCycle::M1, &mut bus, &ctrl);
    let low = bus.read();

    // M2: ROM drives high nibble of instruction
    rom0.tick_bus(BusCycle::M2, &mut bus, &ctrl);
    rom1.tick_bus(BusCycle::M2, &mut bus, &ctrl);
    let high = bus.read();

    let byte = low | (high << 4);
    // Should be 0xD5 (from rom0), not 0xDA (from rom1)
    assert_eq!(byte, 0xD5);
}
