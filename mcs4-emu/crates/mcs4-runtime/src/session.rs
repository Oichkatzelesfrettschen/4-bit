//! Single-owner behavioral session worker for interactive frontends.
//!
//! The worker owns the mutable emulator. A frontend sends bounded commands and
//! receives immutable trace frames, so Run, Step, and Reset never race on a
//! shared `Mcs4System` lock.
//!
//! One machine backs both surfaces. The debugger's Run/Step drive it freely and
//! are authoritative for execution. The Intellec console observes that same
//! machine -- its panel lamps track every phase the debugger produces -- and the
//! source-evidence gate governs only panel-originated stepping.

use std::{
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
};

use mcs4_intellec::{IntellecModel, IntellecPanel, IntellecProfile, PanelInput, PanelSnapshot, ResetScope};
use mcs4_periph::{SevenSegDisplay, Teletype33, TeletypeTiming};
use mcs4_system::{Mcs4System, ReplayInput, ReplaySession, TraceFrame};

use crate::dto::{CpuSnapshot, MemorySnapshot, StackSnapshot};

/// Maximum phases accepted by one UI run request.
pub const MAX_RUN_PHASES: usize = 10_000;

/// ROM chip 0 span published to the memory panel.
const ROM_VIEW_BYTES: u16 = 256;
/// RAM main-memory characters published to the memory panel: 4 registers x 16 characters.
const RAM_VIEW_REGISTERS: u8 = 4;
const RAM_VIEW_CHARACTERS: u8 = 16;

/// Request sent from a frontend thread to the sole behavioral-system owner.
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
    /// Apply one Intellec console panel input to the shared machine's console.
    IntellecPanelInput(PanelInput),
    /// Attempt a panel-originated step, honored only when the evidence gate passes.
    IntellecStep {
        /// Number of bus phases the panel requests.
        phases: u8,
    },
    /// Enqueue one ASR-33 keyboard character.
    IntellecTerminalKey(u8),
    /// Enable or disable ASR-33 punch capture.
    IntellecPunch(bool),
    /// End the worker thread.
    Shutdown,
}

/// A single 7-segment digit driven by the shared machine's RAM output port.
#[derive(Clone, Debug, Default)]
pub struct SevenSegView {
    /// Raw nibble latched at the RAM output port (chip 0).
    pub value: u8,
    /// Lit-segment mask (bit 0 = a .. bit 6 = g, bit 7 = decimal point).
    pub segments: u8,
    /// Decoded character, or a blank/`?` placeholder.
    pub ascii: String,
}

/// Complete debugger view of the single worker machine at one rest point.
///
/// Every field reads the same owned `Mcs4System` the top controls drive, so the
/// register, stack, memory, and peripheral panels observe one machine rather
/// than a copy.
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
    /// 7-segment display attached to the RAM output port of chip 0.
    pub seven_seg: SevenSegView,
}

/// Intellec console view of the same worker machine.
///
/// The panel lamps come from the phases the shared machine produces, so the
/// console observes exactly what the debugger drives. `boot_gate_missing` names
/// the source evidence the profile still requires before panel-originated steps
/// are honored.
#[derive(Clone, Debug)]
pub struct IntellecConsoleSnapshot {
    /// Console switch and lamp state.
    pub panel: PanelSnapshot,
    /// Profile identity the console renders.
    pub model: IntellecModel,
    /// Whether the source-evidence gate permits panel-originated execution.
    pub boot_gate_ok: bool,
    /// Evidence identifiers the profile still requires.
    pub boot_gate_missing: Vec<String>,
    /// Accumulated ASR-33 printer paper.
    pub paper: String,
    /// Count of captured ASR-33 punch bytes.
    pub punch_len: usize,
    /// Result of the most recent panel-originated step attempt, when refused.
    pub panel_step_fault: Option<String>,
}

