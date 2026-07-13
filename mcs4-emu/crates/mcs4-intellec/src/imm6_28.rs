//! Source-bound imm6-28 / IN-28 program-RAM array.
//!
//! Drawing 01-0176-001 shows thirty-two 2102 devices in four banks.  Each
//! bank has eight bit lanes ordered K through C, which correspond to bits zero
//! through seven. The card routes its low address inputs through Intel 3404
//! active-low-write inverting latches. This module models the physical storage
//! organization without asserting the still-unextracted latch timing, control-
//! card timing, or bus ownership.

use mcs4_chips::i2102::{I2102Output, I2102};

/// A driven eight-bit word and the lanes whose values are established.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Imm628Read {
    /// Logic value for known and unknown lanes. Unknown lanes read as zero.
    pub value: u8,
    /// One bit for every lane with a known value.
    pub known_mask: u8,
}

/// One output state of the imm6-28 data bus.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Imm628Output {
    /// The module does not drive the program-memory data bus.
    HighImpedance,
    /// The module drives an eight-bit word, possibly with unknown lanes.
    Driven(Imm628Read),
}

/// One source-visible 2102 location on the IN-28 card.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Imm628ChipLocation {
    /// Zero-based 1 KiB bank selected by MAD10 and MAD11.
    pub bank: u8,
    /// Zero-based byte bit lane.
    pub bit: u8,
    /// Schematic designator such as `1K` or `4C`.
    pub designator: &'static str,
}

const DESIGNATORS: [[&str; 8]; 4] = [
    ["1K", "1J", "1H", "1G", "1F", "1E", "1D", "1C"],
    ["2K", "2J", "2H", "2G", "2F", "2E", "2D", "2C"],
    ["3K", "3J", "3H", "3G", "3F", "3E", "3D", "3C"],
    ["4K", "4J", "4H", "4G", "4F", "4E", "4D", "4C"],
];

/// The imm6-28 4 KiB by 8 program-RAM storage array.
pub struct Imm628 {
    chips: [[I2102; 8]; 4],
}

impl Imm628 {
    /// Construct an uninitialized module with all 2102 power-on bits unknown.
    pub fn new() -> Self {
        Self {
            chips: std::array::from_fn(|_| std::array::from_fn(|_| I2102::new())),
        }
    }

    /// Return the documented number of one-bit 2102 devices.
    pub const fn device_count(&self) -> usize {
        32
    }

    /// Map one byte address and bit lane to a source-visible chip designator.
    pub fn chip_location(address: u16, bit: u8) -> Imm628ChipLocation {
        assert!(bit < 8, "imm6-28 bit lane must be below eight");
        let bank = ((address >> 10) & 0x03) as usize;
        Imm628ChipLocation {
            bank: bank as u8,
            bit,
            designator: DESIGNATORS[bank][usize::from(bit)],
        }
    }

    /// Read a byte only when the module-selection net is asserted.
    pub fn read(&self, address: u16, selected: bool) -> Imm628Output {
        if !selected {
            return Imm628Output::HighImpedance;
        }

        let bank = ((address >> 10) & 0x03) as usize;
        let cell = address & 0x03ff;
        let mut value = 0u8;
        let mut known_mask = 0u8;
        for (bit, chip) in self.chips[bank].iter().enumerate() {
            match chip.read_direct(cell) {
                I2102Output::Zero => known_mask |= 1 << bit,
                I2102Output::One => {
                    value |= 1 << bit;
                    known_mask |= 1 << bit;
                }
                I2102Output::Unknown => {}
                I2102Output::HighImpedance => unreachable!("direct 2102 read cannot be high impedance"),
            }
        }
        Imm628Output::Driven(Imm628Read { value, known_mask })
    }

    /// Apply a byte write to the eight chips addressed by one IN-28 location.
    pub fn write(&mut self, address: u16, value: u8) {
        let bank = ((address >> 10) & 0x03) as usize;
        let cell = address & 0x03ff;
        for (bit, chip) in self.chips[bank].iter_mut().enumerate() {
            chip.write_direct(cell, (value & (1 << bit)) != 0);
        }
    }

    /// Invalidate every 2102 cell after an explicit module power cycle.
    pub fn power_cycle(&mut self) {
        for bank in &mut self.chips {
            for chip in bank {
                chip.power_cycle();
            }
        }
    }
}

impl Default for Imm628 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Imm628, Imm628Output, Imm628Read};

    #[test]
    fn maps_all_bank_boundaries_to_the_documented_designators() {
        assert_eq!(Imm628::chip_location(0x000, 0).designator, "1K");
        assert_eq!(Imm628::chip_location(0x3ff, 7).designator, "1C");
        assert_eq!(Imm628::chip_location(0x400, 0).designator, "2K");
        assert_eq!(Imm628::chip_location(0x800, 4).designator, "3F");
        assert_eq!(Imm628::chip_location(0xfff, 7).designator, "4C");
    }

    #[test]
    fn uninitialized_memory_reports_unknown_lanes() {
        let ram = Imm628::new();
        assert_eq!(
            ram.read(0x000, true),
            Imm628Output::Driven(Imm628Read {
                value: 0,
                known_mask: 0,
            })
        );
    }

    #[test]
    fn module_deselection_is_high_impedance() {
        let ram = Imm628::new();
        assert_eq!(ram.read(0x000, false), Imm628Output::HighImpedance);
    }

    #[test]
    fn byte_writes_touch_exactly_one_bank_and_all_eight_lanes() {
        let mut ram = Imm628::new();
        ram.write(0x800, 0xa5);

        assert_eq!(
            ram.read(0x800, true),
            Imm628Output::Driven(Imm628Read {
                value: 0xa5,
                known_mask: 0xff,
            })
        );
        assert_eq!(
            ram.read(0x400, true),
            Imm628Output::Driven(Imm628Read {
                value: 0,
                known_mask: 0,
            })
        );
    }

    #[test]
    fn every_bank_uses_an_independent_1k_address_space() {
        let mut ram = Imm628::new();
        for (address, value) in [(0x000, 0x12), (0x3ff, 0x34), (0x400, 0x56), (0xfff, 0x78)] {
            ram.write(address, value);
        }

        for (address, value) in [(0x000, 0x12), (0x3ff, 0x34), (0x400, 0x56), (0xfff, 0x78)] {
            assert_eq!(
                ram.read(address, true),
                Imm628Output::Driven(Imm628Read {
                    value,
                    known_mask: 0xff,
                })
            );
        }
    }
}
