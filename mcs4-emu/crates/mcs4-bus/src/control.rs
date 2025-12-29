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

/// Control signals for the MCS-4 bus
#[derive(Clone, Debug)]
pub struct ControlSignals {
    /// SYNC - Machine cycle synchronization
    pub sync: Signal,

    /// CM-ROM0 through CM-ROM3 - ROM bank select
    pub cm_rom: [Signal; 4],

    /// CM-RAM0 through CM-RAM3 - RAM bank select
    pub cm_ram: [Signal; 4],

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
            cm_ram: [
                Signal::new("CM-RAM0", SignalLevel::Low),
                Signal::new("CM-RAM1", SignalLevel::Low),
                Signal::new("CM-RAM2", SignalLevel::Low),
                Signal::new("CM-RAM3", SignalLevel::Low),
            ],
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
            cm_ram: [
                Signal::new("CM-RAM0", SignalLevel::Low),
                Signal::new("CM-RAM1", SignalLevel::Low),
                Signal::new("CM-RAM2", SignalLevel::Low),
                Signal::new("CM-RAM3", SignalLevel::Low),
            ],
            test: Signal::new("TEST", SignalLevel::High),
            reset: Signal::new("RESET", SignalLevel::Low),
            stp: Some(Signal::new("STP", SignalLevel::Low)),
            stop: Some(Signal::new("STOP", SignalLevel::Low)),
            int: Some(Signal::new("INT", SignalLevel::Low)),
        }
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
        for signal in &mut self.cm_rom {
            signal.update(time, SignalLevel::Low);
        }
    }

    /// Deselect all RAM banks
    pub fn deselect_ram(&mut self, time: Time) {
        for signal in &mut self.cm_ram {
            signal.update(time, SignalLevel::Low);
        }
    }

    /// Get currently selected ROM bank (if any)
    pub fn selected_rom(&self) -> Option<u8> {
        let mut bank = 0u8;
        let mut any_selected = false;

        for (i, signal) in self.cm_rom.iter().enumerate() {
            if signal.current == SignalLevel::High {
                bank |= 1 << i;
                any_selected = true;
            }
        }

        if any_selected { Some(bank) } else { None }
    }

    /// Get currently selected RAM bank (if any)
    pub fn selected_ram(&self) -> Option<u8> {
        let mut bank = 0u8;
        let mut any_selected = false;

        for (i, signal) in self.cm_ram.iter().enumerate() {
            if signal.current == SignalLevel::High {
                bank |= 1 << i;
                any_selected = true;
            }
        }

        if any_selected { Some(bank) } else { None }
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
        // In a full implementation, this would be decoded from control lines
        // For now, we use a simplified check based on ROM selection
        self.selected_rom().is_some()
    }

    /// Check if an I/O read operation is in progress
    /// This is set during X3 phase for RDR/RDM/RDx instructions
    pub fn is_io_read(&self) -> bool {
        // In a full implementation, this would be decoded from control lines
        self.selected_rom().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rom_select() {
        let mut ctrl = ControlSignals::mcs4();

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
}
