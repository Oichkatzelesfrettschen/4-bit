//! Unit and integration tests for the 4004 CPU top-level (`i4004/mod.rs`).
//!
//! The 4004 executes every instruction as an 8-subcycle machine cycle: A1/A2/A3
//! emit the 12-bit ROM address one nibble at a time, M1/M2 latch the OPA and OPR
//! nibbles of the instruction, and X1/X2/X3 decode and execute it. SYNC marks the
//! A1 boundary of each machine cycle, and CM-ROM selects the addressed ROM bank
//! during A3.
//!
//! These tests drive the CPU exclusively through its public `tick(phase, bus, ctrl)`
//! surface. A minimal `Cpu` harness stands in for the ROM and peripherals: it feeds
//! `rom[pc]` onto the bus during M1/M2 (mirroring the fetch path in
//! `mcs4-system/src/mcs4.rs`) and drives a settable value onto the bus during X3 for
//! read-oriented instructions (RDM/RDR/RDn/ADM/SBM), exactly where a 4001/4002 would.
//! Ticking all eight phases in order per cycle keeps the CPU's internal `CycleState`
//! (`second_cycle`/`two_cycle`) coherent with the harness phase.

use mcs4_bus::prelude::*;
use mcs4_chips::{i4004::I4004, Chip};
use mcs4_core::prelude::SignalLevel;

/// Minimal single-CPU harness: ROM fetch plus a peripheral-driven X3 read value.
struct Cpu {
    cpu: I4004,
    bus: DataBus,
    ctrl: ControlSignals,
    rom: Vec<u8>,
    phase: BusCycle,
    /// Value a 4002/4001 would drive on the bus during X3 for read-oriented ops.
    read_value: u8,
    /// 12-bit ROM address latched from the bus during A1-A3, as a 4001 would.
    latched_addr: u16,
}

impl Cpu {
    fn new() -> Self {
        Self {
            cpu: I4004::new(),
            bus: DataBus::new(),
            ctrl: ControlSignals::mcs4(),
            rom: vec![0u8; 4096],
            phase: BusCycle::A1,
            read_value: 0,
            latched_addr: 0,
        }
    }

    /// Load program bytes into ROM starting at `addr`.
    fn load(&mut self, addr: usize, bytes: &[u8]) {
        self.rom[addr..addr + bytes.len()].copy_from_slice(bytes);
    }

    /// Tick the current phase, latching the CPU-driven address during A1-A3,
    /// presenting ROM bytes at M1/M2 from the latched address (as a 4001
    /// does; the FIN second cycle emits an indirect address, not the PC), and
    /// driving the peripheral read value at X3 for read ops.
    fn tick_phase(&mut self) {
        let phase = self.phase;
        match phase {
            BusCycle::M1 => {
                let byte = self.rom[self.latched_addr as usize];
                self.bus.write(byte & 0x0F);
            }
            BusCycle::M2 => {
                let byte = self.rom[self.latched_addr as usize];
                self.bus.write((byte >> 4) & 0x0F);
            }
            // Read-oriented ops latch the bus in X3; a peripheral must drive it first.
            BusCycle::X3 if self.cpu.x3_peripheral_io_op().is_some() => {
                self.bus.write(self.read_value & 0x0F);
            }
            _ => {}
        }
        self.cpu.tick(phase, &mut self.bus, &mut self.ctrl);
        match phase {
            BusCycle::A1 => {
                self.latched_addr = (self.latched_addr & 0xFF0) | u16::from(self.bus.read() & 0x0F);
            }
            BusCycle::A2 => {
                self.latched_addr = (self.latched_addr & 0xF0F) | (u16::from(self.bus.read() & 0x0F) << 4);
            }
            BusCycle::A3 => {
                self.latched_addr = (self.latched_addr & 0x0FF) | (u16::from(self.bus.read() & 0x0F) << 8);
            }
            _ => {}
        }
        self.phase = phase.next();
    }

