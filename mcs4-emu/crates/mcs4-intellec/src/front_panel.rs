//! Intellec-4 Front Panel Emulation
//!
//! WHY: The Intellec-4 (and Intellec-4/MOD 40) provided a physical front
//! panel for program entry, debugging, and system control. Emulating this
//! panel enables authentic interaction with the MCS-4 system.
//!
//! WHAT: Toggle switches for address (12-bit) and data (8-bit) entry,
//! control buttons (Run, Stop, Single Step, Reset, Examine, Deposit),
//! and LED indicators mirroring the real hardware.
//!
//! HOW: The panel maintains switch/button state and drives the CPU
//! through the system interface. Run/Stop controls execution, Step
//! advances one instruction, Examine reads memory at the address
//! switches, and Deposit writes the data switches to memory.

/// Front panel operating mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelMode {
    /// CPU is running freely.
    Run,
    /// CPU is halted; front panel has control.
    Stop,
    /// CPU executes one instruction then halts.
    SingleStep,
}

/// LED indicator state.
#[derive(Clone, Copy, Debug, Default)]
pub struct PanelLeds {
    /// Address LEDs (12 bits): current address bus value.
    pub address: u16,
    /// Data LEDs (8 bits): current data bus value.
    pub data: u8,
    /// Run indicator: CPU is executing.
    pub run: bool,
    /// Wait indicator: CPU halted or in single-step.
    pub wait: bool,
    /// Interrupt indicator: interrupt request pending.
    pub interrupt: bool,
}

/// Front panel switch and button state.
#[derive(Clone, Debug)]
pub struct FrontPanel {
    /// Address toggle switches (12 bits).
    address_switches: u16,

    /// Data toggle switches (8 bits).
    data_switches: u8,

    /// Current operating mode.
    mode: PanelMode,

    /// LED indicators.
    leds: PanelLeds,

    /// Single-step request pending (consumed after one instruction).
    step_pending: bool,

    /// Deposit request pending.
    deposit_pending: bool,

    /// Examine request pending.
    examine_pending: bool,

    /// Reset request pending.
    reset_pending: bool,
}

impl FrontPanel {
    /// Create a new front panel in Stop mode.
    pub fn new() -> Self {
        Self {
            address_switches: 0,
            data_switches: 0,
            mode: PanelMode::Stop,
            leds: PanelLeds::default(),
            step_pending: false,
            deposit_pending: false,
            examine_pending: false,
            reset_pending: false,
        }
    }

    // --- Switch access ---

    /// Set the address toggle switches (12-bit, masked).
    pub fn set_address(&mut self, addr: u16) {
        self.address_switches = addr & 0x0FFF;
    }

    /// Read the address toggle switches.
    pub fn address(&self) -> u16 {
        self.address_switches
    }

    /// Set the data toggle switches (8-bit).
    pub fn set_data(&mut self, data: u8) {
        self.data_switches = data;
    }

    /// Read the data toggle switches.
    pub fn data(&self) -> u8 {
        self.data_switches
    }

    // --- Control buttons ---

    /// Press the Run button: begin continuous execution.
    pub fn press_run(&mut self) {
        self.mode = PanelMode::Run;
        self.step_pending = false;
    }

    /// Press the Stop button: halt execution.
    pub fn press_stop(&mut self) {
        self.mode = PanelMode::Stop;
    }

    /// Press the Single Step button: execute one instruction.
    pub fn press_step(&mut self) {
        self.mode = PanelMode::SingleStep;
        self.step_pending = true;
    }

    /// Press the Reset button: request system reset.
    pub fn press_reset(&mut self) {
        self.reset_pending = true;
    }

    /// Press the Examine button: read memory at address switches.
    pub fn press_examine(&mut self) {
        self.examine_pending = true;
    }

    /// Press the Deposit button: write data switches to memory.
    pub fn press_deposit(&mut self) {
        self.deposit_pending = true;
    }

    // --- Mode queries ---

    /// Current operating mode.
    pub fn mode(&self) -> PanelMode {
        self.mode
    }

    /// Check if the CPU should execute (Run mode or Step pending).
    pub fn should_execute(&self) -> bool {
        match self.mode {
            PanelMode::Run => true,
            PanelMode::SingleStep => self.step_pending,
            PanelMode::Stop => false,
        }
    }

    /// Consume the single-step request (call after executing one instruction).
    pub fn consume_step(&mut self) {
        if self.mode == PanelMode::SingleStep {
            self.step_pending = false;
        }
    }

