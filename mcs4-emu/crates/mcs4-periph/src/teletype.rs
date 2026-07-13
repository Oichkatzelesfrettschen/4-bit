//! Source-bound ASR-33 terminal behavior.
//!
//! The MCS-40 manual describes three teletype paths: terminal receive,
//! terminal transmit, and reader control. This module models those paths as
//! serial wires. It deliberately does not provide a CPU-visible byte FIFO.

use std::collections::VecDeque;

/// Timing descriptor for an asynchronous serial wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TeletypeTiming {
    /// Fundamental phase ticks per second.
    pub ticks_per_second: u64,
    /// Serial symbols per second.
    pub baud: u32,
    /// Data bits in each character.
    pub data_bits: u8,
    /// Stop bits in each character.
    pub stop_bits: u8,
}

impl TeletypeTiming {
    /// Create the documented ASR-33 8N2 serial descriptor.
    pub const fn asr33(ticks_per_second: u64) -> Self {
        Self {
            ticks_per_second,
            baud: 110,
            data_bits: 8,
            stop_bits: 2,
        }
    }

    /// Return the number of symbols in one framed character.
    pub const fn symbols_per_character(self) -> u8 {
        1 + self.data_bits + self.stop_bits
    }
}

/// Three source-supported wire values between an Intellec and a terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TeletypeSignals {
    /// Terminal serial output to the machine receive circuit.
    pub terminal_to_machine: bool,
    /// Machine serial output to the terminal printer receive circuit.
    pub machine_to_terminal: bool,
    /// Reader-run line from the machine to the terminal relay circuit.
    pub reader_enabled: bool,
}

#[derive(Clone, Debug)]
struct SerialTransmitter {
    timing: TeletypeTiming,
    queued: VecDeque<u8>,
    active: Option<u8>,
    symbol_index: u8,
    elapsed_scaled: u64,
    line: bool,
}

impl SerialTransmitter {
    fn new(timing: TeletypeTiming) -> Self {
        Self {
            timing,
            queued: VecDeque::new(),
            active: None,
            symbol_index: 0,
            elapsed_scaled: 0,
            line: true,
        }
    }

    fn enqueue(&mut self, byte: u8) {
        self.queued.push_back(byte);
    }

    fn is_idle(&self) -> bool {
        self.active.is_none() && self.queued.is_empty()
    }

    fn line(&self) -> bool {
        self.line
    }

    fn reset(&mut self) {
        self.queued.clear();
        self.active = None;
        self.symbol_index = 0;
        self.elapsed_scaled = 0;
        self.line = true;
    }

    fn begin_next_byte(&mut self) {
        if self.active.is_none() {
            if let Some(byte) = self.queued.pop_front() {
                self.active = Some(byte);
                self.symbol_index = 0;
                self.elapsed_scaled = 0;
                self.line = false;
            }
        }
    }

    fn update_line(&mut self) {
        let Some(byte) = self.active else {
            self.line = true;
            return;
        };

        self.line = match self.symbol_index {
            0 => false,
            index if index <= self.timing.data_bits => ((byte >> (index - 1)) & 1) != 0,
            _ => true,
        };
    }