    /// Run `cycles` complete machine cycles (8 phases each).
    fn run_cycles(&mut self, cycles: usize) {
        for _ in 0..cycles * 8 {
            self.tick_phase();
        }
    }

    fn acc(&self) -> u8 {
        self.cpu.accumulator()
    }

    fn carry(&self) -> bool {
        self.cpu.carry()
    }

    fn pc(&self) -> u16 {
        self.cpu.pc()
    }
}

// ---------------------------------------------------------------------------
// Reset and initial state
// ---------------------------------------------------------------------------

#[test]
fn reset_state_is_all_zero() {
    let cpu = I4004::new();
    assert_eq!(cpu.pc(), 0);
    assert_eq!(cpu.accumulator(), 0);
    assert!(!cpu.carry());
    assert_eq!(cpu.ram_address(), 0);
    assert_eq!(cpu.ram_chip(), 0);
    // RESET selects DATA RAM bank 0, i.e. the CM-RAM0 line.
    assert_eq!(cpu.ram_bank(), 0b0001);
    assert!(!cpu.test_pin());
}

#[test]
fn chip_reset_clears_execution_state() {
    let mut harness = Cpu::new();
    // LDM 5 leaves the accumulator non-zero; the test pin is set high externally.
    harness.load(0, &[0xD5]);
    harness.run_cycles(1);
    harness.cpu.set_test_pin(true);
    assert_eq!(harness.acc(), 5);

    harness.cpu.reset();
    assert_eq!(harness.cpu.pc(), 0);
    assert_eq!(harness.cpu.accumulator(), 0);
    assert!(!harness.cpu.carry());
    assert!(!harness.cpu.test_pin());
}

// ---------------------------------------------------------------------------
// Machine-cycle phase sequence: address emission, SYNC, CM-ROM
// ---------------------------------------------------------------------------

#[test]
fn address_nibbles_and_sync_and_rom_select_across_a_phases() {
    let mut harness = Cpu::new();
    // A 12-bit PC exercises all three address nibbles distinctly.
    harness.cpu.registers.set_pc(0xABC);

    // A1 emits address bits 0-3 and asserts SYNC to mark the machine-cycle start.
    harness.tick_phase();
    assert_eq!(harness.bus.read(), 0xC);
    assert_eq!(harness.ctrl.sync.current, SignalLevel::High);

    // A2 emits bits 4-7 and deasserts SYNC.
    harness.tick_phase();
    assert_eq!(harness.bus.read(), 0xB);
    assert_eq!(harness.ctrl.sync.current, SignalLevel::Low);

    // A3 emits bits 8-11 and drives CM-ROM for the addressed page.
    harness.tick_phase();
    assert_eq!(harness.bus.read(), 0xA);
    assert_eq!(harness.ctrl.selected_rom(), Some(0xA));
}

#[test]
fn eight_phase_cycle_fetches_and_executes_single_byte() {
    let mut harness = Cpu::new();
    harness.load(0, &[0xD7]); // LDM 7

    // Walk all eight phases; the instruction fetches in M1/M2 and executes by X3.
    let expected = [
        BusCycle::A1,
        BusCycle::A2,
        BusCycle::A3,
        BusCycle::M1,
        BusCycle::M2,
        BusCycle::X1,
        BusCycle::X2,
        BusCycle::X3,
    ];
    for phase in expected {
        assert_eq!(harness.phase, phase);
        harness.tick_phase();
    }
    assert_eq!(harness.phase, BusCycle::A1); // wrapped back to a new cycle
    assert_eq!(harness.acc(), 7);
    assert_eq!(harness.pc(), 1); // PC advanced past the one-byte instruction
}

// ---------------------------------------------------------------------------
// ALU / accumulator group
// ---------------------------------------------------------------------------

#[test]
fn ldm_loads_immediate_into_accumulator() {
    let mut harness = Cpu::new();
    harness.load(0, &[0xDA]); // LDM 0xA
    harness.run_cycles(1);
    assert_eq!(harness.acc(), 0xA);
}

