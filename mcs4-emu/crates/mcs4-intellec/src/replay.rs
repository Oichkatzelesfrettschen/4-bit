//! Deterministic replay for source-gated Intellec panel and terminal events.
//!
//! The transcript contains only explicit panel and terminal events plus phase
//! advances. A replay starts from a fresh machine and a copied immutable
//! profile. It never serializes host process state or claims a physical-board
//! snapshot.

use mcs4_system::{Mcs40System, Mcs4System};

use crate::{IntellecBusMachine, IntellecEvent, IntellecFrame, IntellecMachine, IntellecMachineError, IntellecProfile};

/// Schema version for Intellec event transcripts.
pub const INTELLEC_REPLAY_SCHEMA_VERSION: u32 = 1;

/// A command applied in order to an Intellec machine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntellecReplayCommand {
    /// Apply a physical panel or terminal event.
    Event(IntellecEvent),
    /// Advance exactly one observed bus phase.
    StepPhase,
}

/// One complete, replayable Intellec event transcript.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntellecReplayLog {
    /// Schema version for compatibility checks.
    pub schema_version: u32,
    /// Immutable evidence profile used for each replay.
    pub profile: IntellecProfile,
    /// Ordered external events and phase advances.
    pub commands: Vec<IntellecReplayCommand>,
}

/// Restorable observation boundary for an Intellec transcript.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntellecReplayCheckpoint {
    /// Schema version for compatibility checks.
    pub schema_version: u32,
    /// Full transcript through the observed frame.
    pub replay_log: IntellecReplayLog,
    /// Final exact observation produced by the transcript.
    pub expected_frame: IntellecFrame,
}

/// Replay failures preserve the recorded transcript and profile boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntellecReplayError {
    /// The transcript uses an unsupported schema version.
    UnsupportedSchema(u32),
    /// The source gate or machine rejects a phase advance.
    Machine(IntellecMachineError),
    /// The rebuilt observation differs from the checkpoint observation.
    FrameMismatch,
    /// A checkpoint requires at least one completed machine phase.
    MissingFrame,
}

impl std::fmt::Display for IntellecReplayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema(version) => write!(formatter, "unsupported Intellec replay schema {version}"),
            Self::Machine(error) => write!(formatter, "Intellec replay machine error: {error:?}"),
            Self::FrameMismatch => formatter.write_str("Intellec replay frame does not reproduce"),
            Self::MissingFrame => formatter.write_str("Intellec replay checkpoint requires a completed phase"),
        }
    }
}

impl std::error::Error for IntellecReplayError {}

/// Fresh-machine constructor required by deterministic Intellec replay.
pub trait IntellecReplayTarget: IntellecBusMachine {
    /// Build the target's fixed reset state.
    fn fresh_intellec_target() -> Self;
}

impl IntellecReplayTarget for Mcs4System {
    fn fresh_intellec_target() -> Self {
        Self::minimal()
    }
}

impl IntellecReplayTarget for Mcs40System {
    fn fresh_intellec_target() -> Self {
        Self::minimal()
    }
}

/// One-owner replay session with no hidden input path.
pub struct IntellecReplaySession<M: IntellecReplayTarget> {
    machine: IntellecMachine<M>,
    replay_log: IntellecReplayLog,
    last_frame: Option<IntellecFrame>,
}

impl<M: IntellecReplayTarget> IntellecReplaySession<M> {
    /// Create a fresh machine from the supplied immutable profile.
    pub fn new(profile: IntellecProfile) -> Self {
        Self {
            machine: IntellecMachine::new(M::fresh_intellec_target(), profile.clone()),
            replay_log: IntellecReplayLog {
                schema_version: INTELLEC_REPLAY_SCHEMA_VERSION,
                profile,
                commands: Vec::new(),
            },
            last_frame: None,
        }
    }

    /// Return the owned source-gated machine for inspection.
    pub const fn machine(&self) -> &IntellecMachine<M> {
        &self.machine
    }

