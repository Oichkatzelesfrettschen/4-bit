//! UART / Serial Interface
//!
//! WHY: The Intellec-4 communicated with ASR-33 teletypes via serial
//! I/O. MCS-4 systems used bit-bang serial via ROM I/O ports since
//! there was no hardware UART in the chipset.
//!
//! WHAT: A software UART model that emulates asynchronous serial
//! communication with configurable baud rate, data bits, and stop bits.
//! Provides TX and RX FIFOs for buffered operation.
//!
//! HOW: The CPU drives TX by toggling a ROM output port bit via WRR.
//! RX is read from a ROM input port bit via RDR. This module handles
//! the bit timing, start/stop bit framing, and data buffering. The
//! caller is responsible for clocking the UART at the correct rate
//! (typically derived from machine cycle counting).

use std::collections::VecDeque;

/// Serial line configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SerialConfig {
    /// Baud rate (bits per second). ASR-33 uses 110 baud.
    pub baud_rate: u32,

    /// Number of data bits per character (5-8). ASR-33 uses 8.
    pub data_bits: u8,

    /// Number of stop bits (1 or 2). ASR-33 uses 2.
    pub stop_bits: u8,

    /// Machine cycles per bit period.
    /// Derived from baud rate and CPU clock. For a 740kHz 4004:
    /// cycles_per_bit = 740000 / (baud_rate * 8) [8 phases per cycle]
    /// At 110 baud: ~840 cycles per bit.
    pub cycles_per_bit: u32,
}

impl SerialConfig {
    /// ASR-33 teletype: 110 baud, 8 data bits, 2 stop bits.
    pub fn asr33(cpu_clock_hz: u32) -> Self {
        let baud = 110;
        Self {
            baud_rate: baud,
            data_bits: 8,
            stop_bits: 2,
            cycles_per_bit: cpu_clock_hz / baud,
        }
    }

    /// Standard 9600 baud configuration: 8N1.
    pub fn baud_9600(cpu_clock_hz: u32) -> Self {
        let baud = 9600;
        Self {
            baud_rate: baud,
            data_bits: 8,
            stop_bits: 1,
            cycles_per_bit: cpu_clock_hz / baud,
        }
    }
}

/// Transmitter state machine states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TxState {
    Idle,
    StartBit,
    DataBit(u8), // which bit (0 = LSB)
    StopBit(u8), // which stop bit (0-based)
}

/// Receiver state machine states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RxState {
    Idle,
    StartBit,
    DataBit(u8),
    StopBit(u8),
}

/// Software UART for MCS-4 bit-bang serial I/O.
#[derive(Clone, Debug)]
pub struct Uart {
    config: SerialConfig,

    // --- Transmitter ---
    tx_state: TxState,
    tx_shift: u8,
    tx_counter: u32,
    tx_line: bool, // current TX line state (true = mark/idle/1)
    tx_fifo: VecDeque<u8>,

    // --- Receiver ---
    rx_state: RxState,
    rx_shift: u8,
    rx_counter: u32,
    rx_fifo: VecDeque<u8>,

    /// FIFO capacity (same for TX and RX).
    fifo_capacity: usize,
}

impl Uart {
    /// Create a new UART with the given configuration and FIFO depth.
    pub fn new(config: SerialConfig, fifo_depth: usize) -> Self {
        Self {
            config,
            tx_state: TxState::Idle,
            tx_shift: 0,
            tx_counter: 0,
            tx_line: true, // idle = mark = high
            tx_fifo: VecDeque::with_capacity(fifo_depth),
            rx_state: RxState::Idle,
            rx_shift: 0,
            rx_counter: 0,
            rx_fifo: VecDeque::with_capacity(fifo_depth),
            fifo_capacity: fifo_depth,
        }
    }

    /// Queue a byte for transmission.
    ///
    /// Returns false if TX FIFO is full.
    pub fn tx_write(&mut self, byte: u8) -> bool {
        if self.tx_fifo.len() >= self.fifo_capacity {
            return false;
        }
        self.tx_fifo.push_back(byte);
        true
    }

    /// Read a received byte from the RX FIFO.
    pub fn rx_read(&mut self) -> Option<u8> {
        self.rx_fifo.pop_front()
    }

    /// Check if TX FIFO is empty (transmitter idle or finishing).
    pub fn tx_empty(&self) -> bool {
        self.tx_fifo.is_empty() && self.tx_state == TxState::Idle
    }

    /// Check if RX FIFO has data available.
    pub fn rx_available(&self) -> bool {
        !self.rx_fifo.is_empty()
    }

    /// Number of bytes in TX FIFO.
    pub fn tx_pending(&self) -> usize {
        self.tx_fifo.len()
    }