#[test]
fn add_register_with_carry_in_clear() {
    let mut harness = Cpu::new();
    harness.cpu.registers.set_r(5, 3);
    // Fresh CPU: carry starts clear, so the carry-in does not perturb 5 + 3.
    harness.load(0, &[0xD5, 0x85]); // LDM 5 ; ADD R5
    harness.run_cycles(2);
    assert_eq!(harness.acc(), 8);
    assert!(!harness.carry());
}

#[test]
fn add_sets_carry_on_overflow() {
    let mut harness = Cpu::new();
    harness.cpu.registers.set_r(1, 1);
    harness.load(0, &[0xDF, 0x81]); // LDM 0xF ; ADD R1
    harness.run_cycles(2);
    assert_eq!(harness.acc(), 0); // 15 + 1 = 16 -> 0
    assert!(harness.carry());
}

#[test]
fn sub_uses_inverted_borrow_carry_convention() {
    let mut harness = Cpu::new();
    harness.cpu.registers.set_r(3, 3);
    // True subtraction on the 4004 requires carry pre-set: SUB computes
    // ACC + ~R + carry, so STC first gives 8 + 12 + 1 = 21 -> ACC=5, carry=1 (no borrow).
    harness.load(0, &[0xD8, 0xFA, 0x93]); // LDM 8 ; STC ; SUB R3
    harness.run_cycles(3);
    assert_eq!(harness.acc(), 5);
    assert!(harness.carry());
}

#[test]
fn iac_increments_accumulator_and_flags_carry() {
    let mut harness = Cpu::new();
    harness.load(0, &[0xDF, 0xF2]); // LDM 0xF ; IAC
    harness.run_cycles(2);
    assert_eq!(harness.acc(), 0);
    assert!(harness.carry()); // 0xF + 1 overflowed the nibble
}

#[test]
fn dac_decrements_with_borrow_semantics() {
    // DAC sets carry=1 (no borrow) when ACC was non-zero, carry=0 (borrow) on wrap.
    let mut non_zero = Cpu::new();
    non_zero.load(0, &[0xD5, 0xF8]); // LDM 5 ; DAC
    non_zero.run_cycles(2);
    assert_eq!(non_zero.acc(), 4);
    assert!(non_zero.carry());

    let mut wrap = Cpu::new();
    wrap.load(0, &[0xD0, 0xF8]); // LDM 0 ; DAC
    wrap.run_cycles(2);
    assert_eq!(wrap.acc(), 0xF);
    assert!(!wrap.carry());
}

#[test]
fn clb_clears_accumulator_and_carry() {
    let mut harness = Cpu::new();
    harness.load(0, &[0xDA, 0xFA, 0xF0]); // LDM 0xA ; STC ; CLB
    harness.run_cycles(3);
    assert_eq!(harness.acc(), 0);
    assert!(!harness.carry());
}

#[test]
fn carry_group_clc_stc_cmc() {
    let mut harness = Cpu::new();
    // STC sets carry, CMC complements it to clear, CLC leaves it clear.
    harness.load(0, &[0xFA, 0xF3, 0xF1]); // STC ; CMC ; CLC
    harness.run_cycles(1);
    assert!(harness.carry()); // after STC
    harness.run_cycles(1);
    assert!(!harness.carry()); // after CMC
    harness.run_cycles(1);
    assert!(!harness.carry()); // after CLC
}

#[test]
fn ral_and_rar_rotate_through_carry() {
    let mut left = Cpu::new();
    left.load(0, &[0xDA, 0xF5]); // LDM 0xA (1010) ; RAL
    left.run_cycles(2);
    assert_eq!(left.acc(), 0b0100); // bit3 out to carry, carry-in (0) into bit0
    assert!(left.carry());

    let mut right = Cpu::new();
    right.load(0, &[0xD5, 0xF6]); // LDM 5 (0101) ; RAR
    right.run_cycles(2);
    assert_eq!(right.acc(), 0b0010);
    assert!(right.carry());
}

