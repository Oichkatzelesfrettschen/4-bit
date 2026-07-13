//! Intellec console state and source-backed control semantics.

use mcs4_system::PhaseTrace;

/// Program-memory bank selected by the console.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgramMemoryMode {
    /// Resident monitor memory.
    Monitor,
    /// Writable program RAM.
    Ram,
    /// Optional program PROM.
    Prom,
}

/// Reset target selected by the console mode control.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResetScope {
    /// Reset the processor only.
    Cpu,
    /// Reset the complete development system.
    System,
}

/// Named momentary or maintained console controls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PanelControl {
    /// Run continuously.
    Run,
    /// Halt phase advancement.
    Stop,
    /// Advance exactly one eight-phase machine cycle.
    SingleStep,
    /// Request reset through the selected reset scope.
    Reset(ResetScope),
    /// Maintain the processor TEST line.
    TestHold(bool),
    /// Pulse the processor TEST line for one phase.
    TestOneShot,
    /// Enable or disable console memory access.
    ConsoleMemoryAccess(bool),
    /// Request a console-memory write through the backplane.
    ConsoleMemoryWrite,
}

/// Every UI-originated console action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PanelInput {
    /// Change the 12-bit address/data switch bank.
    SetAddressData(u16),
    /// Change the eight writable data switch bits.
    SetWriteData(u8),
    /// Select a program-memory mode.
    SetProgramMemoryMode(ProgramMemoryMode),
    /// Change the four-bit search pass counter.
    SetPassCounter(u8),
    /// Apply a named physical control action.
    Control(PanelControl),
}

/// One console action that requires a backplane implementation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsoleMemoryAccess {
    /// Read the selected program-memory byte.
    Read {
        /// Program-memory address selected by the console switches.
        address: u16,
    },
    /// Write the selected program-memory byte.
    Write {
        /// Program-memory address selected by the console switches.
        address: u16,
        /// Program-memory byte selected by the write-data switches.
        data: u8,
    },
}

/// External line drives produced by a completed panel input batch.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PanelDrive {
    /// Requested reset line, if any.
    pub reset: Option<ResetScope>,
    /// Maintained TEST level.
    pub test_hold: Option<bool>,
    /// One-phase TEST pulse.
    pub test_one_shot: bool,
    /// Console-memory transaction that the backplane must execute.
    pub console_memory_access: Option<ConsoleMemoryAccess>,
}

/// Source-visible console lamps sampled after one completed machine phase.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PanelLamps {
    /// Current program address observation.
    pub address: u16,
    /// M1/M2 instruction observation.
    pub instruction: u8,
    /// X2/X3 execution-bus observation.
    pub execution: u8,
    /// Most recently valid SRC pointer.
    pub last_ram_rom_pointer: Option<u8>,
    /// Active RAM-bank selection.
    pub active_ram_bank: Option<u8>,
    /// Active program-memory selection.
    pub active_rom_bank: Option<u8>,
    /// Processor execution state.
    pub cpu_running: bool,
}

/// Immutable panel state rendered by front ends and captured in traces.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PanelSnapshot {
    /// Address/data switch position.
    pub address_data_switches: u16,
    /// Write-data switch position.
    pub write_data_switches: u8,
    /// Selected program-memory mode.
    pub program_memory_mode: ProgramMemoryMode,
    /// Search pass-counter switch position.
    pub pass_counter: u8,
    /// Console-memory-access enable state.
    pub console_memory_access_enabled: bool,
    /// Latched source-visible lamps.
    pub lamps: PanelLamps,
}

/// Console model that produces line drives but never mutates machine memory.
#[derive(Clone, Debug)]
pub struct IntellecPanel {
    address_data_switches: u16,
    write_data_switches: u8,
    program_memory_mode: ProgramMemoryMode,
    pass_counter: u8,
    console_memory_access_enabled: bool,
    running: bool,
    remaining_step_phases: Option<u8>,
    pending_drive: PanelDrive,
    lamps: PanelLamps,
}

impl IntellecPanel {
    /// Construct a stopped console with monitor memory selected.
    pub const fn new() -> Self {
        Self {
            address_data_switches: 0,
            write_data_switches: 0,
            program_memory_mode: ProgramMemoryMode::Monitor,
            pass_counter: 0,
            console_memory_access_enabled: false,
            running: false,
            remaining_step_phases: None,
            pending_drive: PanelDrive {
                reset: None,
                test_hold: None,
                test_one_shot: false,
                console_memory_access: None,
            },
            lamps: PanelLamps {
                address: 0,
                instruction: 0,
                execution: 0,
                last_ram_rom_pointer: None,
                active_ram_bank: None,
                active_rom_bank: None,
                cpu_running: false,
            },
        }
    }

