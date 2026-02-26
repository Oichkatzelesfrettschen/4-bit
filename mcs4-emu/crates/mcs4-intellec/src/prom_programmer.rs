//! Intellec-4 PROM Programmer Interface
//!
//! WHY: The Intellec-4 included a PROM programmer socket for programming
//! 4702 UV-erasable PROMs. Users loaded code into RAM, then transferred
//! it to the PROM chip in the programmer socket.
//!
//! WHAT: A PROM programmer controller that manages the programming
//! workflow: address selection, data loading, programming voltage timing,
//! verify pass, and blank check.
//!
//! HOW: The programmer drives the 4702's programming interface: it sets
//! addresses, applies programming voltage (Vpp), and verifies written
//! data. The programming pulse width is configurable to match the 4702
//! datasheet (typically 50ms per byte).

use mcs4_chips::i4702::I4702;

/// Programming pulse configuration.
#[derive(Clone, Copy, Debug)]
pub struct ProgramConfig {
    /// Programming pulse width in machine cycles.
    /// Real hardware: ~50ms. At 740kHz: ~37,000 cycles.
    pub pulse_cycles: u32,

    /// Number of retries on verify failure before giving up.
    pub max_retries: u8,
}

impl Default for ProgramConfig {
    fn default() -> Self {
        Self {
            pulse_cycles: 37_000, // ~50ms at 740kHz
            max_retries: 3,
        }
    }
}

/// Result of a programming or verify operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgResult {
    /// Operation completed successfully.
    Ok,
    /// Programming failed at the given address.
    ProgramFail {
        /// Address that failed.
        address: u8,
    },
    /// Verify failed: expected vs actual mismatch.
    VerifyFail {
        /// Address of mismatch.
        address: u8,
        /// Expected data.
        expected: u8,
        /// Actual data read.
        actual: u8,
    },
    /// PROM is not blank at the given address.
    NotBlank {
        /// Address that is not erased.
        address: u8,
    },
}

/// PROM programmer state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProgState {
    Idle,
    Programming,
    Verifying,
}

/// Intellec-4 PROM Programmer.
///
/// Controls the programming workflow for 4702 EPROMs:
/// 1. Blank check (verify all 0xFF)
/// 2. Program (write data with Vpp)
/// 3. Verify (read back and compare)
#[derive(Clone, Debug)]
pub struct PromProgrammer {
    /// Programming configuration.
    config: ProgramConfig,

    /// Internal state.
    state: ProgState,

    /// Source data buffer (up to 256 bytes to program).
    buffer: Vec<u8>,

    /// Current programming address.
    current_address: u8,

    /// Number of bytes successfully programmed in current session.
    bytes_programmed: usize,
}

impl PromProgrammer {
    /// Create a new PROM programmer with the given configuration.
    pub fn new(config: ProgramConfig) -> Self {
        Self {
            config,
            state: ProgState::Idle,
            buffer: Vec::new(),
            current_address: 0,
            bytes_programmed: 0,
        }
    }

    /// Load data into the programmer's buffer for programming.
    pub fn load_buffer(&mut self, data: &[u8]) {
        self.buffer = data.to_vec();
        self.buffer.truncate(256);
    }

    /// Get the programmer's data buffer.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Number of bytes programmed in the last operation.
    pub fn bytes_programmed(&self) -> usize {
        self.bytes_programmed
    }

    /// Whether the programmer is idle (ready for a new operation).
    pub fn is_idle(&self) -> bool {
        self.state == ProgState::Idle
    }

    /// Perform a blank check on the PROM.
    ///
    /// Verifies that all bytes in the specified range are 0xFF (erased).
    pub fn blank_check(&self, prom: &I4702, start: u8, count: u8) -> ProgResult {
        for i in 0..count {
            let addr = start.wrapping_add(i);
            if !prom.is_erased(addr) {
                return ProgResult::NotBlank { address: addr };
            }
        }
        ProgResult::Ok
    }

    /// Program the buffer contents into the PROM.
    ///
    /// Programs `buffer.len()` bytes starting at `start_address`.
    /// The PROM must be in programming mode (Vpp applied) before calling.
    /// After programming, the PROM is taken out of programming mode.
    pub fn program(&mut self, prom: &mut I4702, start_address: u8) -> ProgResult {
        self.state = ProgState::Programming;
        self.bytes_programmed = 0;

        // Enable programming voltage
        prom.set_programming_mode(true);

        for (i, &byte) in self.buffer.iter().enumerate() {
            let addr = start_address.wrapping_add(i as u8);
            prom.set_program_address(addr);

            let mut success = false;
            for _retry in 0..=self.config.max_retries {
                prom.program_byte(byte);

                // Verify this byte
                let readback = prom.read_rom(addr);
                if readback == byte {
                    success = true;
                    break;
                }
            }

            if !success {
                prom.set_programming_mode(false);
                self.state = ProgState::Idle;
                return ProgResult::ProgramFail { address: addr };
            }

            self.bytes_programmed += 1;
        }

        prom.set_programming_mode(false);
        self.state = ProgState::Idle;
        ProgResult::Ok
    }