#[test]
fn tcc_transfers_carry_to_accumulator_and_clears_it() {
    let mut harness = Cpu::new();
    harness.load(0, &[0xFA, 0xF7]); // STC ; TCC
    harness.run_cycles(2);
    assert_eq!(harness.acc(), 1);
    assert!(!harness.carry());
}

// ---------------------------------------------------------------------------
// Index register and register-pair operations
// ---------------------------------------------------------------------------

#[test]
fn ld_loads_register_into_accumulator() {
    let mut harness = Cpu::new();
    harness.cpu.registers.set_r(9, 0xC);
    harness.load(0, &[0xA9]); // LD R9
    harness.run_cycles(1);
    assert_eq!(harness.acc(), 0xC);
}

#[test]
fn xch_swaps_accumulator_and_register() {
    let mut harness = Cpu::new();
    harness.cpu.registers.set_r(5, 0xA);
    harness.load(0, &[0xD3, 0xB5]); // LDM 3 ; XCH R5
    harness.run_cycles(2);
    assert_eq!(harness.acc(), 0xA);
    assert_eq!(harness.cpu.registers.get_r(5), 3);
}

#[test]
fn inc_register_wraps_at_sixteen() {
    let mut harness = Cpu::new();
    harness.cpu.registers.set_r(2, 0xF);
    harness.load(0, &[0x62]); // INC R2
    harness.run_cycles(1);
    assert_eq!(harness.cpu.registers.get_r(2), 0);
}

#[test]
fn fim_loads_a_register_pair_and_src_latches_ram_address() {
    let mut harness = Cpu::new();
    // FIM P0, 0x68 loads R0:R1 = 6:8; SRC P0 splits it into chip=6, char=8.
    harness.load(0, &[0x20, 0x68, 0x21]); // FIM P0, 0x68 ; SRC P0
    harness.run_cycles(2); // FIM is two machine cycles
    assert_eq!(harness.cpu.registers.get_pair(0), 0x68);

    harness.run_cycles(1); // SRC
    assert_eq!(harness.cpu.ram_chip(), 0x6);
    assert_eq!(harness.cpu.ram_address(), 0x8);
}

// ---------------------------------------------------------------------------
// Control flow
// ---------------------------------------------------------------------------

#[test]
fn two_byte_jun_fetch_spans_two_machine_cycles() {
    let mut harness = Cpu::new();
    harness.load(0, &[0x41, 0x23]); // JUN 0x123

    // First machine cycle latches the opcode and advances PC to the operand byte.
    harness.run_cycles(1);
    assert_eq!(harness.pc(), 1);

    // Second machine cycle latches the operand and takes the jump.
    harness.run_cycles(1);
    assert_eq!(harness.pc(), 0x123);
}

#[test]
fn jms_then_bbl_round_trips_the_stack() {
    let mut harness = Cpu::new();
    harness.load(0, &[0x50, 0x05]); // JMS 0x005
    harness.load(0x005, &[0xC7]); // BBL 7 at the subroutine entry

    harness.run_cycles(2); // JMS (two cycles) pushes return addr 0x002, jumps to 0x005
    assert_eq!(harness.pc(), 0x005);

    harness.run_cycles(1); // BBL returns and loads 7 into the accumulator
    assert_eq!(harness.pc(), 0x002);
    assert_eq!(harness.acc(), 7);
}

#[test]
fn jcn_branches_when_condition_true() {
    let mut harness = Cpu::new();
    // Condition 0x2 tests carry; STC first so the branch is taken to page-local 0x0A.
    harness.load(0, &[0xFA, 0x12, 0x0A]); // STC ; JCN carry, 0x0A
    harness.run_cycles(1); // STC
    harness.run_cycles(2); // JCN (two cycles)
    assert_eq!(harness.pc(), 0x00A);
}

