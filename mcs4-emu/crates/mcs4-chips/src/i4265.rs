//! Intel 4265 Programmable General Purpose I/O
//!
//! The 4265 provides 4 ports of 4 bits each (16 I/O lines total).
//! Each port can be independently configured as input or output via
//! direction control registers. It connects to the MCS-40 bus and
//! responds to I/O write/read commands.

use mcs4_bus::BusCycle;

/// Intel 4265: Programmable General Purpose I/O
///
/// 4 ports x 4 bits = 16 I/O lines with independent direction control.
#[derive(Clone, Debug)]
pub struct I4265 {
    /// Port data registers (4 ports, 4 bits each)
    ports: [u8; 4],

    /// Direction registers: bit=1 means output, bit=0 means input
    direction: [u8; 4],

    /// Input latches (external data driven into the chip)
    input_latches: [u8; 4],

    /// Chip ID for bus addressing
    pub chip_id: u8,
}

impl I4265 {
    pub fn new(chip_id: u8) -> Self {
        Self {
            ports: [0; 4],
            direction: [0; 4], // all inputs by default
            input_latches: [0; 4],
            chip_id,
        }
    }

    /// Set port direction (per-bit: 1=output, 0=input)
    pub fn set_direction(&mut self, port: usize, dir: u8) {
        if port < 4 {
            self.direction[port] = dir & 0x0F;
        }
    }

    /// Write data to a port (only output bits are driven)
    pub fn write_port(&mut self, port: usize, data: u8) {
        if port < 4 {
            self.ports[port] = data & 0x0F;
        }
    }

    /// Read a port (output bits return written value, input bits return latch)
    pub fn read_port(&self, port: usize) -> u8 {
        if port < 4 {
            let out_mask = self.direction[port];
            let in_mask = !out_mask & 0x0F;
            (self.ports[port] & out_mask) | (self.input_latches[port] & in_mask)
        } else {
            0x0F
        }
    }

    /// Set external input on a port (for input-configured pins)
    pub fn set_input(&mut self, port: usize, data: u8) {
        if port < 4 {
            self.input_latches[port] = data & 0x0F;
        }
    }

    /// Get the output-driven value for external devices
    pub fn output(&self, port: usize) -> u8 {
        if port < 4 {
            self.ports[port] & self.direction[port] & 0x0F
        } else {
            0
        }
    }

    /// Get direction register for a port
    pub fn get_direction(&self, port: usize) -> u8 {
        if port < 4 {
            self.direction[port]
        } else {
            0
        }
    }
}

impl Default for I4265 {
    fn default() -> Self {
        Self::new(0)
    }
}

impl super::Chip for I4265 {
    fn name(&self) -> &'static str {
        "4265"
    }

    fn reset(&mut self) {
        self.ports = [0; 4];
        self.direction = [0; 4];
        self.input_latches = [0; 4];
    }

    fn tick(&mut self, _phase: BusCycle) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_input_by_default() {
        let io = I4265::new(0);
        for port in 0..4 {
            assert_eq!(io.get_direction(port), 0x0);
        }
    }

    #[test]
    fn output_only_drives_output_pins() {
        let mut io = I4265::new(0);
        io.set_direction(0, 0x0F); // all output
        io.write_port(0, 0xA);
        assert_eq!(io.output(0), 0xA);

        io.set_direction(1, 0x05); // bits 0,2 output; bits 1,3 input
        io.write_port(1, 0xF);
        assert_eq!(io.output(1), 0x5); // only bits 0,2
    }

    #[test]
    fn read_mixes_output_and_input() {
        let mut io = I4265::new(0);
        io.set_direction(0, 0x03); // bits 0,1 output; bits 2,3 input
        io.write_port(0, 0x03); // drive 1s on output pins
        io.set_input(0, 0x0C); // drive 1s on input pins (bits 2,3)

        // Read should combine: out bits (0x03) | in bits (0x0C) = 0x0F
        assert_eq!(io.read_port(0), 0x0F);
    }

    #[test]
    fn input_pins_reflect_external_data() {
        let mut io = I4265::new(0);
        io.set_direction(0, 0x00); // all input
        io.set_input(0, 0x7);
        assert_eq!(io.read_port(0), 0x7);
    }

    #[test]
    fn ports_independent() {
        let mut io = I4265::new(0);
        for port in 0..4 {
            io.set_direction(port, 0x0F);
            io.write_port(port, port as u8 + 1);
        }
        for port in 0..4 {
            assert_eq!(io.read_port(port), port as u8 + 1);
        }
    }

    #[test]
    fn data_masked_to_4_bits() {
        let mut io = I4265::new(0);
        io.set_direction(0, 0xFF);
        assert_eq!(io.get_direction(0), 0x0F);

        io.write_port(0, 0xFF);
        assert_eq!(io.read_port(0), 0x0F);
    }

    #[test]
    fn reset_clears_all() {
        use crate::Chip;
        let mut io = I4265::new(0);
        io.set_direction(0, 0x0F);
        io.write_port(0, 0xA);

        io.reset();

        assert_eq!(io.get_direction(0), 0);
        assert_eq!(io.read_port(0), 0);
    }

    #[test]
    fn out_of_range_port() {
        let io = I4265::new(0);
        assert_eq!(io.read_port(4), 0x0F);
        assert_eq!(io.output(4), 0);
    }

    #[test]
    fn chip_trait_name() {
        use crate::Chip;
        let io = I4265::new(0);
        assert_eq!(io.name(), "4265");
    }
}
