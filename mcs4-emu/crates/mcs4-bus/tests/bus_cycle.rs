//! End-to-end MCS-4 machine-cycle integration tests.
//!
//! These drive the full 8-phase cycle (A1..X3) across `DataBus`,
//! `ControlSignals`, `CycleState`, and `TwoPhaseClock` together, exercising
//! the seams the unit tests skip: SYNC / CM assertion timing bound to the
//! crate's own `cycle_timing` contract, multi-master handoff on the shared
//! data bus, and mask-driven CM-RAM line selection.

use mcs4_bus::{cycle::cycle_timing, prelude::*};
use mcs4_core::prelude::SignalLevel;

/// Walk one full machine cycle and collect the observed phase sequence.
fn phase_walk() -> Vec<BusCycle> {
    let mut state = CycleState::new();
    let mut phases = Vec::with_capacity(8);
    for _ in 0..8 {
        phases.push(state.phase);
        state.advance();
    }
    phases
}

#[test]
fn machine_cycle_walks_all_eight_phases_in_order_then_wraps() {
    let phases = phase_walk();
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
    assert_eq!(phases, expected);

    // Phase numbers are the canonical 0..7 ordering.
    let numbers: Vec<u8> = phases.iter().map(|p| p.phase_number()).collect();
    assert_eq!(numbers, (0..8).collect::<Vec<u8>>());

    // After X3 the next phase wraps to A1 for the following cycle.
    assert_eq!(BusCycle::X3.next(), BusCycle::A1);
}

#[test]
fn sync_asserts_at_a1_and_deasserts_at_a2_per_timing_contract() {
    let mut ctrl = ControlSignals::mcs4();
    let mut state = CycleState::new();
    let mut sync_high_at = Vec::new();

    for tick in 0..8u64 {
        // Follow the crate's declared assertion points rather than magic phases.
        if state.phase == cycle_timing::SYNC_ASSERT {
            ctrl.assert_sync(tick);
        }
        if state.phase == cycle_timing::SYNC_DEASSERT {
            ctrl.deassert_sync(tick);
        }
        if ctrl.sync.current == SignalLevel::High {
            sync_high_at.push(state.phase);
        }
        state.advance();
    }

    // SYNC is high only across A1 (asserted at A1, cleared at A2).
    assert_eq!(sync_high_at, vec![BusCycle::A1]);
}

#[test]
fn cm_rom_becomes_valid_at_a3_phase() {
    let mut ctrl = ControlSignals::mcs4();
    let mut state = CycleState::new();

    // No ROM bank is selected until the A3 validity point is reached.
    assert_eq!(ctrl.selected_rom(), None);

    for tick in 0..8u64 {
        if state.phase == cycle_timing::CM_ROM_VALID {
            ctrl.select_rom(0x5, tick);
        }
        state.advance();
    }

    assert_eq!(ctrl.selected_rom(), Some(0x5));
    assert_eq!(ctrl.cm_rom(), 0x5);
}

#[test]
fn select_ram_is_mask_driven_bit0_is_cmram0() {
    let mut ctrl = ControlSignals::mcs4();

    // A mask with bit 0 set drives CM-RAM0 only; the decoded value is 1.
    ctrl.select_ram(0b0001, 0);
    assert_eq!(ctrl.cm_ram(), 0b0001);
    assert_eq!(ctrl.cm_ram[0].current, SignalLevel::High);
    assert_eq!(ctrl.cm_ram[1].current, SignalLevel::Low);
    assert_eq!(ctrl.cm_ram[2].current, SignalLevel::Low);
    assert_eq!(ctrl.cm_ram[3].current, SignalLevel::Low);

    // A mask with bit 2 set drives CM-RAM2, not "bank 2 as a number".
    ctrl.select_ram(0b0100, 10);
    assert_eq!(ctrl.cm_ram(), 0b0100);
    assert_eq!(ctrl.cm_ram[2].current, SignalLevel::High);
    assert_eq!(ctrl.cm_ram[0].current, SignalLevel::Low);
}

#[test]
fn select_ram_can_assert_multiple_cm_ram_lines_from_mask() {
    let mut ctrl = ControlSignals::mcs4();

    // A multi-bit mask lights every corresponding CM-RAM line simultaneously.
    ctrl.select_ram(0b1010, 0);
    assert_eq!(ctrl.cm_ram(), 0b1010);
    assert_eq!(ctrl.cm_ram[1].current, SignalLevel::High);
    assert_eq!(ctrl.cm_ram[3].current, SignalLevel::High);
    assert_eq!(ctrl.cm_ram[0].current, SignalLevel::Low);
    assert_eq!(ctrl.cm_ram[2].current, SignalLevel::Low);
}

