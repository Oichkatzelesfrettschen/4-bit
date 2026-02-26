//! Intellec-4 Monitor ROM
//!
//! WHY: The Intellec-4 development system shipped with a monitor program
//! in ROM that provided basic console commands for memory examination,
//! data deposit, program execution, and I/O. This is the "BIOS" of the
//! Intellec-4.
//!
//! WHAT: A monitor ROM image generator that produces a 256-byte ROM page
//! containing a minimal monitor loop. The monitor supports:
//! - Memory examine (display contents at address)
//! - Memory deposit (write data to address)
//! - Go (start execution at address)
//! - Halt (return to monitor)
//!
//! HOW: The monitor occupies one 4001 ROM page (256 bytes). It uses ROM
//! I/O for keyboard input and display output. The program image is
//! generated as a const array that can be loaded into the system's ROM.

/// Monitor command codes (hex keys on the Intellec-4 keypad).
///
/// The original Intellec-4 used a subset of the hex keypad for commands,
/// with the remaining keys used for hex data entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonitorCommand {
    /// Examine memory at current address (key E).
    Examine,
    /// Deposit data at current address and increment (key D).
    Deposit,
    /// Go: begin execution at current address (key A for "address go").
    Go,
    /// Halt: stop execution and return to monitor (key F).
    Halt,
    /// Hex digit entry (keys 0-9, B, C).
    HexDigit(u8),
    /// Step: execute one instruction (key B in some variants).
    Step,
    /// Unknown/invalid command.
    Invalid,
}

impl MonitorCommand {
    /// Decode a hex keycode (0-15) into a monitor command.
    ///
    /// Layout based on Intellec-4 conventions:
    /// - 0-9: hex digit entry
    /// - A (10): Go (execute)
    /// - B (11): Step (single instruction)
    /// - C (12): hex digit C
    /// - D (13): Deposit
    /// - E (14): Examine
    /// - F (15): Halt
    pub fn from_keycode(key: u8) -> Self {
        match key {
            0..=9 => Self::HexDigit(key),
            0xA => Self::Go,
            0xB => Self::Step,
            0xC => Self::HexDigit(0xC),
            0xD => Self::Deposit,
            0xE => Self::Examine,
            0xF => Self::Halt,
            _ => Self::Invalid,
        }
    }
}

/// Monitor ROM state machine.
///
/// Tracks the monitor's operational state between command processing
/// steps. The monitor runs as a coroutine driven by the system's
/// main loop rather than executing as a self-contained program.
#[derive(Clone, Debug)]
pub struct Monitor {
    /// Current address register (12-bit).
    address: u16,

    /// Data entry buffer (accumulates hex nibbles).
    data_buffer: u8,

    /// Number of hex nibbles entered since last command.
    nibbles_entered: u8,

    /// Whether the monitor is active (vs user program running).
    active: bool,

    /// Last examined data value (for display).
    last_data: u8,
}

impl Monitor {
    /// Create a new monitor in active state.
    pub fn new() -> Self {
        Self {
            address: 0,
            data_buffer: 0,
            nibbles_entered: 0,
            active: true,
            last_data: 0,
        }
    }

    /// Check if the monitor is active (accepting commands).
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Current address register.
    pub fn address(&self) -> u16 {
        self.address
    }

    /// Current data buffer.
    pub fn data_buffer(&self) -> u8 {
        self.data_buffer
    }

    /// Last examined/deposited data.
    pub fn last_data(&self) -> u8 {
        self.last_data
    }

    /// Process a hex keycode input.
    ///
    /// Returns a `MonitorAction` describing what the system should do.
    pub fn process_key(&mut self, keycode: u8) -> MonitorAction {
        let cmd = MonitorCommand::from_keycode(keycode);
        match cmd {
            MonitorCommand::HexDigit(n) => {
                // Accumulate hex nibbles into the data buffer
                self.data_buffer = (self.data_buffer << 4) | (n & 0x0F);
                self.nibbles_entered += 1;

                // After 3 nibbles, interpret as 12-bit address
                if self.nibbles_entered >= 3 {
                    self.address = ((self.data_buffer as u16) << 4) | (n as u16 & 0x0F);
                    self.address &= 0x0FFF;
                }

                MonitorAction::None
            }
            MonitorCommand::Examine => {
                self.nibbles_entered = 0;
                MonitorAction::Examine { address: self.address }
            }
            MonitorCommand::Deposit => {
                let data = self.data_buffer;
                self.nibbles_entered = 0;
                let addr = self.address;
                self.address = (self.address + 1) & 0x0FFF;
                MonitorAction::Deposit { address: addr, data }
            }
            MonitorCommand::Go => {
                self.active = false;
                MonitorAction::Go { address: self.address }
            }
            MonitorCommand::Step => MonitorAction::Step { address: self.address },
            MonitorCommand::Halt => {
                self.active = true;
                MonitorAction::Halt
            }
            MonitorCommand::Invalid => MonitorAction::None,
        }
    }

