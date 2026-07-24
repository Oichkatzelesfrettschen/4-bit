//! Bundled MCS-4 programs the operator can load at runtime.
//!
//! The runtime is the single source of the scenario list. A frontend renders
//! `SCENARIOS` by index and loads a chosen program through
//! [`rom_image_from_hex`]; it keeps no private copy, so every frontend observes
//! the same program under the same stable id.

use mcs4_system::parse_hex_bytes;

/// 4001 ROM chip span a scenario image fills; the program sits at address zero
/// and the remaining bytes stay NOP.
pub const ROM_IMAGE_BYTES: usize = 256;

/// A selectable bundled program.
pub struct Scenario {
    /// Stable identifier, invariant across frontends and releases.
    pub id: &'static str,
    /// Human-readable menu label.
    pub name: &'static str,
    /// Whitespace-separated hex bytes of the ROM program.
    pub hex: &'static str,
}

/// Bundled MCS-4 programs. Each is a validated fixture under
/// `mcs4-system/fixtures`; the first boots by default so a frontend shows real
/// activity on launch instead of an inert zero-ROM machine.
pub const SCENARIOS: &[Scenario] = &[
    Scenario {
        id: "src_wrm_rdm",
        name: "RAM roundtrip (SRC/WRM/RDM)",
        hex: include_str!("../../mcs4-system/fixtures/src_wrm_rdm.hex"),
    },
    Scenario {
        id: "ram_status_wr1_rd1",
        name: "RAM status write/read",
        hex: include_str!("../../mcs4-system/fixtures/ram_status_wr1_rd1.hex"),
    },
    Scenario {
        id: "rom_port_wrr_rdr",
        name: "ROM port write/read",
        hex: include_str!("../../mcs4-system/fixtures/rom_port_wrr_rdr.hex"),
    },
    Scenario {
        id: "seven_seg_count",
        name: "7-segment counter",
        hex: include_str!("../../mcs4-system/fixtures/seven_seg_count.hex"),
    },
];

/// Build a 256-byte ROM image from a scenario's hex program at address zero,
/// leaving the remaining bytes as NOP.
pub fn rom_image_from_hex(hex: &str) -> Vec<u8> {
    let program = parse_hex_bytes(hex).expect("embedded scenario fixture parses");
    let mut image = vec![0u8; ROM_IMAGE_BYTES];
    let length = program.len().min(image.len());
    image[..length].copy_from_slice(&program[..length]);
    image
}

#[cfg(test)]
mod tests {
    use super::{rom_image_from_hex, ROM_IMAGE_BYTES, SCENARIOS};

    #[test]
    fn bundled_scenarios_embed_runnable_programs() {
        for scenario in SCENARIOS {
            let image = rom_image_from_hex(scenario.hex);
            assert_eq!(image.len(), ROM_IMAGE_BYTES);
            assert!(
                image.iter().any(|&byte| byte != 0),
                "scenario '{}' embeds a non-empty program",
                scenario.name
            );
        }
        // The default scenario opens with LDM 0xA (0xDA) then FIM P0, 0x01 (0x20 0x01).
        assert_eq!(&rom_image_from_hex(SCENARIOS[0].hex)[..3], &[0xDA, 0x20, 0x01]);
    }

    #[test]
    fn scenario_ids_are_unique_and_stable() {
        for (index, scenario) in SCENARIOS.iter().enumerate() {
            assert!(!scenario.id.is_empty(), "scenario {index} carries a stable id");
            for other in &SCENARIOS[index + 1..] {
                assert_ne!(scenario.id, other.id, "scenario ids are unique");
            }
        }
    }
}
