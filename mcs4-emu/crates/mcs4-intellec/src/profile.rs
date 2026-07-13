//! Immutable, source-bound Intellec machine profiles.

/// Historically distinct development-system families.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntellecModel {
    /// The original 4004-based Intellec 4.
    Intellec4,
    /// The 4040-based Intellec 4/MOD 40.
    Intellec4Mod40,
    /// A nonhistorical bench assembly used only for controlled tests.
    Bench,
}

/// One primary-source locator that binds a profile claim to retained evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceReference {
    /// Stable source-ledger identifier.
    pub id: &'static str,
    /// Repository-local source path or local-only cache path.
    pub path: &'static str,
    /// SHA-256 of the referenced source scan.
    pub sha256: &'static str,
    /// Printed page, PDF page, or schematic sheet supporting the claim.
    pub locator: &'static str,
}

/// Documented card locations in an Intellec chassis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardSlot {
    /// Central processor card.
    Cpu,
    /// Program-memory control card.
    MemoryControl,
    /// RAM program-memory card.
    ProgramRam,
    /// PROM programming card.
    PromProgrammer,
    /// Optional instruction/data storage card.
    InstructionDataStorage,
    /// Optional data-storage card.
    DataStorage,
    /// Optional general I/O card.
    InputOutput,
    /// Optional PROM-memory card.
    PromMemory,
    /// Optional universal prototype card.
    Prototype,
    /// Optional high-speed paper reader.
    PaperReader,
}

/// A card that the historical system literature names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardKind {
    /// imm4-42 4004 central processor module.
    Imm442,
    /// imm4-43 4040 central processor module.
    Imm443,
    /// imm4-72 memory-control module.
    Imm472,
    /// imm6-28 program RAM memory module.
    Imm628,
    /// imm6-76 PROM programmer module.
    Imm676,
    /// imm4-22 instruction/data storage module.
    Imm422,
    /// imm4-24 data storage module.
    Imm424,
    /// imm4-60 input/output module.
    Imm460,
    /// imm6-26 PROM memory module.
    Imm626,
    /// imm6-70 universal prototype module.
    Imm670,
    /// imm4-90 high-speed paper reader.
    Imm490,
}

/// One fixed card placement inside a profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CardPlacement {
    /// Chassis slot role.
    pub slot: CardSlot,
    /// Installed historical card.
    pub card: CardKind,
}

/// One bit on a 4002 RAM output port.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RamPortEndpoint {
    /// 4002 bank identity.
    pub bank_id: u8,
    /// 4002 chip identity inside the bank.
    pub chip_id: u8,
    /// Output-port data-line index.
    pub bit: u8,
}

/// Source-bound mapping for the three terminal wires.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalWiring {
    /// ROM input port that samples terminal serial output.
    pub terminal_input_rom_chip_id: u8,
    /// Input-port bit sampled from terminal serial output.
    pub terminal_to_machine_bit: u8,
    /// RAM output port bit that drives terminal serial input.
    pub printer_output: RamPortEndpoint,
    /// RAM output port bit that controls the paper-reader relay.
    pub reader_control: RamPortEndpoint,
}

/// An immutable profile that never silently invents missing board evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntellecProfile {
    model: IntellecModel,
    phase_ticks_per_second: u64,
    cards: &'static [CardPlacement],
    sources: &'static [SourceReference],
    terminal_wiring: Option<TerminalWiring>,
    missing_evidence: &'static [&'static str],
}

/// A failed historical-fidelity gate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileEvidenceError {
    /// Machine profile that remains blocked.
    pub model: IntellecModel,
    /// Stable source-ledger IDs that must be resolved.
    pub missing: &'static [&'static str],
}

