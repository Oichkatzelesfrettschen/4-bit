//! Intel 4009 Standard I/O Expander
//!
//! The 4009 bridges the MCS-4 multiplexed bus to standard (non-MCS-4)
//! I/O devices. It latches I/O data during the X2 and X3 execution phases
//! and provides bidirectional data flow between the MCS-4 bus and external
//! I/O devices.
//!
//! It also decodes CM-RAM bank select lines for expanded memory addressing.

use mcs4_bus::prelude::*;

/// Intel 4009: Standard I/O Expander
///
/// Bridges MCS-4 bus to standard I/O by latching data during execution
/// phases and providing bidirectional buffering with bank select decode.
#[derive(Clone, Debug)]
pub struct I4009 {
    /// Output data latch (CPU -> external I/O, latched during X2)
    output_latch: u8,

    /// Input data latch (external I/O -> CPU, driven during X3)
    input_latch: u8,

    /// CM-RAM bank select output (decoded from control signals)
    ram_bank: Option<u8>,

    /// Current I/O operation direction
    direction: IoDirection,

    /// Chip enabled (active when CM-RAM or CM-ROM selects this device)
    enabled: bool,
}

/// I/O data flow direction
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IoDirection {
    /// No active transfer
    Idle,
    /// CPU writing to external I/O (X2 phase)
    Output,
    /// External I/O driving to CPU (X3 phase)
    Input,
}

impl I4009 {
    pub fn new() -> Self {
        Self {
            output_latch: 0,
            input_latch: 0,
            ram_bank: None,
            direction: IoDirection::Idle,
            enabled: false,
        }
    }

    /// Get the output data latch (data from CPU for external I/O)
    pub fn output(&self) -> u8 {
        self.output_latch & 0x0F
    }

    /// Set external input data (from I/O device, to be read by CPU)
    pub fn set_input(&mut self, data: u8) {
        self.input_latch = data & 0x0F;
    }

    /// Get the decoded RAM bank select output
    pub fn ram_bank(&self) -> Option<u8> {
        self.ram_bank
    }

    /// Current I/O direction
    pub fn direction(&self) -> IoDirection {
        self.direction
    }

    /// Whether the I/O expander is currently enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Process bus phases for I/O data transfer.
    pub fn tick_bus(&mut self, phase: BusCycle, bus: &mut DataBus, ctrl: &ControlSignals) {
        match phase {
            BusCycle::A3 => {
                // Decode chip select from CM-RAM lines
                self.ram_bank = ctrl.selected_ram();
                self.enabled = self.ram_bank.is_some() || ctrl.selected_rom().is_some();
            }
            BusCycle::X2 if self.enabled && ctrl.is_io_write() => {
                // Output phase: CPU drives bus, we latch it for external I/O
                self.output_latch = bus.read() & 0x0F;
                self.direction = IoDirection::Output;
            }
            BusCycle::X3 if self.enabled && ctrl.is_io_read() => {
                // Input phase: external I/O data driven onto bus for CPU
                bus.write(self.input_latch & 0x0F);
                self.direction = IoDirection::Input;
            }
            BusCycle::A1 => {
                // New cycle starts: clear direction
                self.direction = IoDirection::Idle;
            }
            _ => {}
        }
    }
}

impl Default for I4009 {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Chip for I4009 {
    fn name(&self) -> &'static str {
        "4009"
    }

    fn reset(&mut self) {
        self.output_latch = 0;
        self.input_latch = 0;
        self.ram_bank = None;
        self.direction = IoDirection::Idle;
        self.enabled = false;
    }

    fn tick(&mut self, _phase: BusCycle) {
        // Bus-connected operation handled by tick_bus()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctrl_with_ram_and_write() -> ControlSignals {
        let mut ctrl = ControlSignals::mcs4();
        ctrl.select_ram(0, 0);
        ctrl.set_io_op(IoOp::RamMainWrite);
        ctrl
    }

    fn ctrl_with_ram_and_read() -> ControlSignals {
        let mut ctrl = ControlSignals::mcs4();
        ctrl.select_ram(0, 0);
        ctrl.set_io_op(IoOp::RamMainRead);
        ctrl
    }

    #[test]
    fn output_latches_during_x2_write() {
        let mut io = I4009::new();
        let mut bus = DataBus::new();
        let ctrl = ctrl_with_ram_and_write();

        // Enable via A3
        bus.write(0);
        io.tick_bus(BusCycle::A3, &mut bus, &ctrl);
        assert!(io.is_enabled());

        // X2: CPU drives 0xA
        bus.write(0xA);
        io.tick_bus(BusCycle::X2, &mut bus, &ctrl);

        assert_eq!(io.output(), 0xA);
        assert_eq!(io.direction(), IoDirection::Output);
    }

    #[test]
    fn input_drives_bus_during_x3_read() {
        let mut io = I4009::new();
        let mut bus = DataBus::new();
        let ctrl = ctrl_with_ram_and_read();

        // Set external input
        io.set_input(0xC);

        // Enable via A3
        bus.write(0);
        io.tick_bus(BusCycle::A3, &mut bus, &ctrl);

        // X3: external I/O should drive bus
        io.tick_bus(BusCycle::X3, &mut bus, &ctrl);

        assert_eq!(bus.read(), 0xC);
        assert_eq!(io.direction(), IoDirection::Input);
    }

    #[test]
    fn not_enabled_without_chip_select() {
        let mut io = I4009::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs4(); // no RAM/ROM selected

        bus.write(0);
        io.tick_bus(BusCycle::A3, &mut bus, &ctrl);

        assert!(!io.is_enabled());
    }

    #[test]
    fn ram_bank_decoded() {
        let mut io = I4009::new();
        let mut bus = DataBus::new();
        let mut ctrl = ControlSignals::mcs4();
        ctrl.select_ram(3, 0);

        bus.write(0);
        io.tick_bus(BusCycle::A3, &mut bus, &ctrl);

        assert_eq!(io.ram_bank(), Some(3));
    }

    #[test]
    fn direction_resets_on_a1() {
        let mut io = I4009::new();
        let mut bus = DataBus::new();
        let ctrl = ctrl_with_ram_and_write();

        bus.write(0);
        io.tick_bus(BusCycle::A3, &mut bus, &ctrl);
        bus.write(0x5);
        io.tick_bus(BusCycle::X2, &mut bus, &ctrl);
        assert_eq!(io.direction(), IoDirection::Output);

        // A1 resets direction
        io.tick_bus(BusCycle::A1, &mut bus, &ctrl);
        assert_eq!(io.direction(), IoDirection::Idle);
    }

    #[test]
    fn reset_clears_all_state() {
        use crate::Chip;
        let mut io = I4009::new();
        let mut bus = DataBus::new();
        let ctrl = ctrl_with_ram_and_write();

        bus.write(0);
        io.tick_bus(BusCycle::A3, &mut bus, &ctrl);
        bus.write(0xF);
        io.tick_bus(BusCycle::X2, &mut bus, &ctrl);

        io.reset();

        assert_eq!(io.output(), 0);
        assert!(!io.is_enabled());
        assert_eq!(io.ram_bank(), None);
        assert_eq!(io.direction(), IoDirection::Idle);
    }

    #[test]
    fn data_masked_to_4_bits() {
        let mut io = I4009::new();
        io.set_input(0xFF);
        assert_eq!(io.input_latch, 0x0F);
    }

    #[test]
    fn chip_trait_name() {
        use crate::Chip;
        let io = I4009::new();
        assert_eq!(io.name(), "4009");
    }
}
