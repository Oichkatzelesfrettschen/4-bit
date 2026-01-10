//! Control signals for MCS-4/MCS-40 bus

use mcs4_core::prelude::*;

/// Chip select signals
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChipSelect {
    /// No chip selected
    None,
    /// ROM chip selected (CM-ROM active)
    Rom(u8),
    /// RAM chip selected (CM-RAM active)
    Ram(u8),
}

/// High-level bus I/O operation currently in progress (best-effort).
///
/// This is a pragmatic model used to avoid "always-on" peripheral behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IoOp {
    /// SRC: latch RAM chip/register/character address.
    Src,
    /// ROM port write/read (4001): WRR / RDR.
    RomPortWrite,
    RomPortRead,
    /// RAM main memory write/read (4002): WRM / RDM / ADM / SBM.
    RamMainWrite,
    RamMainRead,
    /// RAM output port write (4002): WMP.
    RamPortWrite,
    /// RAM status nibble write/read (4002): WR0-3 / RD0-3.
    RamStatusWrite(u8),
    RamStatusRead(u8),
}

/// Control signals for the MCS-4 bus
#[derive(Clone, Debug)]
pub struct ControlSignals {
    /// SYNC - Machine cycle synchronization
    pub sync: Signal,

    /// CM-ROM0 through CM-ROM3 - ROM bank select
    pub cm_rom: [Signal; 4],

    /// True if ROM chip-select is enabled (allows selecting bank 0 distinctly from "none").
    rom_enabled: bool,

    /// CM-RAM0 through CM-RAM3 - RAM bank select
    pub cm_ram: [Signal; 4],

    /// True if RAM chip-select is enabled (allows selecting bank 0 distinctly from "none").
    ram_enabled: bool,

    /// Decoded I/O operation for the current instruction/cycle.
    pub io_op: Option<IoOp>,

    /// TEST - External test input (active low)
    pub test: Signal,

    /// RESET - System reset (active low for 4004, active high for 4040)
    pub reset: Signal,

    // 4040-specific signals:
    /// STP - Stop acknowledge (4040 only)
    pub stp: Option<Signal>,

    /// STOP - Stop request input (4040 only)
    pub stop: Option<Signal>,

    /// INT - Interrupt request (4040 only)
    pub int: Option<Signal>,
}

impl ControlSignals {
    /// Create control signals for MCS-4 (4004) system
    pub fn mcs4() -> Self {
        Self {
            sync: Signal::new("SYNC", SignalLevel::Low),
            cm_rom: [
                Signal::new("CM-ROM0", SignalLevel::Low),
                Signal::new("CM-ROM1", SignalLevel::Low),
                Signal::new("CM-ROM2", SignalLevel::Low),
                Signal::new("CM-ROM3", SignalLevel::Low),
            ],
            rom_enabled: false,
            cm_ram: [
                Signal::new("CM-RAM0", SignalLevel::Low),
                Signal::new("CM-RAM1", SignalLevel::Low),
                Signal::new("CM-RAM2", SignalLevel::Low),
                Signal::new("CM-RAM3", SignalLevel::Low),
            ],
            ram_enabled: false,
            io_op: None,
            test: Signal::new("TEST", SignalLevel::High), // Active low
            reset: Signal::new("RESET", SignalLevel::Low),
            stp: None,
            stop: None,
            int: None,
        }
    }

