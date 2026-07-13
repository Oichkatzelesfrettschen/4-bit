//! Intel 1702A 256 by 8 UV-erasable PROM.
//!
//! The 1702A stores 256 eight-bit words.  A physical device retains its
//! programmed contents across reset, but the repository does not invent the
//! contents of an unread device.  [`I1702A::new`] therefore starts unread;
//! [`I1702A::erased`] exists for programmer tests that explicitly model a
//! blank device.

use mcs4_bus::BusCycle;

/// One read result from a 1702A device.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum I1702Read {
    /// The repository has not established this physical byte.
    Unknown,
    /// The device drives this byte.
    Value(u8),
}

/// Intel 1702A: 256 words by eight bits UV-erasable PROM.
#[derive(Clone, Debug)]
pub struct I1702A {
    bytes: [u8; 256],
    known: [bool; 256],
    /// Board socket identity.  This does not establish a physical socket map.
    pub socket: u8,
}

impl I1702A {
    /// Construct an unread physical device.
    pub fn new(socket: u8) -> Self {
        Self {
            bytes: [0; 256],
            known: [false; 256],
            socket,
        }
    }

    /// Construct a known blank device for explicit programmer tests.
    pub fn erased(socket: u8) -> Self {
        Self {
            bytes: [0xff; 256],
            known: [true; 256],
            socket,
        }
    }

    /// Load byte values whose content is established for a nonhistorical use.
    ///
    /// This method establishes only the device bytes. It does not claim a
    /// physical read, socket identity, transform, or independent provenance.
    /// A board-level historical gate must record those facts separately.
    pub fn load_established_bytes(&mut self, bytes: &[u8; 256]) {
        self.bytes = *bytes;
        self.known = [true; 256];
    }

    /// Read one byte without asserting an unproven board-level interface.
    pub fn read_direct(&self, address: u8) -> I1702Read {
        let index = address as usize;
        if self.known[index] {
            I1702Read::Value(self.bytes[index])
        } else {
            I1702Read::Unknown
        }
    }

    /// Return whether every byte has a recorded value.
    pub fn is_fully_known(&self) -> bool {
        self.known.iter().all(|known| *known)
    }

    /// Clear all byte provenance after a physical-media replacement.
    pub fn invalidate_contents(&mut self) {
        self.bytes = [0; 256];
        self.known = [false; 256];
    }
}

impl Default for I1702A {
    fn default() -> Self {
        Self::new(0)
    }
}

impl super::Chip for I1702A {
    fn name(&self) -> &'static str {
        "1702A"
    }

    fn reset(&mut self) {}

    fn tick(&mut self, _phase: BusCycle) {}
}

#[cfg(test)]
mod tests {
    use super::{I1702Read, I1702A};
    use crate::Chip;

    #[test]
    fn unread_device_does_not_fabricate_monitor_bytes() {
        let prom = I1702A::new(2);
        assert_eq!(prom.read_direct(0x00), I1702Read::Unknown);
        assert!(!prom.is_fully_known());
    }

    #[test]
    fn established_image_becomes_readable() {
        let mut prom = I1702A::new(1);
        let mut image = [0; 256];
        image[0x42] = 0xa5;
        prom.load_established_bytes(&image);

        assert_eq!(prom.read_direct(0x42), I1702Read::Value(0xa5));
        assert!(prom.is_fully_known());
    }

    #[test]
    fn erased_constructor_is_explicit() {
        let prom = I1702A::erased(3);
        assert_eq!(prom.read_direct(0xff), I1702Read::Value(0xff));
    }

    #[test]
    fn reset_preserves_programmed_contents() {
        let mut prom = I1702A::erased(0);
        prom.reset();
        assert_eq!(prom.read_direct(0), I1702Read::Value(0xff));
    }
}
