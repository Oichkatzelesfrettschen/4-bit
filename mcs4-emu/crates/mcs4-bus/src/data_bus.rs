//! 4-bit bidirectional data bus implementation

use mcs4_core::prelude::*;

/// 4-bit bidirectional data bus
///
/// The MCS-4 data bus carries addresses (A0-A11, sent as three 4-bit nibbles),
/// instructions (8-bit, sent as two 4-bit nibbles), and data.
#[derive(Clone, Debug)]
pub struct DataBus {
    /// D0-D3 signal lines
    pub lines: [Signal; 4],

    /// Current drivers (for bus contention detection)
    drivers: Vec<BusDriver>,
}

/// A device that can drive the bus
#[derive(Clone, Debug)]
pub struct BusDriver {
    /// Driver name (for debugging)
    pub name: String,

    /// Is this driver currently active?
    pub active: bool,

    /// Output value when active
    pub value: u8,
}

impl DataBus {
    /// Create a new data bus
    pub fn new() -> Self {
        Self {
            lines: [
                Signal::new("D0", SignalLevel::Z),
                Signal::new("D1", SignalLevel::Z),
                Signal::new("D2", SignalLevel::Z),
                Signal::new("D3", SignalLevel::Z),
            ],
            drivers: Vec::new(),
        }
    }

    /// Register a bus driver
    pub fn add_driver(&mut self, name: impl Into<String>) -> usize {
        let id = self.drivers.len();
        self.drivers.push(BusDriver {
            name: name.into(),
            active: false,
            value: 0,
        });
        id
    }

    /// Activate a driver and put a value on the bus
    pub fn drive(&mut self, driver_id: usize, value: u8, time: Time) {
        if let Some(driver) = self.drivers.get_mut(driver_id) {
            driver.active = true;
            driver.value = value & 0x0F;
        }
        self.resolve(time);
    }

    /// Deactivate a driver (tri-state)
    pub fn release(&mut self, driver_id: usize, time: Time) {
        if let Some(driver) = self.drivers.get_mut(driver_id) {
            driver.active = false;
        }
        self.resolve(time);
    }

    /// Resolve bus value from all active drivers
    fn resolve(&mut self, time: Time) {
        let active_drivers: Vec<_> = self.drivers.iter().filter(|d| d.active).collect();

        if active_drivers.is_empty() {
            // No drivers - bus floats
            for line in &mut self.lines {
                line.update(time, SignalLevel::Z);
            }
            return;
        }

        if active_drivers.len() > 1 {
            // Multiple drivers - check for contention
            let first_value = active_drivers[0].value;
            let contention = active_drivers.iter().any(|d| d.value != first_value);

            if contention {
                // Bus fight!
                for line in &mut self.lines {
                    line.update(time, SignalLevel::X);
                }
                tracing::warn!(
                    "Bus contention detected between {:?}",
                    active_drivers.iter().map(|d| &d.name).collect::<Vec<_>>()
                );
                return;
            }
        }

        // Single driver or all agree
        let value = active_drivers[0].value;
        for (i, line) in self.lines.iter_mut().enumerate() {
            let level = if (value >> i) & 1 == 1 {
                SignalLevel::High
            } else {
                SignalLevel::Low
            };
            line.update(time, level);
        }
    }

    /// Read current bus value (as 4-bit nibble)
    pub fn read(&self) -> u8 {
        let mut value = 0u8;
        for (i, line) in self.lines.iter().enumerate() {
            if line.current == SignalLevel::High {
                value |= 1 << i;
            }
        }
        value
    }

    /// Check if bus has valid data (not floating or contentious)
    pub fn is_valid(&self) -> bool {
        self.lines.iter().all(|l| l.current.is_defined())
    }

    /// Check for bus contention
    pub fn has_contention(&self) -> bool {
        self.lines.iter().any(|l| l.current == SignalLevel::X)
    }

    /// Get line by index
    pub fn line(&self, index: usize) -> Option<&Signal> {
        self.lines.get(index)
    }
}