    /// Create control signals for MCS-40 (4040) system
    pub fn mcs40() -> Self {
        Self {
            sync: Signal::new("SYNC", SignalLevel::Low),
            cm_rom: [
                Signal::new("CM-ROM0", SignalLevel::Low),
                Signal::new("CM-ROM1", SignalLevel::Low),
                Signal::new("CM-ROM2", SignalLevel::Low),
                Signal::new("CM-ROM3", SignalLevel::Low),
            ],
            rom_enabled: false,
            cm_ram: [
                Signal::new("CM-RAM0", SignalLevel::Low),
                Signal::new("CM-RAM1", SignalLevel::Low),
                Signal::new("CM-RAM2", SignalLevel::Low),
                Signal::new("CM-RAM3", SignalLevel::Low),
            ],
            ram_enabled: false,
            io_op: None,
            test: Signal::new("TEST", SignalLevel::High),
            reset: Signal::new("RESET", SignalLevel::Low),
            stp: Some(Signal::new("STP", SignalLevel::Low)),
            stop: Some(Signal::new("STOP", SignalLevel::Low)),
            int: Some(Signal::new("INT", SignalLevel::Low)),
        }
    }

    pub fn set_io_op(&mut self, op: IoOp) {
        self.io_op = Some(op);
    }

    pub fn clear_io_op(&mut self) {
        self.io_op = None;
    }

    /// Assert SYNC signal
    pub fn assert_sync(&mut self, time: Time) {
        self.sync.update(time, SignalLevel::High);
    }

    /// Deassert SYNC signal
    pub fn deassert_sync(&mut self, time: Time) {
        self.sync.update(time, SignalLevel::Low);
    }

    /// Select ROM bank (0-15)
    pub fn select_rom(&mut self, bank: u8, time: Time) {
        self.rom_enabled = true;
        let bank = bank & 0x0F;
        for (i, signal) in self.cm_rom.iter_mut().enumerate() {
            let level = if (bank >> i) & 1 == 1 {
                SignalLevel::High
            } else {
                SignalLevel::Low
            };
            signal.update(time, level);
        }
    }

    /// Select RAM bank (0-15)
    pub fn select_ram(&mut self, bank: u8, time: Time) {
        self.ram_enabled = true;
        let bank = bank & 0x0F;
        for (i, signal) in self.cm_ram.iter_mut().enumerate() {
            let level = if (bank >> i) & 1 == 1 {
                SignalLevel::High
            } else {
                SignalLevel::Low
            };
            signal.update(time, level);
        }
    }

    /// Deselect all ROM banks
    pub fn deselect_rom(&mut self, time: Time) {
        self.rom_enabled = false;
        for signal in &mut self.cm_rom {
            signal.update(time, SignalLevel::Low);
        }
    }

    /// Deselect all RAM banks
    pub fn deselect_ram(&mut self, time: Time) {
        self.ram_enabled = false;
        for signal in &mut self.cm_ram {
            signal.update(time, SignalLevel::Low);
        }
    }

    /// Get currently selected ROM bank (if any)
    pub fn selected_rom(&self) -> Option<u8> {
        if !self.rom_enabled {
            return None;
        }
        Some(self.cm_rom())
    }

    /// Get currently selected RAM bank (if any)
    pub fn selected_ram(&self) -> Option<u8> {
        if !self.ram_enabled {
            return None;
        }
        Some(self.cm_ram())
    }

    /// Get selected chip as a single enum value (best-effort).
    ///
    /// Note: MCS-4/MCS-40 can have both ROM and RAM selects present depending on phase and
    /// instruction. This helper is primarily for diagnostics and logging.
    pub fn selected_chip(&self) -> ChipSelect {
        match (self.selected_rom(), self.selected_ram()) {
            (Some(rom), _) => ChipSelect::Rom(rom),
            (None, Some(ram)) => ChipSelect::Ram(ram),
            (None, None) => ChipSelect::None,
        }
    }

    /// Check if TEST input is active (active low)
    pub fn test_active(&self) -> bool {
        self.test.current == SignalLevel::Low
    }

    /// Check if system is in reset
    pub fn in_reset(&self) -> bool {
        // 4004 reset is active low, 4040 is active high
        // For simplicity, treat High as "in reset"
        self.reset.current == SignalLevel::High
    }

    /// Assert reset
    pub fn assert_reset(&mut self, time: Time) {
        self.reset.update(time, SignalLevel::High);
    }

    /// Deassert reset
    pub fn deassert_reset(&mut self, time: Time) {
        self.reset.update(time, SignalLevel::Low);
    }