    /// Apply one physical panel input.
    pub fn apply(&mut self, input: PanelInput) {
        match input {
            PanelInput::SetAddressData(value) => self.address_data_switches = value & 0x0fff,
            PanelInput::SetWriteData(value) => self.write_data_switches = value,
            PanelInput::SetProgramMemoryMode(value) => self.program_memory_mode = value,
            PanelInput::SetPassCounter(value) => self.pass_counter = value & 0x0f,
            PanelInput::Control(PanelControl::Run) => {
                self.running = true;
                self.remaining_step_phases = None;
            }
            PanelInput::Control(PanelControl::Stop) => {
                self.running = false;
                self.remaining_step_phases = None;
            }
            PanelInput::Control(PanelControl::SingleStep) => {
                self.running = true;
                self.remaining_step_phases = Some(8);
            }
            PanelInput::Control(PanelControl::Reset(scope)) => self.pending_drive.reset = Some(scope),
            PanelInput::Control(PanelControl::TestHold(value)) => self.pending_drive.test_hold = Some(value),
            PanelInput::Control(PanelControl::TestOneShot) => self.pending_drive.test_one_shot = true,
            PanelInput::Control(PanelControl::ConsoleMemoryAccess(value)) => {
                self.console_memory_access_enabled = value;
            }
            PanelInput::Control(PanelControl::ConsoleMemoryWrite) => {
                if self.console_memory_access_enabled {
                    self.pending_drive.console_memory_access = Some(ConsoleMemoryAccess::Write {
                        address: self.address_data_switches,
                        data: self.write_data_switches,
                    });
                }
            }
        }
    }

    /// Return whether the machine may execute its next phase.
    pub const fn runs_machine(&self) -> bool {
        self.running
    }

    /// Take pending line drives exactly once.
    pub fn take_drive(&mut self) -> PanelDrive {
        let drive = self.pending_drive;
        self.pending_drive = PanelDrive::default();
        drive
    }

    /// Latch one completed machine-phase observation.
    pub fn observe_phase(&mut self, phase: &PhaseTrace) {
        self.lamps.address = phase.pc & 0x0fff;
        self.lamps.execution = phase.bus_value & 0x0f;
        self.lamps.active_rom_bank = phase.selected_rom;
        self.lamps.active_ram_bank = phase.selected_ram;
        self.lamps.cpu_running = self.running;
        if phase.io_op.as_deref() == Some("src") {
            self.lamps.last_ram_rom_pointer = Some(phase.bus_value & 0x0f);
        }

        if let Some(remaining) = self.remaining_step_phases {
            if remaining == 1 {
                self.remaining_step_phases = None;
                self.running = false;
                self.lamps.cpu_running = false;
            } else {
                self.remaining_step_phases = Some(remaining - 1);
            }
        }
    }

    /// Return an immutable rendered state.
    pub const fn snapshot(&self) -> PanelSnapshot {
        PanelSnapshot {
            address_data_switches: self.address_data_switches,
            write_data_switches: self.write_data_switches,
            program_memory_mode: self.program_memory_mode,
            pass_counter: self.pass_counter,
            console_memory_access_enabled: self.console_memory_access_enabled,
            lamps: self.lamps,
        }
    }

    /// Reset panel controls and latches.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for IntellecPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{ConsoleMemoryAccess, IntellecPanel, PanelControl, PanelInput, ProgramMemoryMode, ResetScope};

    #[test]
    fn console_memory_write_creates_a_backplane_request() {
        let mut panel = IntellecPanel::new();
        panel.apply(PanelInput::SetAddressData(0x2ab));
        panel.apply(PanelInput::SetWriteData(0x5c));
        panel.apply(PanelInput::Control(PanelControl::ConsoleMemoryAccess(true)));
        panel.apply(PanelInput::Control(PanelControl::ConsoleMemoryWrite));
        assert_eq!(
            panel.take_drive().console_memory_access,
            Some(ConsoleMemoryAccess::Write {
                address: 0x2ab,
                data: 0x5c,
            })
        );
    }

    #[test]
    fn single_step_consumes_eight_phases() {
        let mut panel = IntellecPanel::new();
        panel.apply(PanelInput::Control(PanelControl::SingleStep));
        assert!(panel.runs_machine());
        for _ in 0..8 {
            panel.observe_phase(&mcs4_system::PhaseTrace {
                schema_version: 1,
                architecture: mcs4_system::SystemArchitecture::Mcs4,
                completed_phase: mcs4_system::TracePhase::A1,
                next_phase: mcs4_system::TracePhase::A2,
                machine_cycles: 0,
                instruction_count: 0,
                pc: 0,
                accumulator: 0,
                carry: false,
                bus_value: 0,
                bus_valid: true,
                bus_contention: false,
                selected_rom: None,
                selected_ram: None,
                io_op: None,
            });
        }
        assert!(!panel.runs_machine());
    }

    #[test]
    fn reset_and_memory_mode_remain_explicit() {
        let mut panel = IntellecPanel::new();
        panel.apply(PanelInput::SetProgramMemoryMode(ProgramMemoryMode::Prom));
        panel.apply(PanelInput::Control(PanelControl::Reset(ResetScope::System)));
        assert_eq!(panel.snapshot().program_memory_mode, ProgramMemoryMode::Prom);
        assert_eq!(panel.take_drive().reset, Some(ResetScope::System));
    }
}
