use mcs4_bus::prelude::*;
use mcs4_chips::i4040::I4040;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_random_execution_does_not_panic(instructions in prop::collection::vec(any::<u8>(), 1..100)) {
        let mut cpu = I4040::new();
        let mut bus = DataBus::new();
        let mut ctrl = ControlSignals::mcs40();

        // Load random bytes as ROM
        // We simulate instruction fetch by providing the byte on the bus
        // during M1/M2 phases.

        for &instr_byte in &instructions {
            // A1-A3: Address phases
            cpu.tick(BusCycle::A1, &mut bus, &mut ctrl);
            cpu.tick(BusCycle::A2, &mut bus, &mut ctrl);
            cpu.tick(BusCycle::A3, &mut bus, &mut ctrl);

            // M1: Provide low nibble
            bus.write(instr_byte & 0x0F);
            cpu.tick(BusCycle::M1, &mut bus, &mut ctrl);

            // M2: Provide high nibble
            bus.write((instr_byte >> 4) & 0x0F);
            cpu.tick(BusCycle::M2, &mut bus, &mut ctrl);

            // X1-X3: Execution phases
            cpu.tick(BusCycle::X1, &mut bus, &mut ctrl);
            cpu.tick(BusCycle::X2, &mut bus, &mut ctrl);
            cpu.tick(BusCycle::X3, &mut bus, &mut ctrl);

            if cpu.halted() {
                break;
            }
        }
    }
}
