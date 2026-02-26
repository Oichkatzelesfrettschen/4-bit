//! Intel 4308 1024x8 ROM with I/O
//!
//! The 4308 contains 8192 bits of mask-programmable ROM organized as 1024 words
//! by 8 bits. It also includes 4 x 4-bit I/O ports accessible via WRR/RDR
//! instructions.
//!
//! Bus protocol:
//! - A1/A2/A3: latches 12-bit address (uses bits 0-9 for ROM, ignores 10-11)
//! - A3: checks CM-ROM for chip select
//! - M1/M2: outputs instruction/data byte (low nibble, then high nibble)
//! - X2: latches I/O write data (WRR instruction)
//! - X3: outputs I/O read data (RDR instruction)

use mcs4_bus::prelude::*;

/// Intel 4308: 1024x8 ROM
#[derive(Clone, Debug)]
pub struct I4308 {
    /// 1024 x 8-bit memory
    memory: Vec<u8>,

    /// 4-bit I/O ports (4 ports of 4 bits)
    ports: [u8; 4],

    /// Chip ID for CM-ROM matching (0-15)
    pub chip_id: u8,

    /// Latched 10-bit ROM address
    latched_address: u16,

    /// Whether this chip is selected by CM-ROM
    selected: bool,

    /// Currently addressed I/O port (from SRC)
    io_port: u8,
}

impl I4308 {
    pub fn new(chip_id: u8) -> Self {
        Self {
            memory: vec![0; 1024],
            ports: [0; 4],
            chip_id,
            latched_address: 0,
            selected: false,
            io_port: 0,
        }
    }

    pub fn load(&mut self, data: &[u8]) {
        let len = data.len().min(self.memory.len());
        self.memory[..len].copy_from_slice(&data[..len]);
    }

    pub fn read_rom(&self, address: u16) -> u8 {
        self.memory[(address & 0x3FF) as usize]
    }

    /// Read an I/O port (0-3)
    pub fn read_port(&self, port: u8) -> u8 {
        self.ports[(port & 3) as usize] & 0x0F
    }

    /// Write an I/O port (0-3)
    pub fn write_port(&mut self, port: u8, data: u8) {
        self.ports[(port & 3) as usize] = data & 0x0F;
    }

    /// Whether this chip is currently selected
    pub fn is_selected(&self) -> bool {
        self.selected
    }

    /// Process a bus phase for ROM fetch and I/O operations.
    pub fn tick_bus(&mut self, phase: BusCycle, bus: &mut DataBus, ctrl: &ControlSignals) {
        match phase {
            BusCycle::A1 => {
                // Latch address bits 0-3
                self.latched_address = (self.latched_address & 0xFFF0) | (bus.read() as u16);
            }
            BusCycle::A2 => {
                // Latch address bits 4-7
                self.latched_address = (self.latched_address & 0xFF0F) | ((bus.read() as u16) << 4);
            }
            BusCycle::A3 => {
                // Latch address bits 8-11
                self.latched_address = (self.latched_address & 0xF0FF) | ((bus.read() as u16) << 8);

                // Check CM-ROM select
                self.selected = ctrl.selected_rom() == Some(self.chip_id);
            }
            BusCycle::M1 => {
                // Output low nibble of addressed ROM byte
                if self.selected {
                    let byte = self.read_rom(self.latched_address);
                    bus.write(byte & 0x0F);
                }
            }
            BusCycle::M2 => {
                // Output high nibble of addressed ROM byte
                if self.selected {
                    let byte = self.read_rom(self.latched_address);
                    bus.write((byte >> 4) & 0x0F);
                }
            }
            BusCycle::X1 => {
                // Decode: no action from ROM
            }
            BusCycle::X2 => {
                // I/O write: WRR instruction writes to ROM I/O port
                if self.selected && ctrl.io_op == Some(IoOp::RomPortWrite) {
                    let data = bus.read() & 0x0F;
                    self.ports[self.io_port as usize] = data;
                }

                // SRC: latch I/O port address
                if self.selected && ctrl.io_op == Some(IoOp::Src) {
                    self.io_port = bus.read() & 0x03;
                }
            }
            BusCycle::X3 => {
                // I/O read: RDR instruction reads ROM I/O port
                if self.selected && ctrl.io_op == Some(IoOp::RomPortRead) {
                    bus.write(self.ports[self.io_port as usize] & 0x0F);
                }
            }
        }
    }
}

