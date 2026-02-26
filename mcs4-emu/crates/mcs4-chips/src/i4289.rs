//! Intel 4289 Standard Memory Interface
//!
//! The 4289 bridges the MCS-4 bus to standard memory (e.g. 2101 RAM, 1702 EPROM).
//! Organization:
//! - Latches 12-bit address from CPU (A1, A2, A3)
//! - Interfaces with CM-ROM/CM-RAM lines
//! - Provides 8-bit data path to standard memory

use mcs4_bus::prelude::*;

/// Intel 4289: Standard Memory Interface
///
/// Bridges the MCS-4 multiplexed bus to standard (non-MCS-4) memory.
/// Latches address during A1/A2/A3 phases, handles data during M1/M2.
/// Provides OE/WE control signals and 8-bit data path to external memory.
#[derive(Clone, Debug)]
pub struct I4289 {
    /// Latched 12-bit address from PC (A0-A11)
    pc_address: u16,

    /// Latched 8-bit address from SRC (X2-X3)
    src_latch: u8,

    /// Current bus phase
    phase: BusCycle,

    /// Chip selects
    cs_rom: bool,
    cs_ram: bool,

    /// 8-bit data latch (assembled from M1 low + M2 high nibbles)
    data_out: u8,

    /// 8-bit write data latch (assembled from X2 low + X3 high nibbles)
    write_data: u8,

    /// Read/write control
    read_mode: bool,

    /// Output Enable (active low): asserted during read operations
    oe_n: bool,

    /// Write Enable (active low): asserted during write operations
    we_n: bool,

    /// First/Last flip-flop (toggles on RPM/WPM)
    fl_flip_flop: bool,

    /// Whether the current instruction is RPM (Read Program Memory)
    rpm_instr: bool,

    /// Whether the current instruction is WPM (Write Program Memory)
    wpm_instr: bool,

    /// Address valid strobe
    address_valid: bool,
}

impl Default for I4289 {
    fn default() -> Self {
        Self {
            pc_address: 0,
            src_latch: 0,
            phase: BusCycle::A1,
            cs_rom: false,
            cs_ram: false,
            data_out: 0,
            write_data: 0,
            read_mode: true,
            oe_n: true, // inactive (high)
            we_n: true, // inactive (high)
            fl_flip_flop: false, // High (First)
            rpm_instr: false,
            wpm_instr: false,
            address_valid: false,
        }
    }
}

impl I4289 {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get current 12-bit address for external memory.
    ///
    /// Returns the combination of current page (A8-A11) and SRC latch (A0-A7)
    /// during RPM/WPM instructions, or pc_address otherwise.
    pub fn address(&self) -> u16 {
        if self.rpm_instr || self.wpm_instr {
            (self.pc_address & 0x0F00) | (self.src_latch as u16)
        } else {
            self.pc_address
        }
    }

    /// Get the 8-bit SRC address latch.
    pub fn src_latch(&self) -> u8 {
        self.src_latch
    }

    /// Get latched 8-bit read data (assembled from M1+M2 bus nibbles)
    pub fn data(&self) -> u8 {
        self.data_out
    }

    /// Get 8-bit write data (assembled from X2+X3 nibbles for RAM writes)
    pub fn write_data(&self) -> u8 {
        self.write_data
    }

    /// Whether in read mode (true) or write mode (false)
    pub fn is_read(&self) -> bool {
        self.read_mode
    }

    /// Output Enable (active low)
    pub fn oe_n(&self) -> bool {
        self.oe_n
    }

    /// Write Enable (active low)
    pub fn we_n(&self) -> bool {
        self.we_n
    }

    /// Whether address latch is valid (set after A3 phase)
    pub fn address_valid(&self) -> bool {
        self.address_valid
    }

    /// Whether any chip select is active
    pub fn chip_selected(&self) -> bool {
        self.cs_rom || self.cs_ram
    }

    /// Supply external read data (from memory to bus).
    /// Called by the system when external memory responds to a read.
    pub fn supply_read_data(&mut self, data: u8) {
        self.data_out = data;
    }