    /// Number of bytes in RX FIFO.
    pub fn rx_pending(&self) -> usize {
        self.rx_fifo.len()
    }

    /// Current TX line state (for ROM output port bit).
    ///
    /// true = mark (idle/1), false = space (0).
    pub fn tx_line(&self) -> bool {
        self.tx_line
    }

    /// Feed the RX line state (from ROM input port bit).
    ///
    /// Call this once per machine cycle tick. The receiver samples
    /// at the configured bit rate.
    pub fn rx_feed(&mut self, line_high: bool) {
        self.rx_counter += 1;

        match self.rx_state {
            RxState::Idle => {
                if !line_high {
                    // Falling edge: potential start bit
                    self.rx_state = RxState::StartBit;
                    // Sample at mid-bit: wait half a bit period
                    self.rx_counter = 0;
                }
            }
            RxState::StartBit => {
                if self.rx_counter >= self.config.cycles_per_bit / 2 {
                    // Mid-bit sample of start bit
                    if !line_high {
                        // Valid start bit; begin receiving data
                        self.rx_state = RxState::DataBit(0);
                        self.rx_shift = 0;
                        self.rx_counter = 0;
                    } else {
                        // False start (noise); go back to idle
                        self.rx_state = RxState::Idle;
                    }
                }
            }
            RxState::DataBit(bit_idx) => {
                if self.rx_counter >= self.config.cycles_per_bit {
                    self.rx_counter = 0;
                    // Sample data bit (LSB first)
                    if line_high {
                        self.rx_shift |= 1 << bit_idx;
                    }
                    if bit_idx + 1 >= self.config.data_bits {
                        // All data bits received; move to stop bit
                        self.rx_state = RxState::StopBit(0);
                    } else {
                        self.rx_state = RxState::DataBit(bit_idx + 1);
                    }
                }
            }
            RxState::StopBit(stop_idx) => {
                if self.rx_counter >= self.config.cycles_per_bit {
                    self.rx_counter = 0;
                    if stop_idx + 1 >= self.config.stop_bits {
                        // All stop bits received; byte complete
                        if self.rx_fifo.len() < self.fifo_capacity {
                            self.rx_fifo.push_back(self.rx_shift);
                        }
                        // Overrun: drop byte silently if FIFO full
                        self.rx_state = RxState::Idle;
                    } else {
                        self.rx_state = RxState::StopBit(stop_idx + 1);
                    }
                }
            }
        }
    }

    /// Advance the transmitter by one machine cycle.
    ///
    /// Call this once per cycle tick. The TX line state is updated
    /// according to the configured bit rate.
    pub fn tx_tick(&mut self) {
        match self.tx_state {
            TxState::Idle => {
                if let Some(byte) = self.tx_fifo.pop_front() {
                    // Begin transmitting: send start bit (space = low)
                    self.tx_shift = byte;
                    self.tx_state = TxState::StartBit;
                    self.tx_line = false;
                    self.tx_counter = 0;
                }
                // else: remain idle (line high)
            }
            TxState::StartBit => {
                self.tx_counter += 1;
                if self.tx_counter >= self.config.cycles_per_bit {
                    self.tx_counter = 0;
                    // Start bit done; send first data bit (LSB)
                    self.tx_state = TxState::DataBit(0);
                    self.tx_line = self.tx_shift & 1 != 0;
                }
            }
            TxState::DataBit(bit_idx) => {
                self.tx_counter += 1;
                if self.tx_counter >= self.config.cycles_per_bit {
                    self.tx_counter = 0;
                    if bit_idx + 1 >= self.config.data_bits {
                        // All data bits sent; send stop bit
                        self.tx_state = TxState::StopBit(0);
                        self.tx_line = true; // stop bit = mark = high
                    } else {
                        let next = bit_idx + 1;
                        self.tx_state = TxState::DataBit(next);
                        self.tx_line = (self.tx_shift >> next) & 1 != 0;
                    }
                }
            }
            TxState::StopBit(stop_idx) => {
                self.tx_counter += 1;
                if self.tx_counter >= self.config.cycles_per_bit {
                    self.tx_counter = 0;
                    if stop_idx + 1 >= self.config.stop_bits {
                        // Transmission complete
                        self.tx_state = TxState::Idle;
                        self.tx_line = true; // back to idle
                    } else {
                        self.tx_state = TxState::StopBit(stop_idx + 1);
                    }
                }
            }
        }
    }

    /// Reset the UART to idle state.
    pub fn reset(&mut self) {
        self.tx_state = TxState::Idle;
        self.tx_line = true;
        self.tx_counter = 0;
        self.tx_shift = 0;
        self.tx_fifo.clear();
        self.rx_state = RxState::Idle;
        self.rx_counter = 0;
        self.rx_shift = 0;
        self.rx_fifo.clear();
    }

