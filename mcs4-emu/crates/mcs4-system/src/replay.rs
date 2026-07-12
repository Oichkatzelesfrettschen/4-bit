//! Deterministic behavioral replay from explicit input and phase commands.
//!
//! A replay checkpoint stores an executable transcript and its final observed
//! frame. Restoration reconstructs a fresh behavioral target and replays the
//! transcript. It does not claim to snapshot electrical solver state.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    trace::{PhaseTrace, TraceFrame, TraceFrameError, TraceProvenance, TraceStimulusKind},
    Mcs40System, Mcs4System, System,
};

/// Schema version for replay transcripts and checkpoints.
pub const REPLAY_SCHEMA_VERSION: u32 = 1;

/// Behavioral system that can execute one replay transcript.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReplayTargetKind {
    /// One 4004, one 4001, and one 4002.
    Mcs4Minimal,
    /// One 4040, one 4201, one 4289, one 4001, and one 4002.
    Mcs40Minimal,
}

/// Explicit external input accepted by the v1 behavioral replay surface.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayInput {
    /// Replace ROM contents from byte zero onward.
    LoadRom { bytes: Vec<u8> },
    /// Apply native reset semantics for the selected system.
    Reset,
    /// Set the CPU program counter before the next phase.
    SetProgramCounter { address: u16 },
    /// Drive the CPU TEST input before the next phase.
    SetTestPin { high: bool },
    /// Drive one 4001 ROM input port before the next phase.
    SetRomPortInput { chip_id: u8, value: u8 },
}

/// One transcript command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayCommand {
    /// Apply one external input and associate it with a stable event ID.
    Input {
        /// Monotonic input event identity.
        event_id: u64,
        /// Input to apply before a phase step.
        input: ReplayInput,
    },
    /// Advance the target by exactly one observable bus phase.
    StepPhase,
}

/// Complete executable transcript for one behavioral run.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReplayLog {
    /// Schema version for transcript compatibility.
    pub schema_version: u32,
    /// Fresh target used to replay every command.
    pub target: ReplayTargetKind,
    /// Ordered input and phase commands.
    pub commands: Vec<ReplayCommand>,
}

/// Canonical external-input transcript used for a behavioral stimulus digest.
#[derive(Serialize)]
struct StimulusTranscript<'a> {
    schema_version: u32,
    target: ReplayTargetKind,
    inputs: Vec<StimulusInput<'a>>,
}

/// One external input in a canonical behavioral stimulus transcript.
#[derive(Serialize)]
struct StimulusInput<'a> {
    event_id: u64,
    input: &'a ReplayInput,
}

/// Restorable replay boundary after an observed frame.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ReplayCheckpoint {
    /// Schema version for checkpoint compatibility.
    pub schema_version: u32,
    /// Target that owns the replay semantics.
    pub target: ReplayTargetKind,
    /// Every command through the observed frame.
    pub replay_log: ReplayLog,
    /// Sequence of the final observed frame within its run.
    pub completed_sequence: u64,
    /// Canonical frame that restoration must regenerate exactly.
    pub expected_frame: TraceFrame,
}

/// Replay failure with no hidden state mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayError {
    /// The transcript or checkpoint uses an unsupported schema.
    UnsupportedSchema(u32),
    /// The transcript target does not match the requested session type.
    TargetMismatch {
        /// Expected target kind.
        expected: ReplayTargetKind,
        /// Actual target kind.
        actual: ReplayTargetKind,
    },
    /// The selected behavioral system cannot apply an input.
    UnsupportedInput {
        /// Target that rejected the input.
        target: ReplayTargetKind,
        /// Stable input name.
        input: &'static str,
    },
    /// Checkpoint creation needs at least one completed frame.
    MissingFrame,
    /// The replay transcript uses a discontinuous input event ID.
    InputEventSequence {
        /// Expected next input event ID.
        expected: u64,
        /// ID recorded in the transcript.
        actual: u64,
    },
    /// Reconstructed output diverges from the recorded checkpoint frame.
    FrameMismatch,
    /// A trace frame violates the shared trace contract.
    InvalidFrame(TraceFrameError),
    /// Canonical replay stimulus serialization failed.
    StimulusSerialization(String),
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema(version) => write!(formatter, "unsupported replay schema version {version}"),
            Self::TargetMismatch { expected, actual } => {
                write!(
                    formatter,
                    "replay target mismatch: expected {expected:?}, got {actual:?}"
                )
            }
            Self::UnsupportedInput { target, input } => {
                write!(formatter, "replay input {input} is unsupported by {target:?}")
            }
            Self::MissingFrame => formatter.write_str("replay checkpoint requires a completed trace frame"),
            Self::InputEventSequence { expected, actual } => {
                write!(
                    formatter,
                    "replay input event sequence expected {expected}, got {actual}"
                )
            }
            Self::FrameMismatch => formatter.write_str("replay checkpoint frame does not reproduce"),
            Self::InvalidFrame(error) => write!(formatter, "invalid replay trace frame: {error}"),
            Self::StimulusSerialization(error) => {
                write!(formatter, "serialize replay stimulus transcript: {error}")
            }
        }
    }
}