/// Immutable observation or fault delivered to a frontend thread.
#[derive(Clone, Debug)]
pub enum SimulationEvent {
    /// Canonical post-phase observation.
    Frame(TraceFrame),
    /// Register, stack, and memory state after a completed rest point.
    Snapshot(MachineSnapshot),
    /// Intellec console state after a completed rest point.
    IntellecConsole(IntellecConsoleSnapshot),
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

/// Frontend-facing endpoint for one owned behavioral session.
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
            .spawn(move || Worker::new(event_sender).run(command_receiver))
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

    /// Drain every event already available without blocking the frame.
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

/// The sole owner of the machine, the Intellec console, and the peripherals that
/// observe it.
struct Worker {
    session: ReplaySession<Mcs4System>,
    profile: IntellecProfile,
    panel: IntellecPanel,
    terminal: Teletype33,
    display: SevenSegDisplay,
    paper: String,
    punch_len: usize,
    events: Sender<SimulationEvent>,
}

impl Worker {
    fn new(events: Sender<SimulationEvent>) -> Self {
        let profile = IntellecProfile::intellec4();
        let terminal = Teletype33::new(TeletypeTiming::asr33(profile.phase_ticks_per_second()));
        let mut display = SevenSegDisplay::new(1);
        display.set_bcd_mode(true);
        Self {
            session: ReplaySession::new(),
            profile,
            panel: IntellecPanel::new(),
            terminal,
            display,
            paper: String::new(),
            punch_len: 0,
            events,
        }
    }

    /// Latch the RAM output port of chip 0 into the 7-segment display.
    fn refresh_display(&mut self) {
        let port = self.session.target().read_ram_port(0, 0).unwrap_or(0);
        self.display.set_raw(0, port & 0x0F);
    }

    fn run(mut self, commands: Receiver<SimulationCommand>) {
        while let Ok(command) = commands.recv() {
            if !self.handle(command) {
                return;
            }
        }
    }

    fn handle(&mut self, command: SimulationCommand) -> bool {
        match command {
            SimulationCommand::Shutdown => return false,
            SimulationCommand::Reset => match self.session.apply_input(ReplayInput::Reset) {
                Ok(_) => {
                    let _ = self.events.send(SimulationEvent::RunBoundary {
                        run_id: self.session.run_id(),
                        reason: "reset",
                    });
                    self.refresh_display();
                    self.emit_machine_snapshot();
                    self.emit_console_snapshot(None);
                }
                Err(error) => return self.fault(error.to_string()),
            },
            SimulationCommand::StepPhase => {
                self.step_once();
                self.emit_machine_snapshot();
                self.emit_console_snapshot(None);
            }
            SimulationCommand::RunPhases { phases } => {
                if phases == 0 || phases > MAX_RUN_PHASES {
                    return self.fault(format!("run request must contain 1 through {MAX_RUN_PHASES} phases"));
                }
                for _ in 0..phases {
                    if !self.step_once() {
                        return true;
                    }
                }
                self.emit_machine_snapshot();
                self.emit_console_snapshot(None);
                let _ = self.events.send(SimulationEvent::BatchComplete);
            }
            SimulationCommand::LoadRom { bytes } => match self.session.apply_input(ReplayInput::LoadRom { bytes }) {
                Ok(_) => {
                    self.refresh_display();
                    self.emit_machine_snapshot();
                    self.emit_console_snapshot(None);
                }
                Err(error) => return self.fault(error.to_string()),
            },
            SimulationCommand::SetTestPin { high } => {
                if let Err(error) = self.session.apply_input(ReplayInput::SetTestPin { high }) {
                    return self.fault(error.to_string());
                }
            }
            SimulationCommand::IntellecPanelInput(input) => {
                self.panel.apply(input);
                self.apply_panel_drive();
                self.emit_console_snapshot(None);
            }
            SimulationCommand::IntellecStep { phases } => self.panel_step(phases),
            SimulationCommand::IntellecTerminalKey(byte) => {
                self.terminal.enqueue_keyboard(byte);
                self.emit_console_snapshot(None);
            }
            SimulationCommand::IntellecPunch(enabled) => {
                self.terminal.set_punch_enabled(enabled);
                self.emit_console_snapshot(None);
            }
        }
        true
    }

