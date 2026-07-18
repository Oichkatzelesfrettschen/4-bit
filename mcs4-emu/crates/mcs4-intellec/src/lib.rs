//! Intellec-4 Development System Emulation
//!
//! The Intellec 4 family is Intel's MCS development-system line. This crate
//! exposes two distinct surfaces:
//!
//! - [`machine`], [`console`], [`profile`], and [`replay`] form the
//!   source-gated implementation. Historical execution rejects incomplete
//!   firmware, console-net, or terminal-port evidence.
//! - [`front_panel`], [`monitor`], and [`system`] preserve a legacy host-side
//!   fixture for compatibility tests. They do not establish historical board
//!   behavior and never provide evidence for a source-gated profile.

pub mod console;
pub mod front_panel;
pub mod imm6_28;
pub mod machine;
pub mod mod40;
pub mod mod40_routes;
pub mod monitor;
pub mod monitor_rom;
pub mod profile;
pub mod prom_programmer;
pub mod replay;
pub mod system;

pub use console::{
    ConsoleMemoryAccess, IntellecPanel, PanelControl, PanelDrive, PanelInput, PanelLamps, PanelSnapshot,
    ProgramMemoryMode, ResetScope,
};
pub use front_panel::{FrontPanel, PanelLeds, PanelMode};
pub use imm6_28::{Imm628, Imm628ChipLocation, Imm628Output, Imm628Read};
pub use machine::{IntellecBusMachine, IntellecEvent, IntellecFrame, IntellecMachine, IntellecMachineError};
pub use mod40::{
    Imm443, Imm472, Mod40AssemblyError, Mod40Board, Mod40SourceGate, Mod40TerminalEndpoint, ProgramStoreId,
};
pub use mod40_routes::{
    cpu_clock_source_is_traced, cpu_nominal_machine_clock_hz, imm628_has_exactly_one_selected_byte,
    imm628_write_read_level_requests_write, keyboard_loop_current_to_rom0_input_bit0, monitor_address_fanout_is_traced,
    monitor_data_polarity_is_traced, monitor_select_decode_inputs_are_traced,
    monitor_select_decode_outputs_are_recorded, program_ram_card_edge_is_complete,
    ram0_port_value_drives_printer_marking_current, ram1_port_value_enables_reader, terminal_cable_routes_are_traced,
    terminal_current_loop_polarity_is_traced, ControlResetInitiator, ControlResetPulseContract, CpuClockResetRoute,
    CpuClockTimingTarget, CpuStopAcknowledgeEndpoint, Imm628LocalWritePath, Imm628WriteReadInput, Mod40RouteEvidence,
    Mod40TerminalPolarity, MonitorAddressFanout, MonitorSelectDecodeInput, MonitorSelectDecodeOutput,
    PanelControlObservation, PanelSingleStepContract, ProgramRamCardEdgeRoute, StopAcknowledgeObservation,
    TerminalCableRoute, TerminalPortPolarity, CONTROL_RESET_PULSE_CONTRACT, CPU_CLOCK_RESET_ROUTES,
    CPU_CLOCK_TIMING_TARGET, CPU_STOP_ACKNOWLEDGE_ENDPOINT, IMM628_LOCAL_WRITE_PATH, IMM628_WRITE_READ_INPUT,
    MONITOR_ADDRESS_FANOUT, MONITOR_SELECT_DECODE_INPUTS, MONITOR_SELECT_DECODE_OUTPUTS, PANEL_CONTROL_OBSERVATIONS,
    PANEL_SINGLE_STEP_CONTRACT, PROGRAM_RAM_CARD_EDGE_ROUTES, STOP_ACKNOWLEDGE_OBSERVATIONS, TERMINAL_CABLE_ROUTES,
    TERMINAL_PORT_POLARITIES,
};
pub use monitor::{Monitor, MonitorAction, MonitorCommand};
pub use monitor_rom::{MonitorRom, MonitorRomError};
pub use profile::{
    CardKind, CardPlacement, CardSlot, IntellecModel, IntellecProfile, ProfileEvidenceError, RamPortEndpoint,
    SourceReference, TerminalWiring,
};
pub use prom_programmer::{ProgResult, PromProgrammer};
pub use replay::{
    IntellecReplayCheckpoint, IntellecReplayCommand, IntellecReplayError, IntellecReplayLog, IntellecReplaySession,
    IntellecReplayTarget, INTELLEC_REPLAY_SCHEMA_VERSION,
};
/// Legacy host-side fixture. Use [`IntellecMachine`] for source-gated work.
pub use system::IntellecSystem;