const INTELLEC4_SOURCES: &[SourceReference] = &[SourceReference {
    id: "intellec4-microcomputer-modules-jan74",
    path: "docs/MCS-4/dev_systems/Intel_Intellec_4_and_Micro_Computer_Modules_Jan74.pdf",
    sha256: "07275bbc06158d6c1ef1c12b40861fcdf6824f8887428fb78d9bf223f4985fdd",
    locator: "PDF pages 3-7",
}];

pub(crate) const MOD40_SOURCES: &[SourceReference] = &[
    SourceReference {
        id: "mcs40-users-manual-nov74",
        path: "docs/MCS-40/MCS-40_Users_Manual_Nov74.pdf",
        sha256: "fe37193f3e0dcba6b176f29a67420d2ca7a0d4f548b6710d5705af0a6e00d63b",
        locator: "PDF pages 57 and 97-99",
    },
    SourceReference {
        id: "intellec4-mod40-reference-schematics",
        path: "docs/MCS-40/Intel_Intellec_4_MOD_40_Reference_Schematics.pdf",
        sha256: "34f82ae6228f6ee0832c9696b4a0e922f8ea4bfccf92131476d013108b703bdb",
        locator: "console, backplane, and terminal sheets",
    },
    SourceReference {
        id: "intellec4-mod40-reference-manual-98-095a",
        path: "docs/evidence/local_sources/intellec/98-095A_Intellec_4_MOD_40_Reference_Manual_Nov74.pdf",
        sha256: "5b613ce889fbf53d2c46434e2e1eae053745b3acb33146a244211d93b7ac4acc",
        locator: "printed pages 20-23 and 97-99",
    },
];

/// Evidence that must exist before the standard MOD 40 can execute a phase.
pub const MOD40_HISTORICAL_EXECUTION_GATES: &[&str] = &[
    "intellec4-mod40-clock-net-map",
    "intellec4-mod40-control-program-ram-net-map",
    "intellec4-mod40-panel-net-map",
    "intellec4-mod40-terminal-electrical-net-map",
    "intellec4-mod40-monitor-prom-set",
];

const INTELLEC4_CARDS: &[CardPlacement] = &[
    CardPlacement {
        slot: CardSlot::Cpu,
        card: CardKind::Imm442,
    },
    CardPlacement {
        slot: CardSlot::MemoryControl,
        card: CardKind::Imm472,
    },
    CardPlacement {
        slot: CardSlot::ProgramRam,
        card: CardKind::Imm628,
    },
    CardPlacement {
        slot: CardSlot::PromProgrammer,
        card: CardKind::Imm676,
    },
];

const MOD40_CARDS: &[CardPlacement] = &[
    CardPlacement {
        slot: CardSlot::Cpu,
        card: CardKind::Imm443,
    },
    CardPlacement {
        slot: CardSlot::MemoryControl,
        card: CardKind::Imm472,
    },
    CardPlacement {
        slot: CardSlot::ProgramRam,
        card: CardKind::Imm628,
    },
    CardPlacement {
        slot: CardSlot::PromProgrammer,
        card: CardKind::Imm676,
    },
];

impl IntellecProfile {
    /// Return the original 4004 profile without inventing absent console nets.
    pub const fn intellec4() -> Self {
        Self {
            model: IntellecModel::Intellec4,
            phase_ticks_per_second: 750_000,
            cards: INTELLEC4_CARDS,
            sources: INTELLEC4_SOURCES,
            terminal_wiring: None,
            missing_evidence: &[
                "intellec4-operator-manual",
                "intellec4-console-schematics",
                "intellec4-monitor-prom-set",
            ],
        }
    }

    /// Return the MOD 40 profile and retain its unresolved monitor-image gate.
    pub const fn intellec4_mod40() -> Self {
        Self {
            model: IntellecModel::Intellec4Mod40,
            phase_ticks_per_second: 740_714,
            cards: MOD40_CARDS,
            sources: MOD40_SOURCES,
            terminal_wiring: Some(TerminalWiring {
                terminal_input_rom_chip_id: 0,
                terminal_to_machine_bit: 0,
                printer_output: RamPortEndpoint {
                    bank_id: 0,
                    chip_id: 0,
                    bit: 0,
                },
                reader_control: RamPortEndpoint {
                    bank_id: 0,
                    chip_id: 1,
                    bit: 0,
                },
            }),
            missing_evidence: MOD40_HISTORICAL_EXECUTION_GATES,
        }
    }