    // --- Pending action queries ---

    /// Check and consume reset request.
    pub fn take_reset(&mut self) -> bool {
        let pending = self.reset_pending;
        self.reset_pending = false;
        pending
    }

    /// Check and consume examine request.
    pub fn take_examine(&mut self) -> bool {
        let pending = self.examine_pending;
        self.examine_pending = false;
        pending
    }

    /// Check and consume deposit request.
    pub fn take_deposit(&mut self) -> bool {
        let pending = self.deposit_pending;
        self.deposit_pending = false;
        pending
    }

    // --- LED access ---

    /// Get the current LED state.
    pub fn leds(&self) -> &PanelLeds {
        &self.leds
    }

    /// Update LED indicators from system state.
    pub fn update_leds(&mut self, address: u16, data: u8, running: bool, interrupt: bool) {
        self.leds.address = address & 0x0FFF;
        self.leds.data = data;
        self.leds.run = running;
        self.leds.wait = !running;
        self.leds.interrupt = interrupt;
    }

    /// Reset the front panel to power-on state.
    pub fn reset(&mut self) {
        self.mode = PanelMode::Stop;
        self.step_pending = false;
        self.deposit_pending = false;
        self.examine_pending = false;
        self.reset_pending = false;
        self.leds = PanelLeds::default();
    }
}

impl Default for FrontPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_panel_in_stop_mode() {
        let panel = FrontPanel::new();
        assert_eq!(panel.mode(), PanelMode::Stop);
        assert!(!panel.should_execute());
    }

    #[test]
    fn address_switches_masked_to_12_bits() {
        let mut panel = FrontPanel::new();
        panel.set_address(0xFFFF);
        assert_eq!(panel.address(), 0x0FFF);
    }

    #[test]
    fn data_switches_full_byte() {
        let mut panel = FrontPanel::new();
        panel.set_data(0xA5);
        assert_eq!(panel.data(), 0xA5);
    }

    #[test]
    fn run_mode_allows_execution() {
        let mut panel = FrontPanel::new();
        panel.press_run();
        assert_eq!(panel.mode(), PanelMode::Run);
        assert!(panel.should_execute());
    }

    #[test]
    fn stop_halts_execution() {
        let mut panel = FrontPanel::new();
        panel.press_run();
        panel.press_stop();
        assert_eq!(panel.mode(), PanelMode::Stop);
        assert!(!panel.should_execute());
    }

    #[test]
    fn single_step_executes_once() {
        let mut panel = FrontPanel::new();
        panel.press_step();
        assert!(panel.should_execute());

        panel.consume_step();
        assert!(!panel.should_execute());
        assert_eq!(panel.mode(), PanelMode::SingleStep);
    }

    #[test]
    fn pending_actions_consumed() {
        let mut panel = FrontPanel::new();

        panel.press_reset();
        assert!(panel.take_reset());
        assert!(!panel.take_reset()); // consumed

        panel.press_examine();
        assert!(panel.take_examine());
        assert!(!panel.take_examine());

        panel.press_deposit();
        assert!(panel.take_deposit());
        assert!(!panel.take_deposit());
    }

    #[test]
    fn led_update() {
        let mut panel = FrontPanel::new();
        panel.update_leds(0x123, 0xAB, true, false);

        assert_eq!(panel.leds().address, 0x123);
        assert_eq!(panel.leds().data, 0xAB);
        assert!(panel.leds().run);
        assert!(!panel.leds().wait);
        assert!(!panel.leds().interrupt);
    }

    #[test]
    fn led_wait_inverse_of_run() {
        let mut panel = FrontPanel::new();
        panel.update_leds(0, 0, false, false);
        assert!(!panel.leds().run);
        assert!(panel.leds().wait);
    }

    #[test]
    fn reset_clears_all_state() {
        let mut panel = FrontPanel::new();
        panel.press_run();
        panel.press_deposit();
        panel.press_examine();
        panel.update_leds(0xFFF, 0xFF, true, true);

        panel.reset();
        assert_eq!(panel.mode(), PanelMode::Stop);
        assert!(!panel.take_deposit());
        assert!(!panel.take_examine());
        assert_eq!(panel.leds().address, 0);
    }

    #[test]
    fn default_is_new() {
        let panel = FrontPanel::default();
        assert_eq!(panel.mode(), PanelMode::Stop);
        assert_eq!(panel.address(), 0);
        assert_eq!(panel.data(), 0);
    }
}