impl super::Chip for I4308 {
    fn name(&self) -> &'static str {
        "4308"
    }

    fn reset(&mut self) {
        // ROM contents preserved
        self.ports = [0; 4];
        self.latched_address = 0;
        self.selected = false;
        self.io_port = 0;
    }

    fn tick(&mut self, _phase: BusCycle) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctrl_rom(id: u8) -> ControlSignals {
        let mut ctrl = ControlSignals::mcs40();
        ctrl.select_rom(id, 0);
        ctrl
    }

    #[test]
    fn load_and_read_roundtrip() {
        let mut rom = I4308::new(0);
        let data: Vec<u8> = (0..=255).collect();
        rom.load(&data);

        for i in 0u16..256 {
            assert_eq!(rom.read_rom(i), i as u8);
        }
    }

    #[test]
    fn address_wraps_at_1024() {
        let mut rom = I4308::new(0);
        rom.load(&[0xAB]);
        assert_eq!(rom.read_rom(0), rom.read_rom(1024));
    }

    #[test]
    fn io_ports_init_zero() {
        let rom = I4308::new(0);
        assert_eq!(rom.ports, [0; 4]);
    }

    #[test]
    fn reset_preserves_rom_clears_ports() {
        use crate::Chip;
        let mut rom = I4308::new(0);
        rom.load(&[0xDE, 0xAD]);
        rom.ports = [1, 2, 3, 4];

        rom.reset();
        assert_eq!(rom.read_rom(0), 0xDE);
        assert_eq!(rom.read_rom(1), 0xAD);
        assert_eq!(rom.ports, [0; 4]);
    }

    #[test]
    fn chip_id_stored() {
        let rom = I4308::new(5);
        assert_eq!(rom.chip_id, 5);
    }

    #[test]
    fn load_truncates_to_1024() {
        let mut rom = I4308::new(0);
        let data: Vec<u8> = vec![0xFF; 2048];
        rom.load(&data);
        assert_eq!(rom.read_rom(1023), 0xFF);
    }

    #[test]
    fn chip_trait_name() {
        use crate::Chip;
        let rom = I4308::new(0);
        assert_eq!(rom.name(), "4308");
    }

    // --- C.3.1: tick_bus address latching + data output ---

    #[test]
    fn tick_bus_fetch_byte() {
        let mut rom = I4308::new(0);
        rom.load(&[0xD4]); // JCN at address 0
        let mut bus = DataBus::new();
        let ctrl = make_ctrl_rom(0);

        // Address phase: address = 0x000
        bus.write(0x0);
        rom.tick_bus(BusCycle::A1, &mut bus, &ctrl);
        bus.write(0x0);
        rom.tick_bus(BusCycle::A2, &mut bus, &ctrl);
        bus.write(0x0);
        rom.tick_bus(BusCycle::A3, &mut bus, &ctrl);

        assert!(rom.is_selected());

        // M1: low nibble of 0xD4 = 0x4
        rom.tick_bus(BusCycle::M1, &mut bus, &ctrl);
        assert_eq!(bus.read(), 0x4);

        // M2: high nibble of 0xD4 = 0xD
        rom.tick_bus(BusCycle::M2, &mut bus, &ctrl);
        assert_eq!(bus.read(), 0xD);
    }

    #[test]
    fn tick_bus_nonzero_address() {
        let mut rom = I4308::new(0);
        let mut data = vec![0u8; 1024];
        data[0x1A5] = 0xBE;
        rom.load(&data);

        let mut bus = DataBus::new();
        let ctrl = make_ctrl_rom(0);

        // Address = 0x1A5 = nibbles: A1=5, A2=A, A3=1
        bus.write(0x5);
        rom.tick_bus(BusCycle::A1, &mut bus, &ctrl);
        bus.write(0xA);
        rom.tick_bus(BusCycle::A2, &mut bus, &ctrl);
        bus.write(0x1);
        rom.tick_bus(BusCycle::A3, &mut bus, &ctrl);

        // M1: low nibble of 0xBE = 0xE
        rom.tick_bus(BusCycle::M1, &mut bus, &ctrl);
        assert_eq!(bus.read(), 0xE);

        // M2: high nibble of 0xBE = 0xB
        rom.tick_bus(BusCycle::M2, &mut bus, &ctrl);
        assert_eq!(bus.read(), 0xB);
    }

    // --- C.3.2: I/O port read/write ---

    #[test]
    fn io_port_write_read() {
        let mut rom = I4308::new(0);
        let mut bus = DataBus::new();
        let mut ctrl = make_ctrl_rom(0);

        // Select chip during A3
        bus.write(0);
        rom.tick_bus(BusCycle::A3, &mut bus, &ctrl);

        // SRC: select I/O port 2
        ctrl.set_io_op(IoOp::Src);
        bus.write(2);
        rom.tick_bus(BusCycle::X2, &mut bus, &ctrl);

        // WRR: write 0xA to port 2
        ctrl.set_io_op(IoOp::RomPortWrite);
        bus.write(0xA);
        rom.tick_bus(BusCycle::X2, &mut bus, &ctrl);

        // RDR: read port 2
        ctrl.set_io_op(IoOp::RomPortRead);
        rom.tick_bus(BusCycle::X3, &mut bus, &ctrl);
        assert_eq!(bus.read(), 0xA);
    }

    // --- C.3.3: chip_id matching ---

    #[test]
    fn unselected_rom_does_not_drive_bus() {
        let mut rom0 = I4308::new(0);
        let mut rom1 = I4308::new(1);
        rom0.load(&[0xAB]);
        rom1.load(&[0xCD]);

        let mut bus = DataBus::new();
        // Select ROM chip 0 only
        let ctrl = make_ctrl_rom(0);

        bus.write(0);
        rom0.tick_bus(BusCycle::A1, &mut bus, &ctrl);
        rom1.tick_bus(BusCycle::A1, &mut bus, &ctrl);
        rom0.tick_bus(BusCycle::A2, &mut bus, &ctrl);
        rom1.tick_bus(BusCycle::A2, &mut bus, &ctrl);
        rom0.tick_bus(BusCycle::A3, &mut bus, &ctrl);
        rom1.tick_bus(BusCycle::A3, &mut bus, &ctrl);

        assert!(rom0.is_selected());
        assert!(!rom1.is_selected());

        // M1: only rom0 should drive bus
        rom0.tick_bus(BusCycle::M1, &mut bus, &ctrl);
        assert_eq!(bus.read(), 0xB); // low nibble of 0xAB
    }

    // --- C.3.5: Multi-ROM 4K address space ---

    #[test]
    fn multi_rom_4k_space() {
        // 4x 4308 = 4K ROM space
        let mut roms: Vec<I4308> = (0..4).map(I4308::new).collect();

        // Load different data into each ROM
        for (i, rom) in roms.iter_mut().enumerate() {
            let data = vec![(i as u8) * 0x10 + 1; 1024];
            rom.load(&data);
        }

        let mut bus = DataBus::new();

        // Read from each ROM at address 0
        for (i, rom) in roms.iter_mut().enumerate() {
            let ctrl = make_ctrl_rom(i as u8);

            bus.write(0);
            rom.tick_bus(BusCycle::A1, &mut bus, &ctrl);
            rom.tick_bus(BusCycle::A2, &mut bus, &ctrl);
            rom.tick_bus(BusCycle::A3, &mut bus, &ctrl);

            assert!(rom.is_selected());

            rom.tick_bus(BusCycle::M1, &mut bus, &ctrl);
            let low = bus.read();
            rom.tick_bus(BusCycle::M2, &mut bus, &ctrl);
            let high = bus.read();

            let byte = low | (high << 4);
            assert_eq!(byte, (i as u8) * 0x10 + 1, "ROM {} data mismatch", i);
        }
    }

    #[test]
    fn port_read_write_accessors() {
        let mut rom = I4308::new(0);
        rom.write_port(0, 0xF);
        rom.write_port(1, 0x5);
        assert_eq!(rom.read_port(0), 0xF);
        assert_eq!(rom.read_port(1), 0x5);
        // Data masked to 4 bits
        rom.write_port(2, 0xFF);
        assert_eq!(rom.read_port(2), 0xF);
    }
}