impl std::error::Error for ReplayError {}

/// Behavioral target that supports fresh construction, phase stepping, and replay inputs.
pub trait TraceReplayTarget: Sized {
    /// Stable target identity used by replay transcripts.
    const TARGET_KIND: ReplayTargetKind;

    /// Construct the exact baseline state used by replay restoration.
    fn fresh_replay_target() -> Self;

    /// Describe the target without claiming a sealed or physical artifact.
    fn replay_provenance() -> TraceProvenance;

    /// Advance exactly one observable bus phase.
    fn step_for_replay(&mut self) -> PhaseTrace;

    /// Apply one externally observable input before the next phase.
    fn apply_replay_input(&mut self, input: &ReplayInput) -> Result<(), ReplayError>;
}

impl TraceReplayTarget for Mcs4System {
    const TARGET_KIND: ReplayTargetKind = ReplayTargetKind::Mcs4Minimal;

    fn fresh_replay_target() -> Self {
        Self::minimal()
    }

    fn replay_provenance() -> TraceProvenance {
        TraceProvenance::behavioral_mcs4()
    }

    fn step_for_replay(&mut self) -> PhaseTrace {
        self.step_traced()
    }

    fn apply_replay_input(&mut self, input: &ReplayInput) -> Result<(), ReplayError> {
        match input {
            ReplayInput::LoadRom { bytes } => self.load_rom(bytes),
            ReplayInput::Reset => self.reset(),
            ReplayInput::SetProgramCounter { address } => System::set_pc(self, *address),
            ReplayInput::SetTestPin { high } => self.set_test_pin(*high),
            ReplayInput::SetRomPortInput { chip_id, value } => self.write_rom_port_input(*chip_id, *value),
        }
        Ok(())
    }
}

impl TraceReplayTarget for Mcs40System {
    const TARGET_KIND: ReplayTargetKind = ReplayTargetKind::Mcs40Minimal;

    fn fresh_replay_target() -> Self {
        Self::minimal()
    }

    fn replay_provenance() -> TraceProvenance {
        TraceProvenance::behavioral_mcs40()
    }

    fn step_for_replay(&mut self) -> PhaseTrace {
        self.step_traced()
    }

    fn apply_replay_input(&mut self, input: &ReplayInput) -> Result<(), ReplayError> {
        match input {
            ReplayInput::LoadRom { bytes } => self.load_rom(bytes),
            ReplayInput::Reset => System::reset(self),
            ReplayInput::SetProgramCounter { address } => System::set_pc(self, *address),
            ReplayInput::SetTestPin { high } => self.set_test_pin(*high),
            ReplayInput::SetRomPortInput { chip_id, value } => self.write_rom_port_input(*chip_id, *value),
        }
        Ok(())
    }
}

/// One-owner behavioral session with executable replay and frame checkpoints.
pub struct ReplaySession<T: TraceReplayTarget> {
    target: T,
    provenance: TraceProvenance,
    run_id: u64,
    next_sequence: u64,
    next_input_event_id: u64,
    last_input_event_id: u64,
    replay_log: ReplayLog,
    last_frame: Option<TraceFrame>,
}

impl<T: TraceReplayTarget> ReplaySession<T> {
    /// Construct a session from the exact fresh state used for restoration.
    pub fn new() -> Self {
        Self {
            target: T::fresh_replay_target(),
            provenance: T::replay_provenance(),
            run_id: 1,
            next_sequence: 1,
            next_input_event_id: 1,
            last_input_event_id: 0,
            replay_log: ReplayLog {
                schema_version: REPLAY_SCHEMA_VERSION,
                target: T::TARGET_KIND,
                commands: Vec::new(),
            },
            last_frame: None,
        }
    }