#[test]
fn address_then_instruction_travel_over_shared_bus_with_master_handoff() {
    let mut bus = DataBus::new();
    let cpu = bus.add_driver("CPU");
    let rom = bus.add_driver("ROM");

    let addr = Address12::new(0xCBA);
    let instr = Byte8::new(0xD4); // JCN

    let mut state = CycleState::new();
    let mut read_a1 = 0u8;
    let mut read_a2 = 0u8;
    let mut read_a3 = 0u8;
    let mut read_m1 = 0u8;
    let mut read_m2 = 0u8;

    for tick in 0..8u64 {
        match state.phase {
            // Address phases: the CPU is bus master.
            BusCycle::A1 => {
                bus.drive(cpu, addr.nibble_a1(), tick);
                read_a1 = bus.read();
            }
            BusCycle::A2 => {
                bus.drive(cpu, addr.nibble_a2(), tick);
                read_a2 = bus.read();
            }
            BusCycle::A3 => {
                bus.drive(cpu, addr.nibble_a3(), tick);
                read_a3 = bus.read();
                // Release the CPU before the ROM takes over so no contention
                // window opens between masters.
                bus.release(cpu, tick);
            }
            // Memory phases: the ROM is bus master.
            BusCycle::M1 => {
                bus.drive(rom, instr.nibble_m1(), tick);
                read_m1 = bus.read();
            }
            BusCycle::M2 => {
                bus.drive(rom, instr.nibble_m2(), tick);
                read_m2 = bus.read();
                bus.release(rom, tick);
            }
            _ => {}
        }
        // The bus never enters contention during the handoff.
        assert!(!bus.has_contention(), "unexpected contention at {:?}", state.phase);
        state.advance();
    }

    let rebuilt_addr = Address12::from_nibbles(read_a1, read_a2, read_a3);
    assert_eq!(rebuilt_addr.value, 0xCBA);

    let rebuilt_instr = Byte8::from_nibbles(read_m1, read_m2);
    assert_eq!(rebuilt_instr.value, 0xD4);
    assert_eq!(rebuilt_instr.opr(), 0xD);
    assert_eq!(rebuilt_instr.opa(), 0x4);
}

#[test]
fn two_masters_driving_different_values_mark_bus_undefined() {
    let mut bus = DataBus::new();
    let cpu = bus.add_driver("CPU");
    let rom = bus.add_driver("ROM");

    bus.drive(cpu, 0b1010, 0);
    bus.drive(rom, 0b0101, 0);

    assert!(bus.has_contention());
    assert!(!bus.is_valid());
}

#[test]
fn released_bus_floats_high_impedance_not_logic_zero() {
    let mut bus = DataBus::new();
    let cpu = bus.add_driver("CPU");

    bus.drive(cpu, 0b1111, 0);
    assert!(bus.is_valid());

    bus.release(cpu, 10);
    // A floating bus is Z, which is not a defined logic level; consumers must
    // not treat the released bus as a driven 0.
    assert!(!bus.is_valid());
    assert!(!bus.has_contention());
    for line in &bus.lines {
        assert_eq!(line.current, SignalLevel::Z);
    }
}

#[test]
fn two_cycle_instruction_completes_after_two_machine_cycles() {
    let mut state = CycleState::new();
    state.set_two_cycle();

    // First machine cycle: instruction not yet retired; fetch of second byte pends.
    for _ in 0..8 {
        state.advance();
    }
    assert_eq!(state.cycle_count, 1);
    assert_eq!(state.instruction_count, 0);
    assert_eq!(state.state, MachineState::Fetch2);

    // Second machine cycle: instruction retires and the state machine rearms.
    for _ in 0..8 {
        state.advance();
    }
    assert_eq!(state.cycle_count, 2);
    assert_eq!(state.instruction_count, 1);
    assert_eq!(state.state, MachineState::Fetch1);
}

#[test]
fn two_phase_clock_never_overlaps_across_a_full_period() {
    let mut clock = TwoPhaseClock::default_config();
    let start = clock.cycle_count();

    // Tick far enough to cross at least one complete PHI1/PHI2 period. The
    // tick model advances internal phase time by variable widths, so drive it
    // by monotonically increasing simulation time.
    for tick in 0..2000u64 {
        clock.tick(tick);
        assert!(
            !(clock.phi1_high() && clock.phi2_high()),
            "PHI1 and PHI2 asserted simultaneously at tick {tick}"
        );
        if clock.cycle_count() > start {
            break;
        }
    }

    assert!(clock.cycle_count() > start, "clock did not complete a period");
}