    /// Get the serial configuration.
    pub fn config(&self) -> &SerialConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_config() -> SerialConfig {
        SerialConfig {
            baud_rate: 9600,
            data_bits: 8,
            stop_bits: 1,
            cycles_per_bit: 10, // short for testing
        }
    }

    #[test]
    fn new_uart_is_idle() {
        let uart = Uart::new(simple_config(), 16);
        assert!(uart.tx_empty());
        assert!(!uart.rx_available());
        assert!(uart.tx_line()); // idle = high
    }

    #[test]
    fn tx_produces_start_bit() {
        let mut uart = Uart::new(simple_config(), 16);
        uart.tx_write(0x55);

        // First tick loads the byte and sets start bit
        uart.tx_tick();
        assert!(!uart.tx_line()); // start bit = low
    }

    #[test]
    fn tx_full_byte_loopback() {
        let cfg = simple_config();
        let cpb = cfg.cycles_per_bit;
        let mut uart = Uart::new(cfg, 16);

        let test_byte: u8 = 0xA5;
        uart.tx_write(test_byte);

        // Tick through: 1 start + 8 data + 1 stop = 10 bit periods
        // First tick starts transmission
        uart.tx_tick();

        // Collect TX line state at mid-bit points
        let mut received: u8 = 0;

        // Start bit period
        for _ in 1..cpb {
            uart.tx_tick();
        }

        // 8 data bit periods
        for bit in 0..8u8 {
            uart.tx_tick(); // transition to data bit
                            // Sample at mid-bit
            for _ in 1..cpb / 2 {
                uart.tx_tick();
            }
            if uart.tx_line() {
                received |= 1 << bit;
            }
            for _ in cpb / 2..cpb {
                uart.tx_tick();
            }
        }

        assert_eq!(received, test_byte);
    }

    #[test]
    fn rx_receives_byte() {
        let cfg = simple_config();
        let cpb = cfg.cycles_per_bit;
        let mut uart = Uart::new(cfg, 16);

        let test_byte: u8 = 0x3C;

        // Feed start bit (low)
        for _ in 0..cpb {
            uart.rx_feed(false);
        }

        // Feed 8 data bits (LSB first)
        for bit in 0..8 {
            let level = (test_byte >> bit) & 1 != 0;
            for _ in 0..cpb {
                uart.rx_feed(level);
            }
        }

        // Feed stop bit (high)
        for _ in 0..cpb {
            uart.rx_feed(true);
        }

        // Byte should be available
        assert!(uart.rx_available());
        assert_eq!(uart.rx_read(), Some(test_byte));
    }

    #[test]
    fn tx_fifo_overflow_returns_false() {
        let mut uart = Uart::new(simple_config(), 2);
        assert!(uart.tx_write(0x01));
        assert!(uart.tx_write(0x02));
        assert!(!uart.tx_write(0x03)); // FIFO full
    }

    #[test]
    fn rx_false_start_rejected() {
        let cfg = simple_config();
        let cpb = cfg.cycles_per_bit;
        let mut uart = Uart::new(cfg, 16);

        // Brief low pulse (less than half bit period) = false start
        for _ in 0..cpb / 4 {
            uart.rx_feed(false);
        }
        // Back to idle
        for _ in 0..cpb * 2 {
            uart.rx_feed(true);
        }

        assert!(!uart.rx_available());
    }

    #[test]
    fn reset_clears_fifos_and_state() {
        let mut uart = Uart::new(simple_config(), 16);
        uart.tx_write(0x42);
        uart.tx_tick();
        uart.reset();

        assert!(uart.tx_empty());
        assert!(uart.tx_line());
        assert!(!uart.rx_available());
    }

    #[test]
    fn asr33_config() {
        let cfg = SerialConfig::asr33(740_000);
        assert_eq!(cfg.baud_rate, 110);
        assert_eq!(cfg.data_bits, 8);
        assert_eq!(cfg.stop_bits, 2);
        assert_eq!(cfg.cycles_per_bit, 740_000 / 110);
    }

    #[test]
    fn multiple_bytes_queued() {
        let mut uart = Uart::new(simple_config(), 16);
        uart.tx_write(0x41); // 'A'
        uart.tx_write(0x42); // 'B'
        assert_eq!(uart.tx_pending(), 2);

        // Start transmitting first byte
        uart.tx_tick();
        assert_eq!(uart.tx_pending(), 1); // one popped from FIFO
        assert!(!uart.tx_empty()); // still transmitting
    }
}