    /// Process bus phases.
    ///
    /// A1/A2/A3: latch address nibbles.
    /// M1/M2: latch instruction/data nibbles (from ROM during instruction fetch).
    /// X2/X3: handle I/O write data latching + control signal generation.
    pub fn tick_bus(&mut self, phase: BusCycle, bus: &mut DataBus, ctrl: &ControlSignals) {
        self.phase = phase;

        match phase {
            BusCycle::A1 => {
                // Start of new machine cycle: deassert control signals
                self.oe_n = true;
                self.we_n = true;
                self.address_valid = false;
                // Latch PC address bits 0-3
                self.pc_address = (self.pc_address & 0xFFF0) | (bus.read() as u16);
            }
            BusCycle::A2 => {
                // Latch PC address bits 4-7
                self.pc_address = (self.pc_address & 0xFF0F) | ((bus.read() as u16) << 4);
            }
            BusCycle::A3 => {
                // Latch PC address bits 8-11
                self.pc_address = (self.pc_address & 0xF0FF) | ((bus.read() as u16) << 8);
                self.address_valid = true;

                // Check if we are selected by CM-ROM/CM-RAM lines
                self.cs_rom = ctrl.selected_rom().is_some();
                self.cs_ram = ctrl.selected_ram().is_some();

                // Determine read/write from I/O operation
                self.read_mode = !ctrl.is_io_write();
            }
            BusCycle::M1 => {
                // Assert OE for read operations (ROM fetch or explicit read)
                if self.read_mode && self.chip_selected() {
                    self.oe_n = false;
                }
                // Latch low nibble of data byte
                self.data_out = (self.data_out & 0xF0) | (bus.read() & 0x0F);
            }
            BusCycle::M2 => {
                // Latch high nibble of data byte
                self.data_out = (self.data_out & 0x0F) | ((bus.read() & 0x0F) << 4);
                // Deassert OE after read data captured
                self.oe_n = true;

                // Decode instruction to track state
                let instr = self.data_out;
                if (instr & 0xF0) == 0x20 {
                    // SRC instruction (0x2X): reset F/L flip-flop to 'First'
                    self.fl_flip_flop = false;
                }
                self.rpm_instr = instr == 0x0E;
                self.wpm_instr = instr == 0xE3;
            }
            BusCycle::X1 => {
                // Decode phase
            }
            BusCycle::X2 => {
                // RPM/WPM behavior: assert PM (mode select)
                // For RPM, drive the bus during X3.
                // For WPM, latch the bus during X2/X3.

                // If SRC is being executed, latch the new address from the bus
                let instr = self.data_out;
                if (instr & 0xF0) == 0x20 {
                    // SRC latches register pair high nibble (address bits 4-7) during X2
                    self.src_latch = (self.src_latch & 0x0F) | ((bus.read() & 0x0F) << 4);
                }

                // Write data low nibble (if write operation)
                if (ctrl.is_io_write() || self.wpm_instr) && self.chip_selected() {
                    self.write_data = (self.write_data & 0xF0) | (bus.read() & 0x0F);
                    // Assert WE on write
                    self.we_n = false;
                }
            }
            BusCycle::X3 => {
                // If SRC is being executed, latch the low nibble from the bus
                let instr = self.data_out;
                if (instr & 0xF0) == 0x20 {
                    // SRC latches register pair low nibble (address bits 0-3) during X3
                    self.src_latch = (self.src_latch & 0xF0) | (bus.read() & 0x0F);
                }

                // Write data high nibble (if write operation)
                if (ctrl.is_io_write() || self.wpm_instr) && self.chip_selected() {
                    self.write_data = (self.write_data & 0x0F) | ((bus.read() & 0x0F) << 4);
                }
                // Deassert WE after write data presented
                self.we_n = true;

                // Handle RPM: drive bus with ROM nibble
                if self.rpm_instr && self.chip_selected() {
                    let nibble = if !self.fl_flip_flop {
                        (self.data_out >> 4) & 0x0F // Upper nibble (First)
                    } else {
                        self.data_out & 0x0F // Lower nibble (Last)
                    };
                    bus.write(nibble);
                }

                // Toggle F/L flip-flop after RPM/WPM execution
                if self.rpm_instr || self.wpm_instr {
                    self.fl_flip_flop = !self.fl_flip_flop;
                }
            }
        }
    }
}