    /// Advance one phase and let the console observe it. Debugger-authoritative:
    /// this runs without consulting the evidence gate.
    fn step_once(&mut self) -> bool {
        match self.session.step_phase() {
            Ok(frame) => {
                if let Some(phase) = frame.phase.as_ref() {
                    self.panel.observe_phase(phase);
                }
                // The Intellec 4 profile carries no terminal wiring, so the ASR-33
                // advances its own timing and idles until a wired profile drives it.
                let _ = self.terminal.advance_phase_ticks(1, true);
                self.drain_terminal();
                self.refresh_display();
                self.events.send(SimulationEvent::Frame(frame)).is_ok()
            }
            Err(error) => self.fault(error.to_string()),
        }
    }

    /// Honor a panel-originated step only when the source-evidence gate passes
    /// and the console is running. Refusal reports through the console snapshot,
    /// leaving the debugger's execution state untouched.
    fn panel_step(&mut self, phases: u8) {
        if let Err(error) = self.profile.validate_boot_evidence() {
            let missing = error.missing.join(", ");
            self.emit_console_snapshot(Some(format!("source gate blocks panel execution: {missing}")));
            return;
        }
        if !self.panel.runs_machine() {
            self.emit_console_snapshot(Some("panel is stopped".to_owned()));
            return;
        }
        for _ in 0..phases {
            if !self.step_once() {
                return;
            }
            if !self.panel.runs_machine() {
                break;
            }
        }
        self.emit_machine_snapshot();
        self.emit_console_snapshot(None);
    }

    /// Translate latched panel line drives into replay-logged machine inputs.
    fn apply_panel_drive(&mut self) {
        let drive = self.panel.take_drive();
        if let Some(scope) = drive.reset {
            let _ = self.session.apply_input(ReplayInput::Reset);
            if scope == ResetScope::System {
                self.terminal.reset();
                self.paper.clear();
                self.punch_len = 0;
            }
            let _ = self.events.send(SimulationEvent::RunBoundary {
                run_id: self.session.run_id(),
                reason: "reset",
            });
            self.emit_machine_snapshot();
        }
        if let Some(hold) = drive.test_hold {
            let _ = self.session.apply_input(ReplayInput::SetTestPin { high: hold });
        }
        if drive.test_one_shot {
            let _ = self.session.apply_input(ReplayInput::SetTestPin { high: true });
        }
        // Console-memory access needs a modeled memory-control card; the drive is
        // latched but unmodeled, so no machine memory changes here.
    }

    fn drain_terminal(&mut self) {
        let printed = self.terminal.drain_printed();
        if !printed.is_empty() {
            self.paper.push_str(&String::from_utf8_lossy(&printed));
        }
        self.punch_len += self.terminal.drain_punched().len();
    }

    fn emit_machine_snapshot(&self) {
        let mut snapshot = snapshot_machine(self.session.target());
        snapshot.seven_seg = SevenSegView {
            value: self.display.raw(0) & 0x0F,
            segments: self.display.segments(0),
            ascii: self.display.render_ascii(),
        };
        let _ = self.events.send(SimulationEvent::Snapshot(snapshot));
    }

    fn emit_console_snapshot(&self, panel_step_fault: Option<String>) {
        let (boot_gate_ok, boot_gate_missing) = match self.profile.validate_boot_evidence() {
            Ok(()) => (true, Vec::new()),
            Err(error) => (false, error.missing.iter().map(|entry| (*entry).to_owned()).collect()),
        };
        let _ = self
            .events
            .send(SimulationEvent::IntellecConsole(IntellecConsoleSnapshot {
                panel: self.panel.snapshot(),
                model: self.profile.model(),
                boot_gate_ok,
                boot_gate_missing,
                paper: self.paper.clone(),
                punch_len: self.punch_len,
                panel_step_fault,
            }));
    }