    fn advance_phase_ticks(&mut self, ticks: u64) {
        self.begin_next_byte();
        if self.active.is_none() {
            return;
        }

        self.elapsed_scaled = self
            .elapsed_scaled
            .saturating_add(ticks.saturating_mul(u64::from(self.timing.baud)));

        while self.elapsed_scaled >= self.timing.ticks_per_second && self.active.is_some() {
            self.elapsed_scaled -= self.timing.ticks_per_second;
            self.symbol_index += 1;
            if self.symbol_index >= self.timing.symbols_per_character() {
                self.active = None;
                self.begin_next_byte();
            }
            self.update_line();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReceiveState {
    Idle,
    Start,
    Data(u8),
    Stop(u8),
}

#[derive(Clone, Debug)]
struct SerialReceiver {
    timing: TeletypeTiming,
    state: ReceiveState,
    elapsed_scaled: u64,
    byte: u8,
    received: VecDeque<u8>,
}

impl SerialReceiver {
    fn new(timing: TeletypeTiming) -> Self {
        Self {
            timing,
            state: ReceiveState::Idle,
            elapsed_scaled: 0,
            byte: 0,
            received: VecDeque::new(),
        }
    }

    fn take_received(&mut self) -> Option<u8> {
        self.received.pop_front()
    }

    fn reset(&mut self) {
        self.state = ReceiveState::Idle;
        self.elapsed_scaled = 0;
        self.byte = 0;
        self.received.clear();
    }

    fn advance_phase_ticks(&mut self, ticks: u64, line: bool) {
        if self.state == ReceiveState::Idle {
            if !line {
                self.state = ReceiveState::Start;
                self.elapsed_scaled = 0;
                self.byte = 0;
            }
            return;
        }

        self.elapsed_scaled = self
            .elapsed_scaled
            .saturating_add(ticks.saturating_mul(u64::from(self.timing.baud) * 2));

        match self.state {
            ReceiveState::Idle => {}
            ReceiveState::Start => {
                if self.elapsed_scaled >= self.timing.ticks_per_second {
                    self.elapsed_scaled = 0;
                    self.state = if line {
                        ReceiveState::Idle
                    } else {
                        ReceiveState::Data(0)
                    };
                }
            }
            ReceiveState::Data(bit_index) => {
                let full_symbol = self.timing.ticks_per_second.saturating_mul(2);
                if self.elapsed_scaled >= full_symbol {
                    self.elapsed_scaled -= full_symbol;
                    if line {
                        self.byte |= 1 << bit_index;
                    }
                    self.state = if bit_index + 1 == self.timing.data_bits {
                        ReceiveState::Stop(0)
                    } else {
                        ReceiveState::Data(bit_index + 1)
                    };
                }
            }
            ReceiveState::Stop(stop_index) => {
                let full_symbol = self.timing.ticks_per_second.saturating_mul(2);
                if self.elapsed_scaled >= full_symbol {
                    self.elapsed_scaled -= full_symbol;
                    if !line {
                        self.state = ReceiveState::Idle;
                    } else if stop_index + 1 == self.timing.stop_bits {
                        self.received.push_back(self.byte);
                        self.state = ReceiveState::Idle;
                    } else {
                        self.state = ReceiveState::Stop(stop_index + 1);
                    }
                }
            }
        }
    }
}

/// ASR-33 keyboard, printer, paper reader, and paper punch state.
#[derive(Clone, Debug)]
pub struct Teletype33 {
    timing: TeletypeTiming,
    terminal_transmitter: SerialTransmitter,
    terminal_receiver: SerialReceiver,
    keyboard: VecDeque<u8>,
    reader: VecDeque<u8>,
    printed: VecDeque<u8>,
    punched: VecDeque<u8>,
    reader_enabled: bool,
    punch_enabled: bool,
}

impl Teletype33 {
    /// Construct an ASR-33 terminal with a source-declared phase clock.
    pub fn new(timing: TeletypeTiming) -> Self {
        assert!(timing.ticks_per_second > 0, "teletype clock must be nonzero");
        assert!(timing.baud > 0, "teletype baud must be nonzero");
        assert!(timing.data_bits == 8, "ASR-33 requires eight data bits");
        assert!(timing.stop_bits == 2, "ASR-33 requires two stop bits");
        Self {
            timing,
            terminal_transmitter: SerialTransmitter::new(timing),
            terminal_receiver: SerialReceiver::new(timing),
            keyboard: VecDeque::new(),
            reader: VecDeque::new(),
            printed: VecDeque::new(),
            punched: VecDeque::new(),
            reader_enabled: false,
            punch_enabled: false,
        }
    }

    /// Return the terminal timing descriptor.
    pub const fn timing(&self) -> TeletypeTiming {
        self.timing
    }

    /// Queue one keyboard character for serial transmission to the machine.
    pub fn enqueue_keyboard(&mut self, byte: u8) {
        self.keyboard.push_back(byte);
    }

    /// Replace the paper-reader tape with raw serial characters.
    pub fn load_reader_tape(&mut self, bytes: impl IntoIterator<Item = u8>) {
        self.reader.clear();
        self.reader.extend(bytes);
    }

    /// Enable or disable the terminal paper reader relay.
    pub fn set_reader_enabled(&mut self, enabled: bool) {
        self.reader_enabled = enabled;
    }

    /// Enable or disable terminal-side punch capture.
    pub fn set_punch_enabled(&mut self, enabled: bool) {
        self.punch_enabled = enabled;
    }

    /// Drain characters printed by the machine-to-terminal wire.
    pub fn drain_printed(&mut self) -> Vec<u8> {
        self.printed.drain(..).collect()
    }

    /// Drain terminal-side paper-punch output.
    pub fn drain_punched(&mut self) -> Vec<u8> {
        self.punched.drain(..).collect()
    }

    /// Advance terminal state by phase ticks and sample the machine transmit wire.
    pub fn advance_phase_ticks(&mut self, ticks: u64, machine_to_terminal: bool) -> TeletypeSignals {
        self.terminal_receiver.advance_phase_ticks(ticks, machine_to_terminal);
        while let Some(byte) = self.terminal_receiver.take_received() {
            self.printed.push_back(byte);
            if self.punch_enabled {
                self.punched.push_back(byte);
            }
        }

        if self.terminal_transmitter.is_idle() {
            if let Some(byte) = self.keyboard.pop_front() {
                self.terminal_transmitter.enqueue(byte);
            } else if self.reader_enabled {
                if let Some(byte) = self.reader.pop_front() {
                    self.terminal_transmitter.enqueue(byte);
                }
            }
        }
        self.terminal_transmitter.advance_phase_ticks(ticks);

        TeletypeSignals {
            terminal_to_machine: self.terminal_transmitter.line(),
            machine_to_terminal,
            reader_enabled: self.reader_enabled,
        }
    }

    /// Reset terminal state without changing the timing descriptor.
    pub fn reset(&mut self) {
        self.terminal_transmitter.reset();
        self.terminal_receiver.reset();
        self.keyboard.clear();
        self.reader.clear();
        self.printed.clear();
        self.punched.clear();
        self.reader_enabled = false;
        self.punch_enabled = false;
    }
}

#[cfg(test)]
mod tests {
    use super::{SerialReceiver, SerialTransmitter, Teletype33, TeletypeTiming};

    const TEST_TIMING: TeletypeTiming = TeletypeTiming::asr33(1_100);

    fn transmit_byte(transmitter: &mut SerialTransmitter, receiver: &mut SerialReceiver, byte: u8) {
        transmitter.enqueue(byte);
        for _ in 0..(usize::from(TEST_TIMING.symbols_per_character()) * 12) {
            transmitter.advance_phase_ticks(1);
            receiver.advance_phase_ticks(1, transmitter.line());
        }
    }

    #[test]
    fn asr33_descriptor_has_eleven_symbols() {
        let timing = TeletypeTiming::asr33(750_000);
        assert_eq!(timing.baud, 110);
        assert_eq!(timing.symbols_per_character(), 11);
    }

    #[test]
    fn serial_wire_preserves_eight_data_bits_and_two_stop_bits() {
        let mut transmitter = SerialTransmitter::new(TEST_TIMING);
        let mut receiver = SerialReceiver::new(TEST_TIMING);
        transmit_byte(&mut transmitter, &mut receiver, 0xA5);
        assert_eq!(receiver.take_received(), Some(0xA5));
    }

    #[test]
    fn keyboard_reaches_machine_wire() {
        let mut terminal = Teletype33::new(TEST_TIMING);
        let mut receiver = SerialReceiver::new(TEST_TIMING);
        terminal.enqueue_keyboard(b'A');
        for _ in 0..200 {
            let signals = terminal.advance_phase_ticks(1, true);
            receiver.advance_phase_ticks(1, signals.terminal_to_machine);
        }
        assert_eq!(receiver.take_received(), Some(b'A'));
    }

    #[test]
    fn reader_runs_only_when_enabled() {
        let mut terminal = Teletype33::new(TEST_TIMING);
        let mut receiver = SerialReceiver::new(TEST_TIMING);
        terminal.load_reader_tape([b'R']);
        for _ in 0..200 {
            let signals = terminal.advance_phase_ticks(1, true);
            receiver.advance_phase_ticks(1, signals.terminal_to_machine);
        }
        assert_eq!(receiver.take_received(), None);

        terminal.set_reader_enabled(true);
        for _ in 0..200 {
            let signals = terminal.advance_phase_ticks(1, true);
            receiver.advance_phase_ticks(1, signals.terminal_to_machine);
        }
        assert_eq!(receiver.take_received(), Some(b'R'));
    }

    #[test]
    fn machine_output_reaches_printer_and_punch() {
        let mut terminal = Teletype33::new(TEST_TIMING);
        let mut transmitter = SerialTransmitter::new(TEST_TIMING);
        terminal.set_punch_enabled(true);
        transmitter.enqueue(b'P');
        for _ in 0..200 {
            transmitter.advance_phase_ticks(1);
            terminal.advance_phase_ticks(1, transmitter.line());
        }
        assert_eq!(terminal.drain_printed(), vec![b'P']);
        assert_eq!(terminal.drain_punched(), vec![b'P']);
    }
}