    /// Verify PROM contents against the buffer.
    pub fn verify(&mut self, prom: &I4702, start_address: u8) -> ProgResult {
        self.state = ProgState::Verifying;

        for (i, &expected) in self.buffer.iter().enumerate() {
            let addr = start_address.wrapping_add(i as u8);
            let actual = prom.read_rom(addr);
            if actual != expected {
                self.state = ProgState::Idle;
                return ProgResult::VerifyFail {
                    address: addr,
                    expected,
                    actual,
                };
            }
        }

        self.state = ProgState::Idle;
        ProgResult::Ok
    }

    /// Program and verify in one operation.
    pub fn program_and_verify(
        &mut self,
        prom: &mut I4702,
        start_address: u8,
    ) -> ProgResult {
        let result = self.program(prom, start_address);
        if result != ProgResult::Ok {
            return result;
        }
        self.verify(prom, start_address)
    }

    /// Reset the programmer state.
    pub fn reset(&mut self) {
        self.state = ProgState::Idle;
        self.current_address = 0;
        self.bytes_programmed = 0;
    }
}

impl Default for PromProgrammer {
    fn default() -> Self {
        Self::new(ProgramConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_check_on_new_prom() {
        let prom = I4702::new(0);
        let prog = PromProgrammer::default();
        assert_eq!(prog.blank_check(&prom, 0, 255), ProgResult::Ok);
    }

    #[test]
    fn blank_check_fails_on_programmed_prom() {
        let mut prom = I4702::new(0);
        prom.load(&[0x42]);

        let prog = PromProgrammer::default();
        assert_eq!(
            prog.blank_check(&prom, 0, 1),
            ProgResult::NotBlank { address: 0 }
        );
    }

    #[test]
    fn program_single_byte() {
        let mut prom = I4702::new(0);
        let mut prog = PromProgrammer::default();
        prog.load_buffer(&[0xA5]);

        let result = prog.program(&mut prom, 0);
        assert_eq!(result, ProgResult::Ok);
        assert_eq!(prom.read_rom(0), 0xA5);
        assert_eq!(prog.bytes_programmed(), 1);
    }

    #[test]
    fn program_multiple_bytes() {
        let mut prom = I4702::new(0);
        let mut prog = PromProgrammer::default();

        let data = vec![0x10, 0x20, 0x30, 0x40];
        prog.load_buffer(&data);

        let result = prog.program(&mut prom, 0);
        assert_eq!(result, ProgResult::Ok);
        assert_eq!(prog.bytes_programmed(), 4);

        for (i, &expected) in data.iter().enumerate() {
            assert_eq!(prom.read_rom(i as u8), expected);
        }
    }

    #[test]
    fn verify_succeeds_after_program() {
        let mut prom = I4702::new(0);
        let mut prog = PromProgrammer::default();

        prog.load_buffer(&[0xDE, 0xAD]);
        prog.program(&mut prom, 0);

        let result = prog.verify(&prom, 0);
        assert_eq!(result, ProgResult::Ok);
    }

    #[test]
    fn verify_detects_mismatch() {
        let mut prom = I4702::new(0);
        let mut prog = PromProgrammer::default();

        prom.load(&[0x42]);
        prog.load_buffer(&[0x99]);

        let result = prog.verify(&prom, 0);
        assert_eq!(
            result,
            ProgResult::VerifyFail {
                address: 0,
                expected: 0x99,
                actual: 0x42,
            }
        );
    }

    #[test]
    fn program_and_verify_roundtrip() {
        let mut prom = I4702::new(0);
        let mut prog = PromProgrammer::default();

        let data: Vec<u8> = (0..128).collect();
        prog.load_buffer(&data);

        let result = prog.program_and_verify(&mut prom, 0);
        assert_eq!(result, ProgResult::Ok);
    }

    #[test]
    fn buffer_truncated_to_256() {
        let mut prog = PromProgrammer::default();
        let data = vec![0u8; 300];
        prog.load_buffer(&data);
        assert_eq!(prog.buffer().len(), 256);
    }

    #[test]
    fn programmer_starts_idle() {
        let prog = PromProgrammer::default();
        assert!(prog.is_idle());
        assert_eq!(prog.bytes_programmed(), 0);
    }

    #[test]
    fn program_at_offset() {
        let mut prom = I4702::new(0);
        let mut prog = PromProgrammer::default();

        prog.load_buffer(&[0xBE, 0xEF]);
        let result = prog.program(&mut prom, 0x10);
        assert_eq!(result, ProgResult::Ok);

        // First 0x10 bytes should still be 0xFF (erased)
        assert_eq!(prom.read_rom(0x0F), 0xFF);
        assert_eq!(prom.read_rom(0x10), 0xBE);
        assert_eq!(prom.read_rom(0x11), 0xEF);
    }
}