    fn fault(&self, message: String) -> bool {
        let _ = self.events.send(SimulationEvent::Fault { message });
        true
    }
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
        seven_seg: SevenSegView::default(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use mcs4_intellec::{PanelControl, PanelInput};

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

    /// Collect the machine and console snapshots that a single command publishes.
    fn wait_for_machine_and_console(session: &SimulationSession) -> (MachineSnapshot, IntellecConsoleSnapshot) {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut machine = None;
        let mut console = None;
        loop {
            for event in session.drain_events() {
                match event {
                    SimulationEvent::Snapshot(snapshot) => machine = Some(snapshot),
                    SimulationEvent::IntellecConsole(snapshot) => console = Some(snapshot),
                    _ => {}
                }
            }
            if let (Some(machine), Some(console)) = (machine.as_ref(), console.as_ref()) {
                return (machine.clone(), console.clone());
            }
            assert!(Instant::now() < deadline, "session worker did not emit both snapshots");
            thread::yield_now();
        }
    }

    fn wait_for_console_fault(session: &SimulationSession) -> String {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            for event in session.drain_events() {
                if let SimulationEvent::IntellecConsole(snapshot) = event {
                    if let Some(fault) = snapshot.panel_step_fault {
                        return fault;
                    }
                }
            }
            assert!(
                Instant::now() < deadline,
                "session worker did not refuse the panel step"
            );
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
        let bytes = fixture_bytes();

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

    /// One machine backs both surfaces: after a debugger step the Intellec console
    /// lamps show the same program counter; a panel-originated step is refused
    /// under the Intellec 4 evidence gate while the debugger still drives the
    /// shared machine to the program's result.
    #[test]
    fn one_machine_backs_debugger_and_intellec_console() {
        let session = SimulationSession::spawn();
        session
            .send(SimulationCommand::LoadRom { bytes: fixture_bytes() })
            .expect("load rom");
        session.send(SimulationCommand::StepPhase).expect("debugger step");

        let (machine, console) = wait_for_machine_and_console(&session);
        assert_eq!(
            console.panel.lamps.address, machine.cpu.pc,
            "console lamps observe the machine the debugger stepped"
        );
        assert!(
            !console.boot_gate_ok,
            "Intellec 4 profile keeps its evidence gate closed"
        );

        // A panel-originated step is refused by the closed evidence gate.
        session
            .send(SimulationCommand::IntellecPanelInput(PanelInput::Control(
                PanelControl::Run,
            )))
            .expect("panel run");
        session
            .send(SimulationCommand::IntellecStep { phases: 1 })
            .expect("panel step");
        let fault = wait_for_console_fault(&session);
        assert!(fault.contains("source gate"), "panel step refused: {fault}");

        // The debugger still drives the shared machine freely to completion.
        session.send(SimulationCommand::RunPhases { phases: 200 }).expect("run");
        let snapshot = wait_for_batch_snapshot(&session);
        assert_eq!(snapshot.cpu.accumulator, 0xA, "debugger drove the shared machine");
    }

    /// A peripheral is driven by the shared machine: the 7-segment counter
    /// fixture writes an incrementing nibble to the RAM output port via WMP, so
    /// the display value the worker publishes advances as the machine runs.
    #[test]
    fn seven_segment_display_follows_the_machine_output_port() {
        const FIXTURE: &str = include_str!("../../mcs4-system/fixtures/seven_seg_count.hex");
        let bytes = mcs4_system::parse_hex_bytes(FIXTURE).expect("parse fixture");

        let session = SimulationSession::spawn();
        session.send(SimulationCommand::LoadRom { bytes }).expect("load rom");
        session
            .send(SimulationCommand::RunPhases { phases: 60 })
            .expect("run 1");
        let first = wait_for_batch_snapshot(&session).seven_seg.value;
        session
            .send(SimulationCommand::RunPhases { phases: 240 })
            .expect("run 2");
        let second = wait_for_batch_snapshot(&session).seven_seg.value;

        assert_ne!(
            first, second,
            "display value advanced with the counter (0x{first:X} -> 0x{second:X})"
        );
    }

    fn fixture_bytes() -> Vec<u8> {
        const FIXTURE: &str = include_str!("../../mcs4-system/fixtures/src_wrm_rdm.hex");
        mcs4_system::parse_hex_bytes(FIXTURE).expect("parse fixture")
    }
}
