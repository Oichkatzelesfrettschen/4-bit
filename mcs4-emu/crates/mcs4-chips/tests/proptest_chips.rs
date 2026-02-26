//! Property-based tests for MCS-40 support chips (D.2.1)
//!
//! Uses proptest to verify invariants across randomized inputs:
//! - 4201: phi1/phi2 never overlap at any frequency
//! - 4289: address nibble latching is independent
//! - 4308: ROM read roundtrip preserves data

#![allow(missing_docs)]

use mcs4_bus::prelude::*;
use mcs4_chips::{i4201::I4201, i4289::I4289, i4308::I4308};
use proptest::prelude::*;

// --- 4201 Clock Generator: phi1/phi2 never overlap ---

proptest! {
    #[test]
    fn phi1_phi2_never_overlap_random_frequency(freq in 100_000u64..2_000_000) {
        let mut clk = I4201::with_frequency(freq);
        for i in 0..500u64 {
            clk.step(i);
            prop_assert!(
                !(clk.phi1() && clk.phi2()),
                "phi1 and phi2 overlapped at freq={}Hz, step={}",
                freq,
                i
            );
        }
    }

    #[test]
    fn phi1_phi2_never_overlap_random_timesteps(steps in prop::collection::vec(0u64..100_000, 10..200)) {
        let mut clk = I4201::new();
        let mut time = 0u64;
        for &dt in &steps {
            time += dt;
            clk.step(time);
            prop_assert!(
                !(clk.phi1() && clk.phi2()),
                "phi1 and phi2 overlapped at time={}",
                time
            );
        }
    }

    #[test]
    fn clock_edges_are_valid(freq in 500_000u64..1_000_000) {
        let mut clk = I4201::with_frequency(freq);
        let mut last_phi1 = clk.phi1();
        let mut last_phi2 = clk.phi2();

        for i in 1..500u64 {
            let edge = clk.step(i);
            let phi1 = clk.phi1();
            let phi2 = clk.phi2();

            // Verify edge report matches actual state changes
            match edge {
                ClockEdge::Phi1Rising => {
                    prop_assert!(!last_phi1 && phi1, "Phi1Rising but phi1 did not go high at step {}", i);
                }
                ClockEdge::Phi1Falling => {
                    prop_assert!(last_phi1 && !phi1, "Phi1Falling but phi1 did not go low at step {}", i);
                }
                ClockEdge::Phi2Rising => {
                    prop_assert!(!last_phi2 && phi2, "Phi2Rising but phi2 did not go high at step {}", i);
                }
                ClockEdge::Phi2Falling => {
                    prop_assert!(last_phi2 && !phi2, "Phi2Falling but phi2 did not go low at step {}", i);
                }
                ClockEdge::None => {
                    // No edge: both phases should be unchanged
                    prop_assert_eq!(phi1, last_phi1, "phi1 changed without edge at step {}", i);
                    prop_assert_eq!(phi2, last_phi2, "phi2 changed without edge at step {}", i);
                }
            }

            last_phi1 = phi1;
            last_phi2 = phi2;
        }
    }
}

// --- 4289 Memory Interface: address nibble independence ---

proptest! {
    #[test]
    fn address_nibbles_independent(a1 in 0u8..16, a2 in 0u8..16, a3 in 0u8..16) {
        let mut iface = I4289::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs40();

        bus.write(a1);
        iface.tick_bus(BusCycle::A1, &mut bus, &ctrl);

        bus.write(a2);
        iface.tick_bus(BusCycle::A2, &mut bus, &ctrl);

        bus.write(a3);
        iface.tick_bus(BusCycle::A3, &mut bus, &ctrl);

        let addr = iface.address();
        prop_assert_eq!(addr & 0x00F, a1 as u16, "A1 nibble mismatch");
        prop_assert_eq!((addr >> 4) & 0x00F, a2 as u16, "A2 nibble mismatch");
        prop_assert_eq!((addr >> 8) & 0x00F, a3 as u16, "A3 nibble mismatch");
    }

    #[test]
    fn address_latching_order_independent(a1 in 0u8..16, a2 in 0u8..16, a3 in 0u8..16) {
        // Verify that re-latching a single nibble only affects that nibble
        let mut iface = I4289::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs40();

        // Set initial address
        bus.write(a1);
        iface.tick_bus(BusCycle::A1, &mut bus, &ctrl);
        bus.write(a2);
        iface.tick_bus(BusCycle::A2, &mut bus, &ctrl);
        bus.write(a3);
        iface.tick_bus(BusCycle::A3, &mut bus, &ctrl);

        // Re-latch A2 only with new value
        let new_a2 = (a2 + 1) % 16;
        bus.write(new_a2);
        iface.tick_bus(BusCycle::A2, &mut bus, &ctrl);

        let addr = iface.address();
        prop_assert_eq!(addr & 0x00F, a1 as u16, "A1 should be unchanged");
        prop_assert_eq!((addr >> 4) & 0x00F, new_a2 as u16, "A2 should be updated");
        // A3 is only checked during A3 phase, so it retains old value
        prop_assert_eq!((addr >> 8) & 0x00F, a3 as u16, "A3 should be unchanged");
    }

    #[test]
    fn data_nibbles_assemble_correctly(low in 0u8..16, high in 0u8..16) {
        let mut iface = I4289::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs40();

        bus.write(low);
        iface.tick_bus(BusCycle::M1, &mut bus, &ctrl);

        bus.write(high);
        iface.tick_bus(BusCycle::M2, &mut bus, &ctrl);

        let expected = high << 4 | low;
        prop_assert_eq!(iface.data(), expected, "data assembly mismatch");
    }
}