#[test]
fn jcn_falls_through_when_condition_false() {
    let mut harness = Cpu::new();
    // Carry is clear on a fresh CPU, so the carry-conditional branch is not taken.
    harness.load(0, &[0x12, 0x0A]); // JCN carry, 0x0A
    harness.run_cycles(2);
    assert_eq!(harness.pc(), 2); // advanced past the two-byte instruction
}

#[test]
fn isz_branches_until_register_wraps_to_zero() {
    // Register below wrap: increment, do not reach zero, take the branch.
    let mut branch = Cpu::new();
    branch.cpu.registers.set_r(3, 5);
    branch.load(0, &[0x73, 0x0A]); // ISZ R3, 0x0A
    branch.run_cycles(2);
    assert_eq!(branch.cpu.registers.get_r(3), 6);
    assert_eq!(branch.pc(), 0x00A);

    // Register at 0xF: increment wraps to zero, skip the branch (fall through).
    let mut wrap = Cpu::new();
    wrap.cpu.registers.set_r(3, 0xF);
    wrap.load(0, &[0x73, 0x0A]); // ISZ R3, 0x0A
    wrap.run_cycles(2);
    assert_eq!(wrap.cpu.registers.get_r(3), 0);
    assert_eq!(wrap.pc(), 2);
}

#[test]
fn jin_jumps_indirect_through_register_pair() {
    let mut harness = Cpu::new();
    // JIN P1 loads PC low byte from R2:R3 within the current page.
    harness.cpu.registers.set_r(2, 0x1);
    harness.cpu.registers.set_r(3, 0xA);
    harness.load(0, &[0x33]); // JIN P1
    harness.run_cycles(1);
    assert_eq!(harness.pc(), 0x01A);
}

// ---------------------------------------------------------------------------
// I/O, RAM control, and phase-accurate bus/control surfaces
// ---------------------------------------------------------------------------

#[test]
fn dcl_decodes_accumulator_into_cm_ram_lines() {
    // DCL uses accumulator bits 2:0 to choose 1 of 8 DATA RAM banks; the CPU
    // records the decoded CM-RAM line mask: 000 asserts CM-RAM0, 001 CM-RAM1,
    // 010 CM-RAM2, 100 CM-RAM3, and other values assert line combinations.
    let cases: [(u8, u8); 8] = [
        (0, 0b0001), // bank 0 -> CM-RAM0
        (1, 0b0010), // bank 1 -> CM-RAM1
        (2, 0b0100), // bank 2 -> CM-RAM2
        (3, 0b0110), // bank 3 -> CM-RAM1 + CM-RAM2
        (4, 0b1000), // bank 4 -> CM-RAM3
        (5, 0b1010), // bank 5 -> CM-RAM1 + CM-RAM3
        (6, 0b1100), // bank 6 -> CM-RAM2 + CM-RAM3
        (7, 0b1110), // bank 7 -> CM-RAM1 + CM-RAM2 + CM-RAM3
    ];
    for (acc, lines) in cases {
        let mut harness = Cpu::new();
        harness.load(0, &[0xD0 | acc, 0xFD]); // LDM acc ; DCL
        harness.run_cycles(2);
        assert_eq!(harness.cpu.ram_bank(), lines, "acc={acc}");
    }

    // Accumulator bit 3 is ignored by the decode.
    let mut high_bit = Cpu::new();
    high_bit.load(0, &[0xD9, 0xFD]); // LDM 9 (1001) ; DCL
    high_bit.run_cycles(2);
    assert_eq!(high_bit.cpu.ram_bank(), 0b0010);
}

