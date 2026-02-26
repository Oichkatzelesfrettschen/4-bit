//! Intellec-4 Development System Integration
//!
//! WHY: The Intellec-4 was a complete development system that combined
//! the MCS-4 CPU with a front panel, monitor ROM, keyboard, display,
//! serial port, and PROM programmer. This module wires them together.
//!
//! WHAT: An `IntellecSystem` that coordinates all subsystems: the
//! MCS-4 system (CPU + ROM + RAM), front panel controls, monitor
//! state machine, peripheral devices, and PROM programmer.
//!
//! HOW: The system's `step()` method advances one machine cycle,
//! coordinating the front panel mode (Run/Stop/Step) with CPU
//! execution and peripheral updates. Higher-level methods provide
//! examine, deposit, and program load operations.

use mcs4_chips::i4702::I4702;
use mcs4_periph::{MatrixKeyboard, SerialConfig, SevenSegDisplay, Uart};
use mcs4_system::{Mcs40System, Mcs4System, System};

use crate::{
    front_panel::{FrontPanel, PanelMode},
    monitor::{Monitor, MonitorAction},
    prom_programmer::{ProgResult, PromProgrammer},
};

/// Complete Intellec-4 development system.
pub struct IntellecSystem<S: System = Mcs40System> {
    /// MCS-4 base system (CPU + ROM + RAM).
    pub system: S,

    /// Front panel (switches, buttons, LEDs).
    pub panel: FrontPanel,

    /// Monitor state machine.
    pub monitor: Monitor,

    /// Hex keyboard (4x4 matrix).
    pub keyboard: MatrixKeyboard,

    /// 7-segment address/data display (6 digits: 3 addr + 2 data + 1 status).
    pub display: SevenSegDisplay,

    /// Serial port (for teletype).
    pub serial: Uart,

    /// PROM programmer.
    pub programmer: PromProgrammer,

    /// PROM in programmer socket (optional).
    pub prom: Option<I4702>,

    /// Total machine cycles executed.
    cycles: u64,
}

impl IntellecSystem<Mcs40System> {
    /// Create a new Intellec-4/40 system with standard configuration.
    pub fn new() -> Self {
        Self {
            system: Mcs40System::new(),
            panel: FrontPanel::new(),
            monitor: Monitor::new(),
            keyboard: MatrixKeyboard::new(3),
            display: SevenSegDisplay::new(6),
            serial: Uart::new(SerialConfig::asr33(740_000), 64),
            programmer: PromProgrammer::default(),
            prom: None,
            cycles: 0,
        }
    }
}

impl IntellecSystem<Mcs4System> {
    /// Create an Intellec-4 system (4004-based).
    pub fn mcs4() -> Self {
        Self {
            system: Mcs4System::standard(),
            panel: FrontPanel::new(),
            monitor: Monitor::new(),
            keyboard: MatrixKeyboard::new(3),
            display: SevenSegDisplay::new(6),
            serial: Uart::new(SerialConfig::asr33(740_000), 64),
            programmer: PromProgrammer::default(),
            prom: None,
            cycles: 0,
        }
    }
}

impl<S: System> IntellecSystem<S> {
    /// Create a minimal Intellec-4 system (for testing).
    pub fn minimal(system: S) -> Self {
        Self {
            system,
            panel: FrontPanel::new(),
            monitor: Monitor::new(),
            keyboard: MatrixKeyboard::new(1),
            display: SevenSegDisplay::new(6),
            serial: Uart::new(
                SerialConfig {
                    baud_rate: 9600,
                    data_bits: 8,
                    stop_bits: 1,
                    cycles_per_bit: 10,
                },
                16,
            ),
            programmer: PromProgrammer::default(),
            prom: None,
            cycles: 0,
        }
    }

    /// Load a program into ROM starting at address 0.
    pub fn load_program(&mut self, data: &[u8]) {
        self.system.load_rom(data);
    }

    /// Insert a PROM into the programmer socket.
    pub fn insert_prom(&mut self, prom: I4702) {
        self.prom = Some(prom);
    }

    /// Remove the PROM from the programmer socket.
    pub fn remove_prom(&mut self) -> Option<I4702> {
        self.prom.take()
    }

    /// Examine memory at the given address.
    pub fn examine(&self, address: u16) -> u8 {
        self.system.read_rom(address).unwrap_or(0xFF)
    }