    /// Construct a nonhistorical bench profile with explicit wiring.
    pub const fn bench(phase_ticks_per_second: u64, terminal_wiring: TerminalWiring) -> Self {
        Self {
            model: IntellecModel::Bench,
            phase_ticks_per_second,
            cards: &[],
            sources: &[],
            terminal_wiring: Some(terminal_wiring),
            missing_evidence: &[],
        }
    }

    /// Return the historical model identity.
    pub const fn model(&self) -> IntellecModel {
        self.model
    }

    /// Return the profile phase-clock rate.
    pub const fn phase_ticks_per_second(&self) -> u64 {
        self.phase_ticks_per_second
    }

    /// Return the fixed card inventory.
    pub const fn cards(&self) -> &'static [CardPlacement] {
        self.cards
    }

    /// Return every source reference that supports this profile.
    pub const fn sources(&self) -> &'static [SourceReference] {
        self.sources
    }

    /// Return the terminal-port mapping when a source has established it.
    pub const fn terminal_wiring(&self) -> Option<TerminalWiring> {
        self.terminal_wiring
    }

    /// Reject historical boot when the required source set remains incomplete.
    pub fn validate_boot_evidence(&self) -> Result<(), ProfileEvidenceError> {
        if self.missing_evidence.is_empty() {
            Ok(())
        } else {
            Err(ProfileEvidenceError {
                model: self.model,
                missing: self.missing_evidence,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IntellecModel, IntellecProfile, RamPortEndpoint, TerminalWiring};

    #[test]
    fn original_profile_keeps_missing_evidence_visible() {
        let error = IntellecProfile::intellec4()
            .validate_boot_evidence()
            .expect_err("historical profile retains unresolved evidence");
        assert_eq!(error.model, IntellecModel::Intellec4);
        assert!(error.missing.contains(&"intellec4-monitor-prom-set"));
    }

    #[test]
    fn bench_profile_is_explicitly_bootable() {
        let profile = IntellecProfile::bench(
            750_000,
            TerminalWiring {
                terminal_input_rom_chip_id: 0,
                terminal_to_machine_bit: 0,
                printer_output: RamPortEndpoint {
                    bank_id: 0,
                    chip_id: 0,
                    bit: 1,
                },
                reader_control: RamPortEndpoint {
                    bank_id: 0,
                    chip_id: 0,
                    bit: 2,
                },
            },
        );
        assert_eq!(profile.validate_boot_evidence(), Ok(()));
        assert_eq!(profile.model(), IntellecModel::Bench);
    }

    #[test]
    fn mod40_profile_models_proven_terminal_endpoints_but_blocks_unextracted_board_nets() {
        let profile = IntellecProfile::intellec4_mod40();
        let wiring = profile.terminal_wiring().expect("MOD 40 terminal endpoints");

        assert_eq!(wiring.terminal_input_rom_chip_id, 0);
        assert_eq!(wiring.terminal_to_machine_bit, 0);
        assert_eq!(
            wiring.printer_output,
            RamPortEndpoint {
                bank_id: 0,
                chip_id: 0,
                bit: 0
            }
        );
        assert_eq!(
            wiring.reader_control,
            RamPortEndpoint {
                bank_id: 0,
                chip_id: 1,
                bit: 0
            }
        );
        let error = profile
            .validate_boot_evidence()
            .expect_err("historical board evidence remains unresolved");
        assert!(error.missing.contains(&"intellec4-mod40-clock-net-map"));
        assert!(error.missing.contains(&"intellec4-mod40-monitor-prom-set"));
    }
}
