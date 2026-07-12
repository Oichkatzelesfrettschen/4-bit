//! Single-owner behavioral session worker for the interactive GUI.
//!
//! The worker owns the mutable emulator. The UI sends bounded commands and
//! receives immutable trace frames, so Run, Step, and Reset never race on a
//! shared `Mcs4System` lock.

use std::{
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
};

use mcs4_system::{Mcs4System, ReplayInput, ReplaySession, TraceFrame};

/// Maximum phases accepted by one UI run request.
pub const MAX_RUN_PHASES: usize = 10_000;

/// Request sent from the GUI thread to the sole behavioral-system owner.
#[derive(Clone, Debug)]
pub enum SimulationCommand {
    /// Reset with the native MCS-4 reset semantics.
    Reset,
    /// Advance exactly one bus phase.
    StepPhase,
    /// Advance a bounded number of bus phases.
    RunPhases {
        /// Number of phases to execute.
        phases: usize,
    },
    /// Replace the loaded ROM image.
    LoadRom {
        /// ROM bytes starting at address zero.
        bytes: Vec<u8>,
    },
    /// Drive the TEST input before the next phase.
    SetTestPin {
        /// Logical level applied to TEST.
        high: bool,
    },
    /// End the worker thread.
    Shutdown,
}

/// Immutable observation or fault delivered to the GUI thread.
#[derive(Clone, Debug)]
pub enum SimulationEvent {
    /// Canonical post-phase observation.
    Frame(TraceFrame),
    /// A reset creates a new frame-identity run.
    RunBoundary {
        /// New run identity.
        run_id: u64,
        /// Stable reason for the boundary.
        reason: &'static str,
    },
    /// One bounded run request completes.
    BatchComplete,
    /// Command or replay failure without a worker crash.
    Fault {
        /// Actionable failure text.
        message: String,
    },
}

/// Command-channel failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationSessionError {
    /// The worker exits before receiving a command.
    WorkerUnavailable,
}

impl std::fmt::Display for SimulationSessionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkerUnavailable => formatter.write_str("simulation worker is unavailable"),
        }
    }
}

impl std::error::Error for SimulationSessionError {}

/// GUI-facing endpoint for one owned behavioral session.
pub struct SimulationSession {
    commands: Sender<SimulationCommand>,
    events: Receiver<SimulationEvent>,
}

impl SimulationSession {
    /// Spawn a fresh MCS-4 behavioral worker.
    pub fn spawn() -> Self {
        let (command_sender, command_receiver) = mpsc::channel();
        let (event_sender, event_receiver) = mpsc::channel();
        thread::Builder::new()
            .name("mcs4-behavioral-session".to_owned())
            .spawn(move || run_worker(command_receiver, event_sender))
            .expect("spawn MCS-4 behavioral session worker");
        Self {
            commands: command_sender,
            events: event_receiver,
        }
    }

    /// Send one bounded command to the worker.
    pub fn send(&self, command: SimulationCommand) -> Result<(), SimulationSessionError> {
        self.commands
            .send(command)
            .map_err(|_| SimulationSessionError::WorkerUnavailable)
    }

    /// Drain every event already available without blocking the GUI frame.
    pub fn drain_events(&self) -> Vec<SimulationEvent> {
        let mut events = Vec::new();
        loop {
            match self.events.try_recv() {
                Ok(event) => events.push(event),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return events,
            }
        }
    }
}

impl Default for SimulationSession {
    fn default() -> Self {
        Self::spawn()
    }
}

impl Drop for SimulationSession {
    fn drop(&mut self) {
        let _ = self.commands.send(SimulationCommand::Shutdown);
    }
}

fn run_worker(commands: Receiver<SimulationCommand>, events: Sender<SimulationEvent>) {
    let mut session = ReplaySession::<Mcs4System>::new();
    while let Ok(command) = commands.recv() {
        let should_continue = handle_command(&mut session, command, &events);
        if !should_continue {
            return;
        }
    }
}

fn handle_command(
    session: &mut ReplaySession<Mcs4System>,
    command: SimulationCommand,
    events: &Sender<SimulationEvent>,
) -> bool {
    match command {
        SimulationCommand::Shutdown => false,
        SimulationCommand::Reset => match session.apply_input(ReplayInput::Reset) {
            Ok(_) => {
                let _ = events.send(SimulationEvent::RunBoundary {
                    run_id: session.run_id(),
                    reason: "reset",
                });
                true
            }
            Err(error) => send_fault(events, error.to_string()),
        },
        SimulationCommand::StepPhase => {
            emit_phase(session, events);
            true
        }
        SimulationCommand::RunPhases { phases } => {
            if phases == 0 || phases > MAX_RUN_PHASES {
                return send_fault(
                    events,
                    format!("run request must contain 1 through {MAX_RUN_PHASES} phases"),
                );
            }
            for _ in 0..phases {
                if !emit_phase(session, events) {
                    return true;
                }
            }
            let _ = events.send(SimulationEvent::BatchComplete);
            true
        }
        SimulationCommand::LoadRom { bytes } => match session.apply_input(ReplayInput::LoadRom { bytes }) {
            Ok(_) => true,
            Err(error) => send_fault(events, error.to_string()),
        },
        SimulationCommand::SetTestPin { high } => match session.apply_input(ReplayInput::SetTestPin { high }) {
            Ok(_) => true,
            Err(error) => send_fault(events, error.to_string()),
        },
    }
}

fn emit_phase(session: &mut ReplaySession<Mcs4System>, events: &Sender<SimulationEvent>) -> bool {
    match session.step_phase() {
        Ok(frame) => events.send(SimulationEvent::Frame(frame)).is_ok(),
        Err(error) => send_fault(events, error.to_string()),
    }
}

fn send_fault(events: &Sender<SimulationEvent>, message: String) -> bool {
    let _ = events.send(SimulationEvent::Fault { message });
    true
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    fn wait_for_frame(session: &SimulationSession) -> TraceFrame {
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            for event in session.drain_events() {
                if let SimulationEvent::Frame(frame) = event {
                    return frame;
                }
            }
            assert!(Instant::now() < deadline, "session worker did not emit a frame");
            thread::yield_now();
        }
    }

    #[test]
    fn one_step_command_emits_one_post_phase_frame() {
        let session = SimulationSession::spawn();
        session.send(SimulationCommand::StepPhase).expect("send step");
        let frame = wait_for_frame(&session);
        assert_eq!(frame.sequence, 1);
        assert_eq!(frame.phase.expect("phase").completed_phase, mcs4_system::TracePhase::A1);
    }

    #[test]
    fn reset_emits_a_new_run_boundary_before_the_next_frame() {
        let session = SimulationSession::spawn();
        session.send(SimulationCommand::StepPhase).expect("first step");
        let first = wait_for_frame(&session);
        session.send(SimulationCommand::Reset).expect("reset");

        let deadline = Instant::now() + Duration::from_secs(1);
        let mut reset_run = None;
        while reset_run.is_none() {
            for event in session.drain_events() {
                if let SimulationEvent::RunBoundary { run_id, .. } = event {
                    reset_run = Some(run_id);
                }
            }
            assert!(
                Instant::now() < deadline,
                "session worker did not emit a reset boundary"
            );
            thread::yield_now();
        }
        session.send(SimulationCommand::StepPhase).expect("second step");
        let second = wait_for_frame(&session);

        assert_eq!(second.sequence, 1);
        assert_eq!(Some(second.run_id), reset_run);
        assert_ne!(first.run_id, second.run_id);
    }
}