#[test]
fn src_drives_x2_x3_bus_and_requests_cm_ram_both_phases() {
    let mut harness = Cpu::new();
    harness.load(0, &[0x20, 0x68, 0x21]); // FIM P0, 0x68 ; SRC P0
    harness.run_cycles(2); // complete FIM; SRC decodes next

    // SRC spans X2+X3: X2 emits the chip/register nibble (0x6), X3 the character
    // nibble (0x8), completing the RAM address latch in the addressed 4002.
    let mut saw_x2 = false;
    let mut saw_x3 = false;
    for _ in 0..8 {
        let phase = harness.phase;
        harness.tick_phase();
        if harness.ctrl.io_op == Some(IoOp::Src) {
            match phase {
                BusCycle::X2 => {
                    assert_eq!(harness.bus.read(), 0x6);
                    saw_x2 = true;
                }
                BusCycle::X3 => {
                    assert_eq!(harness.bus.read(), 0x8);
                    saw_x3 = true;
                }
                _ => {}
            }
        }
    }
    assert!(saw_x2);
    assert!(saw_x3);

    // With SRC still decoded, the CPU exposes its phase-accurate control surface:
    // it drives the bus in X3, and needs CM-RAM asserted in both X2 and X3.
    assert!(harness.cpu.x3_cpu_drives_first());
    assert!(harness.cpu.x2_ram_bank_select());
    assert!(harness.cpu.x3_ram_bank_select());
    // SRC is not a peripheral read, so peripherals must not drive the bus in X3.
    assert_eq!(harness.cpu.x3_peripheral_io_op(), None);
}

#[test]
fn wrm_writes_accumulator_to_bus_and_asserts_io_op_in_x2() {
    let mut harness = Cpu::new();
    harness.load(0, &[0xDA, 0xE0]); // LDM 0xA ; WRM

    let mut saw_write = false;
    for _ in 0..2 * 8 {
        let phase = harness.phase;
        harness.tick_phase();
        if harness.ctrl.io_op == Some(IoOp::RamMainWrite) {
            // WRM is write-oriented: it drives the accumulator during X2 only.
            assert_eq!(phase, BusCycle::X2);
            assert_eq!(harness.bus.read(), 0xA);
            saw_write = true;
        }
    }
    assert!(saw_write);
}

#[test]
fn rdm_latches_the_bus_in_x3_into_the_accumulator() {
    let mut harness = Cpu::new();
    harness.read_value = 0xB; // value a selected 4002 would drive during X3
    harness.load(0, &[0xD0, 0xE9]); // LDM 0 ; RDM
    harness.run_cycles(2);
    assert_eq!(harness.acc(), 0xB);
}

#[test]
fn rdm_exposes_read_io_op_only_in_x3() {
    let mut harness = Cpu::new();
    harness.read_value = 0x5;
    harness.load(0, &[0xE9]); // RDM

    let mut saw_read = false;
    for _ in 0..8 {
        let phase = harness.phase;
        harness.tick_phase();
        if harness.ctrl.io_op == Some(IoOp::RamMainRead) {
            assert_eq!(phase, BusCycle::X3);
            saw_read = true;
        }
    }
    assert!(saw_read);
    // The CPU also advertises RDM as a peripheral X3 read while it stays decoded.
    assert_eq!(harness.cpu.x3_peripheral_io_op(), Some(IoOp::RamMainRead));
}

#[test]
fn adm_adds_the_bus_value_read_in_x3() {
    let mut harness = Cpu::new();
    harness.read_value = 4;
    harness.load(0, &[0xD3, 0xEB]); // LDM 3 ; ADM
    harness.run_cycles(2);
    assert_eq!(harness.acc(), 7); // 3 + 4 with carry-in 0
    assert!(!harness.carry());
}

// ---------------------------------------------------------------------------
// FIN: indirect ROM fetch through register pair 0 (two machine cycles)
// ---------------------------------------------------------------------------