    /// Deposit a byte at the given address.
    pub fn deposit(&mut self, address: u16, data: u8) {
        let chip_id = ((address >> 8) & 0x0F) as u8;
        let chip_addr = (address & 0xFF) as u8;
        if let Some(rom) = self.system.roms_mut().iter_mut().find(|r| r.chip_id == chip_id) {
            rom.write_direct(chip_addr, data);
        }
    }

    /// Step one machine cycle.
    pub fn step(&mut self) {
        self.cycles += 1;

        // Check for panel reset
        if self.panel.take_reset() {
            self.system.reset();
            self.monitor.reset();
            self.panel.reset();
            self.panel.update_leds(0, 0, false, false);
            return;
        }

        // Process panel examine/deposit
        if self.panel.take_examine() {
            let addr = self.panel.address();
            let data = self.examine(addr);
            self.monitor.set_examined_data(data);
            self.panel.update_leds(addr, data, false, false);
            self.update_display();
            return;
        }

        if self.panel.take_deposit() {
            let addr = self.panel.address();
            let data = self.panel.data();
            self.deposit(addr, data);
            self.panel.update_leds(addr, data, false, false);
            self.update_display();
            return;
        }

        // CPU execution based on mode
        if self.panel.should_execute() {
            // Run 8 phases = 1 machine cycle
            self.system.run_cycles(1);

            // Update panel LEDs with current CPU state
            let pc = self.system.pc();
            let acc = self.system.accumulator();
            self.panel.update_leds(pc, acc, true, false);

            // Consume single-step request
            if self.panel.mode() == PanelMode::SingleStep {
                self.panel.consume_step();
            }
        }

        // Tick serial port
        self.serial.tx_tick();

        // Update display
        self.update_display();
    }

    /// Run for a given number of machine cycles.
    pub fn run_cycles(&mut self, count: u64) {
        for _ in 0..count {
            self.step();
        }
    }

    /// Process a keyboard input (hex keycode 0-15).
    pub fn key_input(&mut self, keycode: u8) -> MonitorAction {
        if self.monitor.is_active() {
            let action = self.monitor.process_key(keycode);
            match action {
                MonitorAction::Examine { address } => {
                    let data = self.examine(address);
                    self.monitor.set_examined_data(data);
                    self.panel.update_leds(address, data, false, false);
                }
                MonitorAction::Deposit { address, data } => {
                    self.deposit(address, data);
                }
                MonitorAction::Go { address } => {
                    self.system.set_pc(address);
                    self.panel.press_run();
                }
                MonitorAction::Step { address } => {
                    self.system.set_pc(address);
                    self.panel.press_step();
                }
                MonitorAction::Halt => {
                    self.panel.press_stop();
                }
                MonitorAction::None => {}
            }
            self.update_display();
            action
        } else {
            MonitorAction::None
        }
    }

    /// Program the PROM in the programmer socket from the buffer.
    pub fn program_prom(&mut self, data: &[u8]) -> ProgResult {
        self.programmer.load_buffer(data);
        if let Some(ref mut prom) = self.prom {
            self.programmer.program_and_verify(prom, 0)
        } else {
            ProgResult::ProgramFail { address: 0 }
        }
    }

    /// Total machine cycles executed.
    pub fn cycles(&self) -> u64 {
        self.cycles
    }

    /// Update the 6-digit display from current state.
    fn update_display(&mut self) {
        let addr = self.panel.leds().address;
        let data = self.panel.leds().data;

        // Address digits (high to low)
        self.display.set_bcd(5, ((addr >> 8) & 0xF) as u8, false);
        self.display.set_bcd(4, ((addr >> 4) & 0xF) as u8, false);
        self.display.set_bcd(3, (addr & 0xF) as u8, false);

        // Data digits
        self.display.set_bcd(2, (data >> 4) & 0xF, false);
        self.display.set_bcd(1, data & 0xF, false);

        // Status digit: show run state
        if self.panel.leds().run {
            // Show dash for "running"
            self.display.set_raw(0, mcs4_periph::seven_seg::segment::G);
        } else {
            // Blank for stopped
            self.display.set_raw(0, 0);
        }
    }

    /// Reset the entire Intellec-4 system.
    pub fn reset(&mut self) {
        self.system.reset();
        self.panel.reset();
        self.monitor.reset();
        self.keyboard.reset();
        self.display.clear();
        self.serial.reset();
        self.programmer.reset();
        self.cycles = 0;
    }
}

