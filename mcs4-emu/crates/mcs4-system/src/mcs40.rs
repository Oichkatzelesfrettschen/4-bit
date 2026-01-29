//! MCS-40 System (4040-based)
//!
//! Complete system integration for Intel MCS-40 architecture.
//! Wires together 4040 CPU, 4201 Clock, and optional MCS-40 support chips.

use mcs4_bus::prelude::*;
use mcs4_chips::{i4001::I4001, i4002::I4002, i4040::I4040, i4201::I4201, i4289::I4289};

use crate::fixture::load_hex_bytes;

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
            BusCycle::X1 => {
                // No data transfer yet; keep RAM bank select inactive to avoid "always-on" behavior.
                self.control.deselect_ram(0);
                self.cpu.tick(phase, &mut self.bus, &mut self.control);
                for r in &mut self.ram {
                    r.tick_bus(phase, &mut self.bus, &self.control);
                }
                for r in &mut self.rom {
                    r.tick_bus(phase, &mut self.bus, &self.control);
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
                for r in &mut self.ram {
                    r.tick_bus(phase, &mut self.bus, &self.control);
                }
                for r in &mut self.rom {
                    r.tick_bus(phase, &mut self.bus, &self.control);
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
                    for r in &mut self.ram {
                        r.tick_bus(phase, &mut self.bus, &self.control);
                    }
                    for r in &mut self.rom {
                        r.tick_bus(phase, &mut self.bus, &self.control);
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
                    for r in &mut self.ram {
                        r.tick_bus(phase, &mut self.bus, &self.control);
                    }
                    for r in &mut self.rom {
                        r.tick_bus(phase, &mut self.bus, &self.control);
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

        self.cycle.advance();
        if self.cycle.phase == BusCycle::A1 {
            self.total_cycles += 1;
        }
    }

    /// Run for N machine cycles (8 bus phases per cycle)
    pub fn run_cycles(&mut self, cycles: usize) {
        for _ in 0..(cycles * 8) {
            self.step();
        }
    }

    pub fn load_rom(&mut self, data: &[u8]) {
        for (i, chunk) in data.chunks(256).enumerate() {
            if i < self.rom.len() {
                self.rom[i].load(chunk);
            }
        }
    }

    /// Load program from a file using memory mapping.
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

    /// Set the CPU test pin.
    pub fn set_test_pin(&mut self, state: bool) {
        self.cpu.set_test_pin(state);
    }

    pub fn pc(&self) -> u16 {
        self.cpu.pc()
    }

    pub fn phase(&self) -> BusCycle {
        self.cycle.phase
    }
}

impl Default for Mcs40System {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use mcs4_core::prelude::SignalLevel;

    use super::*;

    #[test]
    fn test_two_byte_jun_jumps_to_target() {
        let mut sys = Mcs40System::new();

        // JUN 0x005 (two-byte instruction): 0x40 0x05
        sys.load_rom(&[0x40, 0x05, 0x00, 0x00, 0x00, 0x00]);

        sys.run_cycles(1);
        assert_eq!(sys.pc(), 1);

        sys.run_cycles(1);
        assert_eq!(sys.pc(), 0x005);
    }

    #[test]
    fn test_two_byte_jms_and_bbl_return_address() {
        let mut sys = Mcs40System::new();

        // JMS 0x004 (two-byte instruction): 0x50 0x04
        // At 0x004: BBL 0xA
        sys.load_rom(&[0x50, 0x04, 0x00, 0x00, 0xCA]);

        sys.run_cycles(1);
        assert_eq!(sys.pc(), 1);

        sys.run_cycles(1);
        assert_eq!(sys.pc(), 0x004);

        sys.run_cycles(1);
        assert_eq!(sys.cpu.alu.accumulator(), 0xA);
        assert_eq!(sys.pc(), 0x002);
    }

    #[test]
    fn test_jcn_test_pin_not_taken_advances() {
        let mut sys = Mcs40System::new();

        // JCN TEST -> 0x005 (two-byte instruction): 0x11 0x05
        sys.load_rom(&[0x11, 0x05, 0x00, 0x00, 0x00, 0x00]);
        sys.set_test_pin(false);

        sys.run_cycles(1);
        assert_eq!(sys.pc(), 1);

        sys.run_cycles(1);
        assert_eq!(sys.pc(), 0x002);
    }

    #[test]
    fn test_jcn_test_pin_taken_jumps() {
        let mut sys = Mcs40System::new();

        // JCN TEST -> 0x005 (two-byte instruction): 0x11 0x05
        sys.load_rom(&[0x11, 0x05, 0x00, 0x00, 0x00, 0x00]);
        sys.set_test_pin(true);

        sys.run_cycles(1);
        assert_eq!(sys.pc(), 1);

        sys.run_cycles(1);
        assert_eq!(sys.pc(), 0x005);
    }

    #[test]
    fn test_wrr_writes_rom_io_port() {
        let mut sys = Mcs40System::new();

        // LDM 0xA; WRR; NOP
        sys.load_rom(&[0xDA, 0xE2, 0x00]);

        sys.run_cycles(2);

        assert_eq!(sys.rom[0].io_output(), 0xA);
    }

    #[test]
    fn test_end_to_end_src_wrm_rdm_roundtrip() {
        let mut sys = Mcs40System::new();

        sys.load_rom(&[
            0xDA, // LDM 0xA
            0x20, 0x01, // FIM P0, 0x01
            0x21, // SRC P0
            0xE0, // WRM
            0xD0, // LDM 0x0
            0xE9, // RDM
            0x00, // NOP
        ]);

        sys.run_cycles(10);

        assert_eq!(sys.cpu.alu.accumulator(), 0xA);
        assert_eq!(sys.ram[0].read_direct(0, 1), 0xA);
    }

    #[test]
    fn test_wrm_without_src_does_not_write() {
        let mut sys = Mcs40System::new();

        // LDM 0xA; WRM; NOP
        sys.load_rom(&[0xDA, 0xE0, 0x00]);

        // Run a few cycles; WRM should be ignored without a preceding SRC.
        sys.run_cycles(6);

        assert_eq!(sys.ram[0].read_direct(0, 0), 0x0);
    }

    #[test]
    fn test_fixture_src_wrm_rdm_hex_executes() {
        let mut sys = Mcs40System::new();

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("src_wrm_rdm.hex");
        sys.load_rom_hex_file(path).expect("load fixture");

        sys.run_cycles(10);

        assert_eq!(sys.cpu.alu.accumulator(), 0xA);
        assert_eq!(sys.ram[0].read_direct(0, 1), 0xA);
    }

    #[test]
    fn test_fixture_rom_port_wrr_rdr_hex_executes() {
        let mut sys = Mcs40System::new();

        sys.rom[0].set_io_input(0xC);

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("rom_port_wrr_rdr.hex");
        sys.load_rom_hex_file(path).expect("load fixture");

        sys.run_cycles(12);

        assert_eq!(sys.rom[0].io_output(), 0xA);
        assert_eq!(sys.cpu.alu.accumulator(), 0xC);
    }

    #[test]
    fn test_rom_io_op_is_phase_accurate() {
        let mut sys = Mcs40System::new();

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
        let mut sys = Mcs40System::new();

        // SRC P0 ; WRM ; RDM ; NOP
        sys.load_rom(&[0x21, 0xE0, 0xE9, 0x00]);

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
        let mut sys = Mcs40System::new();

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
                    assert_eq!(sys.control.selected_ram(), None);
                    saw_x1 = true;
                }
                BusCycle::X2 => {
                    if sys.control.is_io_write() {
                        assert!(sys.control.selected_ram().is_some());
                        saw_x2 = true;
                    }
                }
                BusCycle::X3 => {
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
    fn test_fixture_ram_status_wr1_rd1_hex_executes() {
        let mut sys = Mcs40System::new();

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("ram_status_wr1_rd1.hex");
        sys.load_rom_hex_file(path).expect("load fixture");

        sys.run_cycles(12);

        assert_eq!(sys.cpu.alu.accumulator(), 0xB);
        assert_eq!(sys.ram[0].read_status(1), 0xB);
    }

    #[test]
    fn test_io_op_is_phase_accurate() {
        let mut sys = Mcs40System::new();

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
    fn test_interrupt_ein_vectors_to_003_and_bbs_returns() {
        let mut sys = Mcs40System::new();

        // 0x000: EIN
        // 0x001: NOP (return address after interrupt)
        // 0x003: BBS (return from interrupt)
        sys.load_rom(&[0x0C, 0x00, 0x00, 0x02, 0x00]);

        sys.run_cycles(1);
        assert_eq!(sys.pc(), 1);

        // Raise INT before the next instruction boundary (sampled at A1).
        sys.control
            .int
            .as_mut()
            .expect("INT present")
            .update(0, SignalLevel::High);

        sys.run_cycles(1);

        // After servicing, PC returns to the interrupted instruction address.
        assert_eq!(sys.pc(), 1);
        assert!(!sys.cpu.intr.enabled());
        assert!(!sys.cpu.intr.pending());
    }
}