    /// Check if interrupt is pending (4040 only)
    pub fn interrupt_pending(&self) -> bool {
        self.int
            .as_ref()
            .map(|s| s.current == SignalLevel::High)
            .unwrap_or(false)
    }

    /// Check if stop is requested (4040 only)
    pub fn stop_requested(&self) -> bool {
        self.stop
            .as_ref()
            .map(|s| s.current == SignalLevel::High)
            .unwrap_or(false)
    }

    /// Get CM-ROM value as a 4-bit number
    pub fn cm_rom(&self) -> u8 {
        let mut value = 0u8;
        for (i, signal) in self.cm_rom.iter().enumerate() {
            if signal.current == SignalLevel::High {
                value |= 1 << i;
            }
        }
        value
    }

    /// Get CM-RAM value as a 4-bit number
    pub fn cm_ram(&self) -> u8 {
        let mut value = 0u8;
        for (i, signal) in self.cm_ram.iter().enumerate() {
            if signal.current == SignalLevel::High {
                value |= 1 << i;
            }
        }
        value
    }

    /// Check if an I/O write operation is in progress
    /// This is set during X2 phase for WRR/WRM/WMP/WRx instructions
    pub fn is_io_write(&self) -> bool {
        matches!(
            self.io_op,
            Some(IoOp::RamMainWrite | IoOp::RamPortWrite | IoOp::RomPortWrite | IoOp::RamStatusWrite(_) | IoOp::Src)
        )
    }

    /// Check if an I/O read operation is in progress
    /// This is set during X3 phase for RDR/RDM/RDx instructions
    pub fn is_io_read(&self) -> bool {
        matches!(
            self.io_op,
            Some(IoOp::RamMainRead | IoOp::RomPortRead | IoOp::RamStatusRead(_))
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rom_select() {
        let mut ctrl = ControlSignals::mcs4();

        ctrl.select_rom(0, 0);
        assert_eq!(ctrl.selected_rom(), Some(0));

        ctrl.select_rom(5, 0); // Binary 0101
        assert_eq!(ctrl.selected_rom(), Some(5));

        ctrl.deselect_rom(100);
        assert_eq!(ctrl.selected_rom(), None);
    }

    #[test]
    fn test_ram_select() {
        let mut ctrl = ControlSignals::mcs4();

        ctrl.select_ram(10, 0); // Binary 1010
        assert_eq!(ctrl.selected_ram(), Some(10));
    }

    #[test]
    fn test_sync() {
        let mut ctrl = ControlSignals::mcs4();

        assert_eq!(ctrl.sync.current, SignalLevel::Low);

        ctrl.assert_sync(100);
        assert_eq!(ctrl.sync.current, SignalLevel::High);

        ctrl.deassert_sync(200);
        assert_eq!(ctrl.sync.current, SignalLevel::Low);
    }

    #[test]
    fn test_mcs40_signals() {
        let ctrl = ControlSignals::mcs40();

        assert!(ctrl.stp.is_some());
        assert!(ctrl.stop.is_some());
        assert!(ctrl.int.is_some());
    }

    #[test]
    fn test_io_op_helpers() {
        let mut ctrl = ControlSignals::mcs4();
        assert!(!ctrl.is_io_write());
        assert!(!ctrl.is_io_read());

        ctrl.io_op = Some(IoOp::Src);
        assert!(ctrl.is_io_write());
        assert!(!ctrl.is_io_read());

        ctrl.io_op = Some(IoOp::RamMainWrite);
        assert!(ctrl.is_io_write());
        assert!(!ctrl.is_io_read());

        ctrl.io_op = Some(IoOp::RamMainRead);
        assert!(!ctrl.is_io_write());
        assert!(ctrl.is_io_read());

        ctrl.io_op = Some(IoOp::RomPortRead);
        assert!(!ctrl.is_io_write());
        assert!(ctrl.is_io_read());
    }
}