impl super::Chip for I4289 {
    fn name(&self) -> &'static str {
        "4289"
    }
    fn reset(&mut self) {
        self.pc_address = 0;
        self.src_latch = 0;
        self.cs_rom = false;
        self.cs_ram = false;
        self.data_out = 0;
        self.write_data = 0;
        self.read_mode = true;
        self.oe_n = true;
        self.we_n = true;
        self.fl_flip_flop = false;
        self.rpm_instr = false;
        self.wpm_instr = false;
        self.address_valid = false;
    }
    fn tick(&mut self, _phase: BusCycle) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctrl_with_rom(rom_id: u8) -> ControlSignals {
        let mut ctrl = ControlSignals::mcs40();
        ctrl.select_rom(rom_id, 0);
        ctrl
    }

    #[test]
    fn address_latching_a1_a2_a3() {
        let mut iface = I4289::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs40();

        // A1: low nibble = 0x5
        bus.write(0x5);
        iface.tick_bus(BusCycle::A1, &mut bus, &ctrl);

        // A2: mid nibble = 0xA
        bus.write(0xA);
        iface.tick_bus(BusCycle::A2, &mut bus, &ctrl);

        // A3: high nibble = 0x3
        bus.write(0x3);
        iface.tick_bus(BusCycle::A3, &mut bus, &ctrl);

        // Address should be 0x3A5
        assert_eq!(iface.address(), 0x3A5);
    }

    #[test]
    fn chip_select_rom() {
        let mut iface = I4289::new();
        let mut bus = DataBus::new();
        let ctrl = make_ctrl_with_rom(0);

        bus.write(0);
        iface.tick_bus(BusCycle::A3, &mut bus, &ctrl);
        assert!(iface.cs_rom);
    }

    #[test]
    fn no_select_without_cm() {
        let mut iface = I4289::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs40(); // no ROM/RAM enabled

        bus.write(0);
        iface.tick_bus(BusCycle::A3, &mut bus, &ctrl);
        assert!(!iface.cs_rom);
        assert!(!iface.cs_ram);
    }

    #[test]
    fn reset_clears_address() {
        use crate::Chip;
        let mut iface = I4289::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs40();

        bus.write(0xF);
        iface.tick_bus(BusCycle::A1, &mut bus, &ctrl);
        assert_ne!(iface.address(), 0);

        iface.reset();
        assert_eq!(iface.address(), 0);
        assert!(!iface.cs_rom);
        assert!(!iface.cs_ram);
    }

    #[test]
    fn data_latching_m1_m2() {
        let mut iface = I4289::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs40();

        // M1: low nibble = 0xB
        bus.write(0xB);
        iface.tick_bus(BusCycle::M1, &mut bus, &ctrl);

        // M2: high nibble = 0x7
        bus.write(0x7);
        iface.tick_bus(BusCycle::M2, &mut bus, &ctrl);

        // Data should be 0x7B
        assert_eq!(iface.data(), 0x7B);
    }

    #[test]
    fn address_nibbles_independent() {
        let mut iface = I4289::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs40();

        // Set A1 = 0xF
        bus.write(0xF);
        iface.tick_bus(BusCycle::A1, &mut bus, &ctrl);
        assert_eq!(iface.address() & 0x00F, 0xF);

        // Set A2 = 0x0, should not touch A1
        bus.write(0x0);
        iface.tick_bus(BusCycle::A2, &mut bus, &ctrl);
        assert_eq!(iface.address() & 0x00F, 0xF);
        assert_eq!(iface.address() & 0x0F0, 0x0);
    }

    // --- C.2.1: Write mode ---

    #[test]
    fn write_data_latching_x2_x3() {
        let mut iface = I4289::new();
        let mut bus = DataBus::new();
        let mut ctrl = ControlSignals::mcs40();
        ctrl.select_ram(0, 0);
        ctrl.set_io_op(IoOp::RamMainWrite);

        // A3 with write op sets write mode
        bus.write(0);
        iface.tick_bus(BusCycle::A3, &mut bus, &ctrl);
        assert!(!iface.is_read());

        // X2: low nibble of write data = 0xC
        bus.write(0xC);
        iface.tick_bus(BusCycle::X2, &mut bus, &ctrl);

        // X3: high nibble of write data = 0x3
        bus.write(0x3);
        iface.tick_bus(BusCycle::X3, &mut bus, &ctrl);

        assert_eq!(iface.write_data(), 0x3C);
    }

    // --- C.2.2: Chip select decode ---

    #[test]
    fn chip_select_ram() {
        let mut iface = I4289::new();
        let mut bus = DataBus::new();
        let mut ctrl = ControlSignals::mcs40();
        ctrl.select_ram(2, 0);

        bus.write(0);
        iface.tick_bus(BusCycle::A3, &mut bus, &ctrl);
        assert!(iface.cs_ram);
        assert!(iface.chip_selected());
    }

    // --- C.2.3: OE/WE control signals ---

    #[test]
    fn oe_asserted_during_read() {
        let mut iface = I4289::new();
        let mut bus = DataBus::new();
        let ctrl = make_ctrl_with_rom(0);

        // Go through address phases
        bus.write(0);
        iface.tick_bus(BusCycle::A1, &mut bus, &ctrl);
        assert!(iface.oe_n()); // deasserted at A1

        iface.tick_bus(BusCycle::A2, &mut bus, &ctrl);
        iface.tick_bus(BusCycle::A3, &mut bus, &ctrl);

        // M1: OE should be asserted (low) for read
        bus.write(0xB);
        iface.tick_bus(BusCycle::M1, &mut bus, &ctrl);
        assert!(!iface.oe_n(), "OE should be asserted (low) during read M1");

        // M2: OE deasserted
        bus.write(0x7);
        iface.tick_bus(BusCycle::M2, &mut bus, &ctrl);
        assert!(iface.oe_n(), "OE should deassert after M2");
    }

    #[test]
    fn we_asserted_during_write() {
        let mut iface = I4289::new();
        let mut bus = DataBus::new();
        let mut ctrl = ControlSignals::mcs40();
        ctrl.select_ram(0, 0);
        ctrl.set_io_op(IoOp::RamMainWrite);

        // Address phases
        bus.write(0);
        iface.tick_bus(BusCycle::A1, &mut bus, &ctrl);
        iface.tick_bus(BusCycle::A2, &mut bus, &ctrl);
        iface.tick_bus(BusCycle::A3, &mut bus, &ctrl);

        // X2: WE asserted during write
        bus.write(0xA);
        iface.tick_bus(BusCycle::X2, &mut bus, &ctrl);
        assert!(!iface.we_n(), "WE should be asserted (low) during write X2");

        // X3: WE deasserted after data captured
        bus.write(0x5);
        iface.tick_bus(BusCycle::X3, &mut bus, &ctrl);
        assert!(iface.we_n(), "WE should deassert after X3");
    }

    // --- C.2.4: Address valid ---

    #[test]
    fn address_valid_after_a3() {
        let mut iface = I4289::new();
        let mut bus = DataBus::new();
        let ctrl = ControlSignals::mcs40();

        assert!(!iface.address_valid());

        bus.write(0);
        iface.tick_bus(BusCycle::A1, &mut bus, &ctrl);
        assert!(!iface.address_valid());

        iface.tick_bus(BusCycle::A2, &mut bus, &ctrl);
        assert!(!iface.address_valid());

        iface.tick_bus(BusCycle::A3, &mut bus, &ctrl);
        assert!(iface.address_valid());
    }

    // --- C.2.5: Supply read data ---

    #[test]
    fn supply_external_read_data() {
        let mut iface = I4289::new();
        iface.supply_read_data(0xDE);
        assert_eq!(iface.data(), 0xDE);
    }

    #[test]
    fn test_src_and_rpm_addressing() {
        let mut iface = I4289::new();
        let mut bus = DataBus::new();
        let mut ctrl = ControlSignals::mcs40();
        ctrl.select_rom(0, 0);

        // 1. Fetch SRC P0 (at PC=0x123)
        // A1-A3: PC=0x123
        bus.write(0x3); iface.tick_bus(BusCycle::A1, &mut bus, &ctrl);
        bus.write(0x2); iface.tick_bus(BusCycle::A2, &mut bus, &ctrl);
        bus.write(0x1); iface.tick_bus(BusCycle::A3, &mut bus, &ctrl);
        assert_eq!(iface.address(), 0x123);

        // M1-M2: Opcode 0x21 (SRC P0)
        bus.write(0x1); iface.tick_bus(BusCycle::M1, &mut bus, &ctrl);
        bus.write(0x2); iface.tick_bus(BusCycle::M2, &mut bus, &ctrl);

        // X2-X3: SRC address 0x68 (High=6, Low=8)
        bus.write(0x6); iface.tick_bus(BusCycle::X2, &mut bus, &ctrl);
        bus.write(0x8); iface.tick_bus(BusCycle::X3, &mut bus, &ctrl);
        
        // After SRC, address register bits 0-7 should be 0x68
        assert_eq!(iface.src_latch, 0x68);

        // 2. Fetch RPM (at PC=0x124)
        // A1-A3: PC=0x124
        bus.write(0x4); iface.tick_bus(BusCycle::A1, &mut bus, &ctrl);
        bus.write(0x2); iface.tick_bus(BusCycle::A2, &mut bus, &ctrl);
        bus.write(0x1); iface.tick_bus(BusCycle::A3, &mut bus, &ctrl);
        assert_eq!(iface.address(), 0x124); // Driving PC during fetch

        // M1-M2: Opcode 0x0E (RPM)
        bus.write(0xE); iface.tick_bus(BusCycle::M1, &mut bus, &ctrl);
        bus.write(0x0); iface.tick_bus(BusCycle::M2, &mut bus, &ctrl);

        // During RPM execution phases, address should switch to combination of PC page (0x1) and SRC latch (0x68)
        iface.tick_bus(BusCycle::X1, &mut bus, &ctrl);
        assert_eq!(iface.address(), 0x168);
        
        iface.tick_bus(BusCycle::X2, &mut bus, &ctrl);
        assert_eq!(iface.address(), 0x168);

        iface.tick_bus(BusCycle::X3, &mut bus, &ctrl);
        assert_eq!(iface.address(), 0x168);
    }

    #[test]
    fn chip_trait_name() {
        use crate::Chip;
        let iface = I4289::new();
        assert_eq!(iface.name(), "4289");
    }
}