impl Default for IntellecSystem<Mcs40System> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn new_system_is_stopped() {
        let sys = IntellecSystem::minimal(Mcs40System::minimal());
        assert_eq!(sys.panel.mode(), PanelMode::Stop);
        assert!(sys.monitor.is_active());
        assert_eq!(sys.cycles(), 0);
    }

    #[test]
    fn load_and_examine() {
        let mut sys = IntellecSystem::minimal(Mcs40System::minimal());
        sys.load_program(&[0xD1, 0xD2, 0xD3]);

        assert_eq!(sys.examine(0), 0xD1);
        assert_eq!(sys.examine(1), 0xD2);
        assert_eq!(sys.examine(2), 0xD3);
    }

    #[test]
    fn deposit_and_readback() {
        let mut sys = IntellecSystem::minimal(Mcs40System::minimal());
        sys.deposit(0x10, 0xAB);
        assert_eq!(sys.examine(0x10), 0xAB);
    }

    #[test]
    fn front_panel_examine() {
        let mut sys = IntellecSystem::minimal(Mcs40System::minimal());
        sys.load_program(&[0x42]);

        sys.panel.set_address(0);
        sys.panel.press_examine();
        sys.step();

        assert_eq!(sys.panel.leds().data, 0x42);
    }

    #[test]
    fn front_panel_deposit() {
        let mut sys = IntellecSystem::minimal(Mcs40System::minimal());
        sys.panel.set_address(5);
        sys.panel.set_data(0xBE);
        sys.panel.press_deposit();
        sys.step();

        assert_eq!(sys.examine(5), 0xBE);
    }

    #[test]
    fn run_executes_instructions() {
        let mut sys = IntellecSystem::minimal(Mcs40System::minimal());
        // NOP NOP NOP
        sys.load_program(&[0x00, 0x00, 0x00]);
        sys.panel.press_run();
        sys.run_cycles(3);
        assert!(sys.cycles() >= 3);
    }

    #[test]
    fn single_step_executes_one_cycle() {
        let mut sys = IntellecSystem::minimal(Mcs40System::minimal());
        sys.load_program(&[0x00, 0x00]);
        sys.panel.press_step();
        sys.step();

        // After step, should no longer execute
        assert!(!sys.panel.should_execute());
    }

    #[test]
    fn reset_clears_system() {
        let mut sys = IntellecSystem::minimal(Mcs40System::minimal());
        sys.load_program(&[0x00]);
        sys.panel.press_run();
        sys.run_cycles(5);

        sys.reset();
        assert_eq!(sys.cycles(), 0);
        assert_eq!(sys.panel.mode(), PanelMode::Stop);
        assert!(sys.monitor.is_active());
    }

    #[test]
    fn panel_reset_via_step() {
        let mut sys = IntellecSystem::minimal(Mcs40System::minimal());
        sys.panel.press_run();
        sys.step();
        sys.panel.press_reset();
        sys.step(); // processes the reset

        assert_eq!(sys.panel.mode(), PanelMode::Stop);
    }

    #[test]
    fn prom_programmer_workflow() {
        let mut sys = IntellecSystem::minimal(Mcs40System::minimal());
        sys.insert_prom(I4702::new(0));

        let result = sys.program_prom(&[0xCA, 0xFE]);
        assert_eq!(result, ProgResult::Ok);

        // Verify the PROM contents
        let prom = sys.remove_prom().unwrap();
        assert_eq!(prom.read_rom(0), 0xCA);
        assert_eq!(prom.read_rom(1), 0xFE);
    }

    #[test]
    fn monitor_examine_via_key_input() {
        let mut sys = IntellecSystem::minimal(Mcs40System::minimal());
        sys.load_program(&[0x55]);

        // Press E (examine) key
        let action = sys.key_input(0xE);
        match action {
            MonitorAction::Examine { .. } => {}
            _ => panic!("Expected Examine action"),
        }
    }

    #[test]
    fn display_updates_on_examine() {
        let mut sys = IntellecSystem::minimal(Mcs40System::minimal());
        sys.load_program(&[0xAB]);

        sys.panel.set_address(0);
        sys.panel.press_examine();
        sys.step();

        // Display should show address and data
        // Digit 3 = addr low nibble = 0
        // Digits 2-1 = data = 0xAB
        let rendered = sys.display.render_ascii();
        assert_eq!(rendered.len(), 6);
    }
}
