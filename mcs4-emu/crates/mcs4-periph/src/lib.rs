//! Peripheral device emulation for MCS-4 systems.
//!
//! Provides emulation of common peripherals connected to MCS-4
//! microcomputer systems via the 4-bit data bus:
//!
//! - [`seven_seg`]: 7-segment LED display driver (via 4003 shift register)
//! - [`keyboard`]: Matrix keyboard scanner (via ROM I/O ports)
//! - [`uart`]: UART/Teletype serial interface (bit-bang via ROM I/O)

pub mod keyboard;
pub mod seven_seg;
pub mod uart;

pub use keyboard::{KeyEvent, KeyState, MatrixKeyboard};
pub use seven_seg::SevenSegDisplay;
pub use uart::{SerialConfig, Uart};
