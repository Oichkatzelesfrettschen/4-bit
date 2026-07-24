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

use crate::panels::{memory::MemorySnapshot, registers::CpuSnapshot, stack::StackSnapshot};

/// Maximum phases accepted by one UI run request.
pub const MAX_RUN_PHASES: usize = 10_000;

/// ROM chip 0 span published to the memory panel.
const ROM_VIEW_BYTES: u16 = 256;
/// RAM main-memory characters published to the memory panel: 4 registers x 16 characters.
const RAM_VIEW_REGISTERS: u8 = 4;
const RAM_VIEW_CHARACTERS: u8 = 16;

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

/// Complete debugger view of the single worker machine at one rest point.
///
/// Every field reads the same owned `Mcs4System` the top controls drive, so the
/// register, stack, and memory panels observe one machine rather than a copy.
#[derive(Clone, Debug)]
pub struct MachineSnapshot {
    /// Register file, accumulator, carry, program counter, and stack.
    pub cpu: CpuSnapshot,
    /// Call stack rendered in the stack-panel schema.
    pub stack: StackSnapshot,
    /// ROM chip 0 image.
    pub rom: MemorySnapshot,
    /// RAM bank 0 chip 0 main memory.
    pub ram: MemorySnapshot,
}

/// Immutable observation or fault delivered to the GUI thread.
#[derive(Clone, Debug)]
pub enum SimulationEvent {
    /// Canonical post-phase observation.
    Frame(TraceFrame),
    /// Register, stack, and memory state after a completed rest point.
    Snapshot(MachineSnapshot),
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
                emit_snapshot(session, events);
                true
            }
            Err(error) => send_fault(events, error.to_string()),
        },
        SimulationCommand::StepPhase => {
            emit_phase(session, events);
            emit_snapshot(session, events);
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
            emit_snapshot(session, events);
            let _ = events.send(SimulationEvent::BatchComplete);
            true
        }
        SimulationCommand::LoadRom { bytes } => match session.apply_input(ReplayInput::LoadRom { bytes }) {
            Ok(_) => {
                emit_snapshot(session, events);
                true
            }
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

fn emit_snapshot(session: &ReplaySession<Mcs4System>, events: &Sender<SimulationEvent>) {
    let _ = events.send(SimulationEvent::Snapshot(snapshot_machine(session.target())));
}

/// Read the owned worker machine into the debugger-panel schema.
///
/// The register file, stack, ROM image, and RAM main memory all come from one
/// `Mcs4System`, so a published snapshot describes the same machine the top
/// Run/Step controls advance.
fn snapshot_machine(system: &Mcs4System) -> MachineSnapshot {
    let mut registers = [0u8; 16];
    for (index, slot) in registers.iter_mut().enumerate() {
        *slot = system.register(index as u8);
    }
    let stack = system.stack();
    let stack_pointer = system.stack_pointer();

    let cpu = CpuSnapshot {
        registers: registers.to_vec(),
        accumulator: system.accumulator() & 0x0F,
        carry: system.carry(),
        pc: system.pc() & 0x0FFF,
        stack: stack.to_vec(),
        sp: stack_pointer,
        halted: false,
        interrupt_enabled: false,
    };

    let rom_bytes: Vec<u8> = (0..ROM_VIEW_BYTES)
        .map(|addr| system.read_rom(addr).unwrap_or(0))
        .collect();

    let mut ram_bytes = Vec::with_capacity(usize::from(RAM_VIEW_REGISTERS) * usize::from(RAM_VIEW_CHARACTERS));
    for register in 0..RAM_VIEW_REGISTERS {
        for character in 0..RAM_VIEW_CHARACTERS {
            ram_bytes.push(system.read_ram(0, 0, register, character).unwrap_or(0));
        }
    }

    MachineSnapshot {
        cpu,
        stack: StackSnapshot::new(&stack, stack_pointer),
        rom: MemorySnapshot::from_rom(&rom_bytes),
        ram: MemorySnapshot::from_ram(0, 0, &ram_bytes),
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

    fn wait_for_batch_snapshot(session: &SimulationSession) -> MachineSnapshot {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut latest = None;
        loop {
            for event in session.drain_events() {
                match event {
                    SimulationEvent::Snapshot(snapshot) => latest = Some(snapshot),
                    SimulationEvent::BatchComplete => {
                        if let Some(snapshot) = latest.take() {
                            return snapshot;
                        }
                    }
                    _ => {}
                }
            }
            assert!(
                Instant::now() < deadline,
                "session worker did not complete the run batch"
            );
            thread::yield_now();
        }
    }

    fn wait_for_snapshot(session: &SimulationSession) -> MachineSnapshot {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut latest = None;
        loop {
            for event in session.drain_events() {
                if let SimulationEvent::Snapshot(snapshot) = event {
                    latest = Some(snapshot);
                }
            }
            if let Some(snapshot) = latest.take() {
                return snapshot;
            }
            assert!(Instant::now() < deadline, "session worker did not emit a snapshot");
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

    #[test]
    fn step_publishes_a_snapshot_shaped_for_the_debugger_panels() {
        let session = SimulationSession::spawn();
        session.send(SimulationCommand::StepPhase).expect("send step");
        let snapshot = wait_for_snapshot(&session);

        assert_eq!(snapshot.cpu.registers.len(), 16);
        assert_eq!(snapshot.cpu.stack.len(), 3);
        assert_eq!(snapshot.rom.data.len(), usize::from(ROM_VIEW_BYTES));
        assert_eq!(
            snapshot.ram.data.len(),
            usize::from(RAM_VIEW_REGISTERS) * usize::from(RAM_VIEW_CHARACTERS)
        );
    }

    /// The published snapshot reads the same worker machine the controls drive:
    /// running the validated `src_wrm_rdm` fixture leaves ACC=0xA, R1=0x1, and
    /// RAM register 0 character 1 = 0xA, exactly the program's result.
    #[test]
    fn snapshot_follows_the_running_program_trajectory() {
        const FIXTURE: &str = include_str!("../../mcs4-system/fixtures/src_wrm_rdm.hex");
        let bytes = mcs4_system::parse_hex_bytes(FIXTURE).expect("parse fixture");

        let session = SimulationSession::spawn();
        session.send(SimulationCommand::LoadRom { bytes }).expect("load rom");
        // Seven instructions at up to two machine cycles each; 200 phases clears them.
        session.send(SimulationCommand::RunPhases { phases: 200 }).expect("run");

        // LoadRom also publishes a (fresh) snapshot, so read the one that lands
        // with the run batch, not the first snapshot to arrive.
        let snapshot = wait_for_batch_snapshot(&session);
        assert_eq!(snapshot.cpu.accumulator, 0xA, "RDM reads the written nibble back");
        assert_eq!(snapshot.cpu.registers[1], 0x1, "FIM P0 0x01 loads R1");
        assert_eq!(snapshot.ram.data[1], 0xA, "WRM stored ACC into RAM reg0 char1");
    }
}