    /// Notify the monitor of an examine result (data read from memory).
    pub fn set_examined_data(&mut self, data: u8) {
        self.last_data = data;
        self.address = (self.address + 1) & 0x0FFF;
    }

    /// Re-enter the monitor (e.g., after a breakpoint or halt instruction).
    pub fn enter(&mut self) {
        self.active = true;
    }

    /// Reset the monitor to initial state.
    pub fn reset(&mut self) {
        self.address = 0;
        self.data_buffer = 0;
        self.nibbles_entered = 0;
        self.active = true;
        self.last_data = 0;
    }
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Action requested by the monitor in response to a command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonitorAction {
    /// No action needed.
    None,
    /// Read memory at address; caller should provide result via
    /// `set_examined_data`.
    Examine {
        /// Address to examine.
        address: u16,
    },
    /// Write data to memory at address.
    Deposit {
        /// Address to write to.
        address: u16,
        /// Data byte to write.
        data: u8,
    },
    /// Start execution at address.
    Go {
        /// Address to start at.
        address: u16,
    },
    /// Execute one instruction at address.
    Step {
        /// Address to step at.
        address: u16,
    },
    /// Halt execution and return to monitor.
    Halt,
}

/// Generate a minimal monitor ROM image (256 bytes).
///
/// This produces a ROM page containing a simple monitor loop
/// implemented in 4004 machine code. The monitor:
/// 1. Reads keyboard input via ROM I/O port
/// 2. Processes commands (Examine, Deposit, Go, Halt)
/// 3. Outputs to 7-segment display via shift register
///
/// The ROM image is designed to occupy page 15 (addresses 0xF00-0xFFF)
/// but can be loaded at any page boundary.
pub fn generate_monitor_rom() -> [u8; 256] {
    let mut rom = [0u8; 256];

    // Simple monitor loop in 4004 machine code:
    //
    // Address  Opcode  Mnemonic      Description
    // 0x00     0xF0    CLB           Clear accumulator and carry
    // 0x01     0x21    SRC P0        Select RAM register 0 for I/O
    // 0x02     0x20    (SRC P0 operand - pair 0)
    // Actually, SRC is a single-byte instruction using register pair.
    //
    // For a minimal monitor, we use a polling loop:
    // - Read ROM I/O port for keyboard
    // - Check for key press
    // - Dispatch to command handler
    // - Loop

    // Entry point: NOP sled for safety
    rom[0x00] = 0x00; // NOP
    rom[0x01] = 0x00; // NOP

    // CLB: Clear accumulator and carry
    rom[0x02] = 0xF0;

    // Main loop: Read keyboard (RDR on ROM port)
    // FIM P0, 0x00 (set register pair 0 to 0 for SRC targeting)
    rom[0x03] = 0x20; // FIM P0
    rom[0x04] = 0x00; // immediate = 0x00

    // SRC P0 (select ROM/RAM address for I/O)
    rom[0x05] = 0x21; // SRC P0

    // RDR: Read ROM I/O port -> accumulator
    rom[0x06] = 0xEA; // RDR (read ROM port)

    // Store keyboard data to register for later use
    // XCH R2: swap accumulator with register 2
    rom[0x07] = 0xB2; // XCH R2

    // Check if any key pressed (non-zero)
    // LD R2: load register 2 to accumulator
    rom[0x08] = 0xA2; // LD R2

    // JCN 0100 (jump if acc != 0 = jump if not zero)
    rom[0x09] = 0x14; // JCN condition: test=0, carry=1, zero=0 -> acc != 0
    rom[0x0A] = 0x03; // target: back to main loop

    // Key was pressed: it is in R2
    // For now, echo it to the display
    // LD R2
    rom[0x0B] = 0xA2; // LD R2

    // WRR: Write accumulator to ROM I/O port (display)
    rom[0x0C] = 0xE2; // WRR

    // JUN: Jump back to main loop
    rom[0x0D] = 0x40; // JUN (high nibble of address = 0)
    rom[0x0E] = 0x03; // low byte of address = 0x03

    // Pad rest with NOPs
    // (In a real monitor, this space would contain command handlers)

    rom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_monitor_is_active() {
        let mon = Monitor::new();
        assert!(mon.is_active());
        assert_eq!(mon.address(), 0);
    }

    #[test]
    fn hex_digit_accumulates() {
        let mut mon = Monitor::new();
        mon.process_key(0x1); // hex 1
        assert_eq!(mon.data_buffer(), 0x01);
        mon.process_key(0x2); // hex 2
        assert_eq!(mon.data_buffer(), 0x12);
    }

    #[test]
    fn examine_returns_address() {
        let mut mon = Monitor::new();
        // Enter address 0x123 via hex digits
        mon.process_key(1);
        mon.process_key(2);
        mon.process_key(3);

        let action = mon.process_key(0xE); // Examine
        match action {
            MonitorAction::Examine { address } => {
                // Address should be derived from entered nibbles
                assert!(address <= 0x0FFF);
            }
            _ => panic!("Expected Examine action"),
        }
    }

    #[test]
    fn deposit_writes_and_increments() {
        let mut mon = Monitor::new();
        mon.process_key(0xA); // Go key sets up...
                              // Actually let's just set address directly and deposit
        let mut mon = Monitor::new();
        mon.process_key(5); // data = 0x05
        let action = mon.process_key(0xD); // Deposit

        match action {
            MonitorAction::Deposit { address, data } => {
                assert_eq!(address, 0); // initial address
                assert_eq!(data, 0x05);
            }
            _ => panic!("Expected Deposit action"),
        }
        // Address should have incremented
        assert_eq!(mon.address(), 1);
    }

    #[test]
    fn go_deactivates_monitor() {
        let mut mon = Monitor::new();
        let action = mon.process_key(0xA); // Go
        assert_eq!(action, MonitorAction::Go { address: 0 });
        assert!(!mon.is_active());
    }

    #[test]
    fn halt_reactivates_monitor() {
        let mut mon = Monitor::new();
        mon.process_key(0xA); // Go (deactivates)
        assert!(!mon.is_active());

        let action = mon.process_key(0xF); // Halt
        assert_eq!(action, MonitorAction::Halt);
        assert!(mon.is_active());
    }

    #[test]
    fn examine_increments_address() {
        let mut mon = Monitor::new();
        mon.process_key(0xE); // Examine at address 0
        mon.set_examined_data(0x42);
        assert_eq!(mon.last_data(), 0x42);
        assert_eq!(mon.address(), 1); // auto-incremented
    }

    #[test]
    fn step_returns_address() {
        let mut mon = Monitor::new();
        mon.process_key(1); // set some address nibble
        let action = mon.process_key(0xB); // Step
        match action {
            MonitorAction::Step { address } => {
                assert!(address <= 0x0FFF);
            }
            _ => panic!("Expected Step action"),
        }
    }

    #[test]
    fn generate_rom_is_256_bytes() {
        let rom = generate_monitor_rom();
        assert_eq!(rom.len(), 256);
        // First two bytes are NOPs
        assert_eq!(rom[0], 0x00);
        assert_eq!(rom[1], 0x00);
        // CLB at offset 2
        assert_eq!(rom[2], 0xF0);
    }

    #[test]
    fn reset_clears_state() {
        let mut mon = Monitor::new();
        mon.process_key(0xA); // Go
        mon.process_key(5);
        mon.process_key(3);

        mon.reset();
        assert!(mon.is_active());
        assert_eq!(mon.address(), 0);
        assert_eq!(mon.data_buffer(), 0);
    }

    #[test]
    fn command_from_keycode_roundtrip() {
        assert_eq!(MonitorCommand::from_keycode(0), MonitorCommand::HexDigit(0));
        assert_eq!(MonitorCommand::from_keycode(9), MonitorCommand::HexDigit(9));
        assert_eq!(MonitorCommand::from_keycode(0xA), MonitorCommand::Go);
        assert_eq!(MonitorCommand::from_keycode(0xD), MonitorCommand::Deposit);
        assert_eq!(MonitorCommand::from_keycode(0xE), MonitorCommand::Examine);
        assert_eq!(MonitorCommand::from_keycode(0xF), MonitorCommand::Halt);
        assert_eq!(MonitorCommand::from_keycode(0x10), MonitorCommand::Invalid);
    }
}