    /// Return the owned behavioral target for read-only inspection.
    pub fn target(&self) -> &T {
        &self.target
    }

    /// Return the stable identity of the active replay run.
    pub const fn run_id(&self) -> u64 {
        self.run_id
    }

    /// Return the transcript through the most recent command.
    pub fn replay_log(&self) -> &ReplayLog {
        &self.replay_log
    }

    /// Return the most recent completed frame.
    pub fn last_frame(&self) -> Option<&TraceFrame> {
        self.last_frame.as_ref()
    }

    /// Apply and record one input before the next phase step.
    pub fn apply_input(&mut self, input: ReplayInput) -> Result<u64, ReplayError> {
        let event_id = self.next_input_event_id;
        self.apply_input_with_id(event_id, input, true)?;
        Ok(event_id)
    }

    /// Advance one phase, record the command, and return its canonical frame.
    pub fn step_phase(&mut self) -> Result<TraceFrame, ReplayError> {
        self.step_phase_inner(true)
    }

    /// Build a replay checkpoint after a completed observable frame.
    pub fn checkpoint(&self) -> Result<ReplayCheckpoint, ReplayError> {
        let expected_frame = self.last_frame.clone().ok_or(ReplayError::MissingFrame)?;
        Ok(ReplayCheckpoint {
            schema_version: REPLAY_SCHEMA_VERSION,
            target: T::TARGET_KIND,
            replay_log: self.replay_log.clone(),
            completed_sequence: expected_frame.sequence,
            expected_frame,
        })
    }

    /// Restore a fresh target by executing the checkpoint transcript exactly once.
    pub fn restore_from_checkpoint(checkpoint: ReplayCheckpoint) -> Result<Self, ReplayError> {
        if checkpoint.schema_version != REPLAY_SCHEMA_VERSION
            || checkpoint.replay_log.schema_version != REPLAY_SCHEMA_VERSION
        {
            return Err(ReplayError::UnsupportedSchema(checkpoint.schema_version));
        }
        if checkpoint.target != T::TARGET_KIND || checkpoint.replay_log.target != T::TARGET_KIND {
            return Err(ReplayError::TargetMismatch {
                expected: T::TARGET_KIND,
                actual: checkpoint.target,
            });
        }

        let mut session = Self::new();
        for command in &checkpoint.replay_log.commands {
            match command {
                ReplayCommand::Input { event_id, input } => {
                    session.apply_input_with_id(*event_id, input.clone(), true)?;
                }
                ReplayCommand::StepPhase => {
                    let _ = session.step_phase_inner(true)?;
                }
            }
        }

        let actual = session.last_frame.as_ref().ok_or(ReplayError::MissingFrame)?;
        if actual != &checkpoint.expected_frame || actual.sequence != checkpoint.completed_sequence {
            return Err(ReplayError::FrameMismatch);
        }
        Ok(session)
    }

    fn apply_input_with_id(&mut self, event_id: u64, input: ReplayInput, record: bool) -> Result<(), ReplayError> {
        if event_id != self.next_input_event_id {
            return Err(ReplayError::InputEventSequence {
                expected: self.next_input_event_id,
                actual: event_id,
            });
        }
        self.target.apply_replay_input(&input)?;
        self.next_input_event_id += 1;
        self.last_input_event_id = event_id;

        if matches!(input, ReplayInput::Reset) {
            self.run_id += 1;
            self.next_sequence = 1;
            self.last_frame = None;
        }
        if record {
            self.replay_log.commands.push(ReplayCommand::Input { event_id, input });
            self.update_stimulus_identity()?;
        }
        Ok(())
    }

    fn update_stimulus_identity(&mut self) -> Result<(), ReplayError> {
        let inputs = self
            .replay_log
            .commands
            .iter()
            .filter_map(|command| match command {
                ReplayCommand::Input { event_id, input } => Some(StimulusInput {
                    event_id: *event_id,
                    input,
                }),
                ReplayCommand::StepPhase => None,
            })
            .collect();
        let transcript = StimulusTranscript {
            schema_version: REPLAY_SCHEMA_VERSION,
            target: T::TARGET_KIND,
            inputs,
        };
        let bytes =
            serde_json::to_vec(&transcript).map_err(|error| ReplayError::StimulusSerialization(error.to_string()))?;
        self.provenance.stimulus_sha256 = Some(sha256(&bytes));
        self.provenance.stimulus_kind = Some(TraceStimulusKind::ReplayInputTranscript);
        Ok(())
    }