// --- 4308 ROM: read roundtrip preserves data ---

proptest! {
    #[test]
    fn rom_read_roundtrip(data in prop::collection::vec(any::<u8>(), 1024)) {
        let mut rom = I4308::new(0);
        rom.load(&data);

        // Verify every byte can be read back
        for (addr, &expected) in data.iter().enumerate() {
            let actual = rom.read_rom(addr as u16);
            prop_assert_eq!(actual, expected, "ROM mismatch at address 0x{:03X}", addr);
        }
    }

    #[test]
    fn rom_address_wraps_at_1024(addr in 0u16..1024, offset in 1u16..16) {
        let mut rom = I4308::new(0);
        let mut data = vec![0u8; 1024];
        data[addr as usize] = 0xAB;
        rom.load(&data);

        // Address with 1024 offset should read same byte
        let wrapped = addr + offset * 1024;
        prop_assert_eq!(
            rom.read_rom(addr),
            rom.read_rom(wrapped),
            "ROM wrap mismatch: addr={}, wrapped={}",
            addr,
            wrapped
        );
    }

    #[test]
    fn rom_bus_fetch_matches_direct_read(addr in 0u16..1024) {
        let mut rom = I4308::new(0);
        // Fill with address-based pattern
        let data: Vec<u8> = (0..1024).map(|i| (i & 0xFF) as u8).collect();
        rom.load(&data);

        let mut bus = DataBus::new();
        let mut ctrl = ControlSignals::mcs40();
        ctrl.select_rom(0, 0);

        // Latch address via bus phases
        bus.write((addr & 0x0F) as u8);
        rom.tick_bus(BusCycle::A1, &mut bus, &ctrl);

        bus.write(((addr >> 4) & 0x0F) as u8);
        rom.tick_bus(BusCycle::A2, &mut bus, &ctrl);

        bus.write(((addr >> 8) & 0x0F) as u8);
        rom.tick_bus(BusCycle::A3, &mut bus, &ctrl);

        // Read via bus
        rom.tick_bus(BusCycle::M1, &mut bus, &ctrl);
        let low = bus.read();

        rom.tick_bus(BusCycle::M2, &mut bus, &ctrl);
        let high = bus.read();

        let bus_byte = low | (high << 4);
        let direct_byte = rom.read_rom(addr);

        prop_assert_eq!(
            bus_byte, direct_byte,
            "bus fetch vs direct read mismatch at addr=0x{:03X}: bus=0x{:02X}, direct=0x{:02X}",
            addr, bus_byte, direct_byte
        );
    }

    #[test]
    fn rom_io_port_roundtrip(port in 0u8..4, value in 0u8..16) {
        let mut rom = I4308::new(0);
        rom.write_port(port, value);
        prop_assert_eq!(rom.read_port(port), value, "I/O port roundtrip mismatch");
    }

    #[test]
    fn rom_io_port_masks_to_4_bits(port in 0u8..4, value in 16u8..=255) {
        let mut rom = I4308::new(0);
        rom.write_port(port, value);
        let read = rom.read_port(port);
        prop_assert_eq!(read, value & 0x0F, "I/O port should mask to 4 bits");
        prop_assert!(read < 16, "I/O port value should be < 16");
    }
}