impl Default for DataBus {
    fn default() -> Self {
        Self::new()
    }
}

/// 12-bit address formed from three bus cycles
#[derive(Clone, Copy, Debug, Default)]
pub struct Address12 {
    pub value: u16,
}

impl Address12 {
    pub fn new(value: u16) -> Self {
        Self { value: value & 0xFFF }
    }

    /// Build address from three 4-bit nibbles (A1, A2, A3 phases)
    pub fn from_nibbles(a1: u8, a2: u8, a3: u8) -> Self {
        let value = (a1 as u16) | ((a2 as u16) << 4) | ((a3 as u16) << 8);
        Self { value }
    }

    /// Get ROM page (upper 4 bits)
    pub fn page(&self) -> u8 {
        ((self.value >> 8) & 0xF) as u8
    }

    /// Get offset within page (lower 8 bits)
    pub fn offset(&self) -> u8 {
        (self.value & 0xFF) as u8
    }

    /// Get nibble for A1 phase (bits 0-3)
    pub fn nibble_a1(&self) -> u8 {
        (self.value & 0xF) as u8
    }

    /// Get nibble for A2 phase (bits 4-7)
    pub fn nibble_a2(&self) -> u8 {
        ((self.value >> 4) & 0xF) as u8
    }

    /// Get nibble for A3 phase (bits 8-11)
    pub fn nibble_a3(&self) -> u8 {
        ((self.value >> 8) & 0xF) as u8
    }
}

/// 8-bit instruction/data formed from two bus cycles
#[derive(Clone, Copy, Debug, Default)]
pub struct Byte8 {
    pub value: u8,
}

impl Byte8 {
    pub fn new(value: u8) -> Self {
        Self { value }
    }

    /// Build from two 4-bit nibbles (M1, M2 phases)
    pub fn from_nibbles(m1: u8, m2: u8) -> Self {
        Self {
            value: (m1 & 0xF) | ((m2 & 0xF) << 4),
        }
    }

    /// Get OPR (opcode) nibble (upper 4 bits)
    pub fn opr(&self) -> u8 {
        (self.value >> 4) & 0xF
    }

    /// Get OPA (operand/modifier) nibble (lower 4 bits)
    pub fn opa(&self) -> u8 {
        self.value & 0xF
    }

    /// Get nibble for M1 phase (bits 0-3)
    pub fn nibble_m1(&self) -> u8 {
        self.value & 0xF
    }

    /// Get nibble for M2 phase (bits 4-7)
    pub fn nibble_m2(&self) -> u8 {
        (self.value >> 4) & 0xF
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bus_drive() {
        let mut bus = DataBus::new();
        let cpu = bus.add_driver("CPU");

        bus.drive(cpu, 0b1010, 0);
        assert_eq!(bus.read(), 0b1010);
        assert!(bus.is_valid());
    }

    #[test]
    fn test_bus_release() {
        let mut bus = DataBus::new();
        let cpu = bus.add_driver("CPU");

        bus.drive(cpu, 0b1010, 0);
        bus.release(cpu, 100);

        assert!(!bus.is_valid()); // Bus is floating
    }

    #[test]
    fn test_bus_contention() {
        let mut bus = DataBus::new();
        let cpu = bus.add_driver("CPU");
        let rom = bus.add_driver("ROM");

        bus.drive(cpu, 0b1010, 0);
        bus.drive(rom, 0b0101, 0); // Different value!

        assert!(bus.has_contention());
    }

    #[test]
    fn test_address12() {
        let addr = Address12::from_nibbles(0xA, 0xB, 0xC);
        assert_eq!(addr.value, 0xCBA);
        assert_eq!(addr.page(), 0xC);
        assert_eq!(addr.offset(), 0xBA);
    }

    #[test]
    fn test_byte8() {
        let byte = Byte8::from_nibbles(0x4, 0xD); // D4 = JCN instruction
        assert_eq!(byte.value, 0xD4);
        assert_eq!(byte.opr(), 0xD);
        assert_eq!(byte.opa(), 0x4);
    }
}