    fn step_phase_inner(&mut self, record: bool) -> Result<TraceFrame, ReplayError> {
        let phase = self.target.step_for_replay();
        let frame = TraceFrame::from_behavioral_phase(
            self.run_id,
            self.next_sequence,
            self.last_input_event_id,
            self.provenance.clone(),
            phase,
        );
        frame.validate().map_err(ReplayError::InvalidFrame)?;
        self.next_sequence += 1;
        self.last_frame = Some(frame.clone());
        if record {
            self.replay_log.commands.push(ReplayCommand::StepPhase);
        }
        Ok(frame)
    }
}

impl<T: TraceReplayTarget> Default for ReplaySession<T> {
    fn default() -> Self {
        Self::new()
    }
}

fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_records_phase_unique_frames() {
        let mut session = ReplaySession::<Mcs4System>::new();
        session
            .apply_input(ReplayInput::LoadRom { bytes: vec![0x00; 256] })
            .expect("load ROM");

        let frames: Vec<_> = (0..8).map(|_| session.step_phase().expect("step phase")).collect();

        let sequences: Vec<_> = frames.iter().map(|frame| frame.sequence).collect();
        assert_eq!(sequences, (1..=8).collect::<Vec<_>>());
        assert_eq!(
            frames[0].phase.as_ref().expect("phase").completed_phase,
            crate::TracePhase::A1
        );
        assert_eq!(
            frames[7].phase.as_ref().expect("phase").completed_phase,
            crate::TracePhase::X3
        );
    }

    #[test]
    fn checkpoint_replays_to_identical_behavioral_frame() {
        let mut session = ReplaySession::<Mcs4System>::new();
        session
            .apply_input(ReplayInput::LoadRom {
                bytes: vec![0x00, 0x00, 0x00, 0x00],
            })
            .expect("load ROM");
        session
            .apply_input(ReplayInput::SetTestPin { high: true })
            .expect("set TEST");
        for _ in 0..11 {
            let _ = session.step_phase().expect("step phase");
        }
        let checkpoint = session.checkpoint().expect("checkpoint");

        let restored =
            ReplaySession::<Mcs4System>::restore_from_checkpoint(checkpoint.clone()).expect("restore checkpoint");
        assert_eq!(restored.last_frame(), Some(&checkpoint.expected_frame));
        assert_eq!(restored.replay_log(), &checkpoint.replay_log);
    }

    #[test]
    fn reset_starts_a_new_run_without_reusing_frame_identity() {
        let mut session = ReplaySession::<Mcs4System>::new();
        let first = session.step_phase().expect("first phase");
        session.apply_input(ReplayInput::Reset).expect("reset");
        let second = session.step_phase().expect("second phase");

        assert_eq!(first.sequence, 1);
        assert_eq!(second.sequence, 1);
        assert_ne!(first.run_id, second.run_id);
    }

    #[test]
    fn comparable_frames_require_shared_stimulus_and_nonblocked_evidence() {
        let mut left_session = ReplaySession::<Mcs4System>::new();
        let mut right_session = ReplaySession::<Mcs4System>::new();
        let input = ReplayInput::LoadRom { bytes: vec![0x00; 4] };
        left_session.apply_input(input.clone()).expect("load left");
        right_session.apply_input(input).expect("load right");

        let left = left_session.step_phase().expect("left frame");
        let right = right_session.step_phase().expect("right frame");
        assert!(crate::frames_are_comparable(&left, &right));
    }

    #[test]
    fn non_rom_input_changes_the_canonical_stimulus_digest() {
        let mut left_session = ReplaySession::<Mcs4System>::new();
        let mut right_session = ReplaySession::<Mcs4System>::new();
        let input = ReplayInput::LoadRom { bytes: vec![0x00; 4] };
        left_session.apply_input(input.clone()).expect("load left");
        right_session.apply_input(input).expect("load right");
        left_session
            .apply_input(ReplayInput::SetTestPin { high: true })
            .expect("set left TEST");

        let left = left_session.step_phase().expect("step left");
        let right = right_session.step_phase().expect("step right");
        assert_ne!(left.provenance.stimulus_sha256, right.provenance.stimulus_sha256);
        assert!(!crate::frames_are_comparable(&left, &right));
    }
}
