//! Checksum-pinned monitor ROM media.

use sha2::{Digest, Sha256};

use crate::profile::IntellecModel;

/// Immutable monitor image with source and slot provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonitorRom {
    model: IntellecModel,
    load_address: u16,
    source_id: String,
    sha256: String,
    bytes: Vec<u8>,
}

/// Monitor-media validation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MonitorRomError {
    /// The image has no executable bytes.
    EmptyImage,
    /// The retained image hash differs from its declared source hash.
    DigestMismatch {
        /// Digest declared by the source ledger.
        expected: String,
        /// Digest computed from the supplied bytes.
        actual: String,
    },
    /// The profile and image identify different hardware families.
    ModelMismatch {
        /// Profile model identity.
        profile: IntellecModel,
        /// Image model identity.
        image: IntellecModel,
    },
}

impl MonitorRom {
    /// Validate and retain one source-proven monitor image.
    pub fn from_bytes(
        model: IntellecModel,
        load_address: u16,
        source_id: impl Into<String>,
        expected_sha256: impl Into<String>,
        bytes: Vec<u8>,
    ) -> Result<Self, MonitorRomError> {
        if bytes.is_empty() {
            return Err(MonitorRomError::EmptyImage);
        }
        let expected_sha256 = expected_sha256.into();
        let actual_sha256 = format!("{:x}", Sha256::digest(&bytes));
        if actual_sha256 != expected_sha256 {
            return Err(MonitorRomError::DigestMismatch {
                expected: expected_sha256,
                actual: actual_sha256,
            });
        }
        Ok(Self {
            model,
            load_address,
            source_id: source_id.into(),
            sha256: actual_sha256,
            bytes,
        })
    }

    /// Return the target historical model.
    pub const fn model(&self) -> IntellecModel {
        self.model
    }

    /// Return the requested program-memory load address.
    pub const fn load_address(&self) -> u16 {
        self.load_address
    }

    /// Return the source-ledger ID.
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    /// Return the verified image digest.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Return immutable program bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Reject a profile/image family mismatch before backplane loading.
    pub fn validate_model(&self, profile: IntellecModel) -> Result<(), MonitorRomError> {
        if self.model == profile || profile == IntellecModel::Bench {
            Ok(())
        } else {
            Err(MonitorRomError::ModelMismatch {
                profile,
                image: self.model,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IntellecModel, MonitorRom, MonitorRomError};

    #[test]
    fn monitor_rom_rejects_changed_media() {
        let error = MonitorRom::from_bytes(IntellecModel::Intellec4, 0, "fixture", "00", vec![0])
            .expect_err("wrong digest rejects monitor media");
        assert!(matches!(error, MonitorRomError::DigestMismatch { .. }));
    }

    #[test]
    fn monitor_rom_accepts_matching_digest() {
        let bytes = vec![0, 1, 2, 3];
        let digest = "054edec1d0211f624fed0cbca9d4f9400b0e491c43742af2c5b0abebf0c990d8";
        let rom = MonitorRom::from_bytes(IntellecModel::Intellec4, 0, "fixture", digest, bytes)
            .expect("matching digest accepts monitor media");
        assert_eq!(rom.bytes(), &[0, 1, 2, 3]);
    }
}
