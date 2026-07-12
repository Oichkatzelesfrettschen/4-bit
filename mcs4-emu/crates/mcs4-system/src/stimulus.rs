//! Common bounded stimulus for behavioral and FPGA MCS-4 adapters.

use serde::{Deserialize, Serialize};

/// Schema version accepted by the common MCS-4 stimulus contract.
pub const COMMON_STIMULUS_SCHEMA_VERSION: u32 = 1;

/// Stable target name for a stimulus that both MCS-4 adapters accept.
pub const COMMON_STIMULUS_TARGET: &str = "mcs4-common-stimulus";

/// Maximum ROM bytes that the shared one-4001 system can load.
pub const COMMON_STIMULUS_ROM_BYTES: usize = 256;

/// Maximum actions accepted by either adapter.
pub const COMMON_STIMULUS_MAX_ACTIONS: usize = 100_000;

/// Maximum phase boundaries requested by one common stimulus.
pub const COMMON_STIMULUS_MAX_PHASES: u64 = 1_000_000;

/// Deterministic input accepted by behavioral replay and the system Verilator adapter.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommonStimulus {
    /// Schema version that controls parsing and execution semantics.
    pub schema_version: u32,
    /// Stable common target name.
    pub target: String,
    /// Exactly 256 ROM bytes written as hexadecimal pairs with optional ASCII whitespace.
    pub rom_hex: String,
    /// Ordered external actions executed by both adapters.
    pub actions: Vec<CommonStimulusAction>,
}

/// One action in the common behavioral and FPGA input intersection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum CommonStimulusAction {
    /// Restore the adapter-specific baseline state before subsequent actions.
    Reset,
    /// Drive the CPU TEST input before subsequent phase boundaries.
    SetTest {
        /// Logical TEST input level.
        value: bool,
    },
    /// Advance through exactly this many observed CPU phase boundaries.
    RunPhases {
        /// Positive number of phase boundaries to observe.
        value: u64,
    },
}

impl CommonStimulus {
    /// Parse, validate, and decode the shared ROM image.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let stimulus: Self =
            serde_json::from_slice(bytes).map_err(|error| format!("parse common stimulus JSON: {error}"))?;
        stimulus.validate()?;
        Ok(stimulus)
    }

    /// Validate the shared subset before either execution backend mutates state.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != COMMON_STIMULUS_SCHEMA_VERSION {
            return Err(format!(
                "unsupported common stimulus schema version {}",
                self.schema_version
            ));
        }
        if self.target != COMMON_STIMULUS_TARGET {
            return Err(format!("common stimulus target must be {COMMON_STIMULUS_TARGET}"));
        }
        if self.actions.is_empty() {
            return Err("common stimulus requires at least one action".to_owned());
        }
        if self.actions.len() > COMMON_STIMULUS_MAX_ACTIONS {
            return Err("common stimulus exceeds the action safety limit".to_owned());
        }
        if !matches!(self.actions.first(), Some(CommonStimulusAction::Reset)) {
            return Err("common stimulus must begin with reset".to_owned());
        }
        let _ = self.rom_bytes()?;
        let mut requested_phases = 0_u64;
        for action in &self.actions {
            if let CommonStimulusAction::RunPhases { value } = action {
                if *value == 0 {
                    return Err("common stimulus run_phases value must be positive".to_owned());
                }
                requested_phases = requested_phases
                    .checked_add(*value)
                    .ok_or_else(|| "common stimulus cumulative phase count overflows u64".to_owned())?;
                if requested_phases > COMMON_STIMULUS_MAX_PHASES {
                    return Err("common stimulus cumulative phase count exceeds the safety limit".to_owned());
                }
            }
        }
        Ok(())
    }

    /// Decode the fixed-size ROM image after validating hexadecimal spelling.
    pub fn rom_bytes(&self) -> Result<Vec<u8>, String> {
        let digits: String = self
            .rom_hex
            .chars()
            .filter(|character| !character.is_ascii_whitespace())
            .collect();
        if digits.len() != COMMON_STIMULUS_ROM_BYTES * 2 {
            return Err(format!(
                "common stimulus rom_hex must contain exactly {} hexadecimal bytes",
                COMMON_STIMULUS_ROM_BYTES
            ));
        }
        if !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("common stimulus rom_hex contains a non-hexadecimal byte".to_owned());
        }
        (0..COMMON_STIMULUS_ROM_BYTES)
            .map(|index| {
                u8::from_str_radix(&digits[index * 2..index * 2 + 2], 16)
                    .map_err(|error| format!("decode common stimulus ROM byte {index}: {error}"))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_stimulus() -> CommonStimulus {
        CommonStimulus {
            schema_version: COMMON_STIMULUS_SCHEMA_VERSION,
            target: COMMON_STIMULUS_TARGET.to_owned(),
            rom_hex: "00".repeat(COMMON_STIMULUS_ROM_BYTES),
            actions: vec![
                CommonStimulusAction::Reset,
                CommonStimulusAction::SetTest { value: false },
                CommonStimulusAction::RunPhases { value: 8 },
            ],
        }
    }

    #[test]
    fn common_stimulus_decodes_exact_rom_bytes() {
        let stimulus = valid_stimulus();
        assert_eq!(
            stimulus.rom_bytes().expect("decode ROM"),
            vec![0; COMMON_STIMULUS_ROM_BYTES]
        );
    }

    #[test]
    fn common_stimulus_requires_reset_before_observation() {
        let mut stimulus = valid_stimulus();
        stimulus.actions.remove(0);
        assert_eq!(
            stimulus.validate(),
            Err("common stimulus must begin with reset".to_owned())
        );
    }

    #[test]
    fn common_stimulus_rejects_nonpositive_phase_request() {
        let mut stimulus = valid_stimulus();
        stimulus.actions[2] = CommonStimulusAction::RunPhases { value: 0 };
        assert_eq!(
            stimulus.validate(),
            Err("common stimulus run_phases value must be positive".to_owned())
        );
    }
}