    /// Return the exact transcript recorded so far.
    pub const fn replay_log(&self) -> &IntellecReplayLog {
        &self.replay_log
    }

    /// Apply and record one explicit external event.
    pub fn apply_event(&mut self, event: IntellecEvent) {
        self.machine.apply_event(event.clone());
        self.replay_log.commands.push(IntellecReplayCommand::Event(event));
    }

    /// Advance and record one complete machine phase.
    pub fn step_phase(&mut self) -> Result<&IntellecFrame, IntellecReplayError> {
        let frame = self.machine.step_phase().map_err(IntellecReplayError::Machine)?;
        self.replay_log.commands.push(IntellecReplayCommand::StepPhase);
        self.last_frame = Some(frame);
        Ok(self.last_frame.as_ref().expect("stored replay frame"))
    }

    /// Capture an exact replay checkpoint after one observed phase.
    pub fn checkpoint(&self) -> Result<IntellecReplayCheckpoint, IntellecReplayError> {
        Ok(IntellecReplayCheckpoint {
            schema_version: INTELLEC_REPLAY_SCHEMA_VERSION,
            replay_log: self.replay_log.clone(),
            expected_frame: self.last_frame.clone().ok_or(IntellecReplayError::MissingFrame)?,
        })
    }

    /// Rebuild a fresh target from an exact transcript and validate its frame.
    pub fn restore_from_checkpoint(checkpoint: IntellecReplayCheckpoint) -> Result<Self, IntellecReplayError> {
        if checkpoint.schema_version != INTELLEC_REPLAY_SCHEMA_VERSION
            || checkpoint.replay_log.schema_version != INTELLEC_REPLAY_SCHEMA_VERSION
        {
            return Err(IntellecReplayError::UnsupportedSchema(checkpoint.schema_version));
        }

        let mut session = Self::new(checkpoint.replay_log.profile.clone());
        for command in &checkpoint.replay_log.commands {
            match command {
                IntellecReplayCommand::Event(event) => session.apply_event(event.clone()),
                IntellecReplayCommand::StepPhase => {
                    let _ = session.step_phase()?;
                }
            }
        }
        if session.last_frame.as_ref() != Some(&checkpoint.expected_frame) {
            return Err(IntellecReplayError::FrameMismatch);
        }
        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use mcs4_system::Mcs4System;

    use super::{IntellecReplayError, IntellecReplaySession};
    use crate::{
        IntellecEvent, IntellecProfile, PanelControl, PanelInput, ProgramMemoryMode, RamPortEndpoint, TerminalWiring,
    };

    fn bench_profile() -> IntellecProfile {
        IntellecProfile::bench(
            110,
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
        )
    }

    #[test]
    fn checkpoint_replays_explicit_panel_and_terminal_events() {
        let mut session = IntellecReplaySession::<Mcs4System>::new(bench_profile());
        session.apply_event(IntellecEvent::Panel(PanelInput::SetProgramMemoryMode(
            ProgramMemoryMode::Monitor,
        )));
        session.apply_event(IntellecEvent::TerminalKey(b'A'));
        session.apply_event(IntellecEvent::Panel(PanelInput::Control(PanelControl::Run)));
        let _ = session.step_phase().expect("bench profile advances");
        let checkpoint = session.checkpoint().expect("completed frame");
        let restored = IntellecReplaySession::<Mcs4System>::restore_from_checkpoint(checkpoint).expect("exact replay");
        assert_eq!(restored.replay_log(), session.replay_log());
    }

    #[test]
    fn historical_profile_rejects_phase_advance_without_evidence() {
        let mut session = IntellecReplaySession::<Mcs4System>::new(IntellecProfile::intellec4());
        session.apply_event(IntellecEvent::Panel(PanelInput::Control(PanelControl::Run)));
        assert!(matches!(session.step_phase(), Err(IntellecReplayError::Machine(_))));
    }
}