#[test]
fn fin_fetches_rom_byte_addressed_by_pair_zero_into_target_pair() {
    let mut harness = Cpu::new();
    harness.cpu.registers.set_pair(0, 0x42); // indirect address, low 8 bits
    harness.load(0, &[0x32]); // FIN P1
    harness.load(0x042, &[0x6E]); // data fetched from the FIN's own page

    // First machine cycle fetches the FIN opcode; the pair is still untouched.
    harness.run_cycles(1);
    assert_eq!(harness.cpu.registers.get_pair(1), 0x00);

    // Second machine cycle sends R0R1 out as the ROM address and loads the
    // fetched byte into the target pair. FIN does not branch.
    harness.run_cycles(1);
    assert_eq!(harness.cpu.registers.get_pair(1), 0x6E);
    assert_eq!(harness.cpu.registers.get_pair(0), 0x42); // source pair unaffected
    assert_eq!(harness.pc(), 1);
}

#[test]
fn fin_into_pair_zero_replaces_the_address_source() {
    let mut harness = Cpu::new();
    harness.cpu.registers.set_pair(0, 0x10);
    harness.load(0, &[0x30]); // FIN P0
    harness.load(0x010, &[0xAB]);
    harness.run_cycles(2);
    assert_eq!(harness.cpu.registers.get_pair(0), 0xAB);
}

#[test]
fn fin_execution_does_not_disturb_the_following_instruction() {
    let mut harness = Cpu::new();
    harness.cpu.registers.set_pair(0, 0x20);
    harness.load(0, &[0x32, 0xD7]); // FIN P1 ; LDM 7
    harness.load(0x020, &[0x55]);
    harness.run_cycles(3); // FIN (2 cycles) + LDM (1 cycle)
    assert_eq!(harness.cpu.registers.get_pair(1), 0x55);
    assert_eq!(harness.acc(), 7);
    assert_eq!(harness.pc(), 2);
}

#[test]
fn fin_at_last_location_of_page_fetches_from_next_page() {
    // A FIN in the last location of a page takes its indirect page from the
    // incremented PC, so the data comes from the NEXT page.
    let mut harness = Cpu::new();
    harness.cpu.registers.set_pair(0, 0x3C);
    harness.load(0x0FF, &[0x3E]); // FIN P7 at the last byte of page 0
    harness.load(0x13C, &[0x9A]); // fetched from page 1, not page 0
    harness.load(0x03C, &[0x11]); // decoy on the FIN's own page
    harness.cpu.registers.set_pc(0x0FF);
    harness.run_cycles(2);
    assert_eq!(harness.cpu.registers.get_pair(7), 0x9A);
    assert_eq!(harness.pc(), 0x100);
}

// ---------------------------------------------------------------------------
// JCN TEST-pin condition: condition bit C1 jumps when the TEST pin is 0
// ---------------------------------------------------------------------------

#[test]
fn jcn_test_condition_jumps_when_test_pin_is_zero() {
    let mut harness = Cpu::new();
    harness.cpu.set_test_pin(false);
    harness.load(0, &[0x11, 0x0A]); // JCN test, 0x0A
    harness.run_cycles(2);
    assert_eq!(harness.pc(), 0x00A);
}

#[test]
fn jcn_test_condition_falls_through_when_test_pin_is_one() {
    let mut harness = Cpu::new();
    harness.cpu.set_test_pin(true);
    harness.load(0, &[0x11, 0x0A]); // JCN test, 0x0A
    harness.run_cycles(2);
    assert_eq!(harness.pc(), 2);
}

#[test]
fn jcn_inverted_test_condition_jumps_when_test_pin_is_one() {
    // Condition 0x9 = invert + test: jump when NOT (TEST == 0), i.e. TEST = 1.
    let mut harness = Cpu::new();
    harness.cpu.set_test_pin(true);
    harness.load(0, &[0x19, 0x0A]); // JCN invert|test, 0x0A
    harness.run_cycles(2);
    assert_eq!(harness.pc(), 0x00A);

    let mut fall_through = Cpu::new();
    fall_through.cpu.set_test_pin(false);
    fall_through.load(0, &[0x19, 0x0A]);
    fall_through.run_cycles(2);
    assert_eq!(fall_through.pc(), 2);
}
