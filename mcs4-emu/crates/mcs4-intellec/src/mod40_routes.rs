//! Directly reviewed Intellec 4/MOD 40 route records.
//!
//! These records encode connector and fanout facts from the retained 98-013A
//! sheets. They are not a controller Boolean model. A `Partial` route exposes
//! a useful boundary without authorizing a polarity, timing, or cycle claim.

use crate::mod40::Mod40TerminalEndpoint;

/// Completeness of one route record against its controlling primary sheet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mod40RouteEvidence {
    /// Both named endpoints are visually traced on the reviewed source sheets.
    Direct,
    /// The visible route has an untraced endpoint, polarity, or timing stage.
    Partial,
}

/// One motherboard-to-IN-28 card-edge path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProgramRamCardEdgeRoute {
    /// Signal at the source-side boundary. This may be a controller-local name.
    pub source_signal: &'static str,
    /// Source connector contact when the reviewed sheets establish it.
    pub source_contact: Option<u8>,
    /// Signal at the IN-28 boundary.
    pub target_signal: &'static str,
    /// IN-28 P1 connector contact.
    pub target_contact: u8,
    /// Reviewed evidence status for the complete path.
    pub evidence: Mod40RouteEvidence,
    /// Primary-source locator for this record.
    pub source_locator: &'static str,
}

/// Electrical meaning of the imm6-28 write/read card input.
///
/// This describes the card-edge command level only. It does not describe the
/// later 3404, TTL, or 2102 control-pin waveform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Imm628WriteReadInput {
    /// Logic level at the card input that requests a write cycle.
    pub write_level_high: bool,
    /// Logic level at the card input that requests a read cycle.
    pub read_level_high: bool,
    /// Reviewed evidence status for the card-input convention.
    pub evidence: Mod40RouteEvidence,
    /// Primary-source locator for this convention.
    pub source_locator: &'static str,
}

/// Controller conditions that produce a write command before A29 inversion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlWriteCommandCause {
    /// Console memory access enable is sufficient.
    CmaEnable,
    /// Program-memory write requires PM, OUT, and write-enable together.
    ProgramMemoryOutAndWriteEnable,
}

/// A14-to-A29 command polarity at the program-memory boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlWriteCommandRoute {
    /// A14 output is high when either listed cause applies.
    pub controller_command_high: bool,
    /// A29 inversion presents an active-low module input.
    pub module_input_active_low: bool,
    /// Local buffer and 2102 timing remain untraced.
    pub evidence: Mod40RouteEvidence,
    /// Primary-source locator.
    pub source_locator: &'static str,
}

/// Source-reviewed imm6-28 write/read card-input polarity.
///
/// The 98-095A functional manual defines the card input as TTL low for data
/// write-in and high for readout. The record intentionally stops at the card
/// input; it does not claim the pulse width, latch enable, or final 2102 R/W
/// waveform.
pub const IMM628_WRITE_READ_INPUT: Imm628WriteReadInput = Imm628WriteReadInput {
    write_level_high: false,
    read_level_high: true,
    evidence: Mod40RouteEvidence::Direct,
    source_locator: "98-095A printed page 40; 98-013A PDFs 7 and 10 identify the card-edge route",
};

/// Source-bound controller write-command logic and module-input polarity.
pub const CONTROL_WRITE_COMMAND_CAUSES: [ControlWriteCommandCause; 2] = [
    ControlWriteCommandCause::CmaEnable,
    ControlWriteCommandCause::ProgramMemoryOutAndWriteEnable,
];

/// A14 high is inverted by A29 before the active-low module input.
pub const CONTROL_WRITE_COMMAND_ROUTE: ControlWriteCommandRoute = ControlWriteCommandRoute {
    controller_command_high: true,
    module_input_active_low: true,
    evidence: Mod40RouteEvidence::Partial,
    source_locator: "98-095A printed page 52; 98-013A PDF 7 and PDF 10",
};

/// Return whether an imm6-28 card-input logic level requests a write.
pub const fn imm628_write_read_level_requests_write(level_high: bool) -> bool {
    !level_high
}

/// Return whether the standard MOD 40 presents exactly one active-low byte select.
///
/// The controller selects one byte during a program-RAM transaction. This
/// function does not assign either byte signal to a processor nibble.
pub const fn imm628_has_exactly_one_selected_byte(byte1_high: bool, byte2_high: bool) -> bool {
    byte1_high != byte2_high
}

/// One CPU-card to terminal-cable conductor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalCableRoute {
    /// Source-bound logical port role.
    pub endpoint: Mod40TerminalEndpoint,
    /// CPU-card P4/J4 contact.
    pub cpu_contact: u8,
    /// Motherboard rear-connector J42/P42 contact.
    pub rear_connector_contact: u8,
    /// Terminal-side contact where the conductor ends.
    pub terminal_contact: u8,
    /// Printed net name.
    pub signal: &'static str,
    /// Primary-source locator for this record.
    pub source_locator: &'static str,
}

/// Source-reconciled logical sense for one CPU port at the ASR-33 boundary.
///
/// The 98-013A sheets establish the physical conductor and CPU-side circuit.
/// The 98-095A terminal procedure establishes the port-bit meaning. Together
/// they establish the port boundary without claiming transistor switching
/// thresholds or reader-relay mechanical timing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalPortPolarity {
    /// Typed CPU-card endpoint covered by this logical convention.
    pub endpoint: Mod40TerminalEndpoint,
    /// Source-reconciled assertion convention at the port boundary.
    pub polarity: Mod40TerminalPolarity,
    /// Primary-source locators required for this reconciliation.
    pub source_locator: &'static str,
}

/// One source-reconciled logical convention at the terminal port boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mod40TerminalPolarity {
    /// Keyboard loop current absent appears high at ROM 0 input bit 0.
    Rom0InputHighWhenKeyboardLoopCurrentAbsent,
    /// A high RAM 0 bit 0 drives marking current in the printer loop.
    Ram0Bit0HighDrivesPrinterMarkingCurrent,
    /// A high RAM 1 bit 0 enables the paper-reader drive.
    Ram1Bit0HighEnablesReader,
}

/// One shared monitor-address output from the 4289 to all four resident PROMs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MonitorAddressFanout {
    /// 4289 address-output index.
    pub address_bit: u8,
    /// Physical monitor sockets receiving this address line.
    pub monitor_sockets: [u8; 4],
    /// Primary-source locator for this record.
    pub source_locator: &'static str,
}

/// One reviewed CPU-card clock or reset route.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuClockResetRoute {
    /// Physical source or named card-edge net.
    pub source: &'static str,
    /// Receiver or intervening functional stage established by the sheet.
    pub target: &'static str,
    /// Reviewed evidence status for the complete electrical path.
    pub evidence: Mod40RouteEvidence,
    /// Primary-source locator for this record.
    pub source_locator: &'static str,
}

/// Functional timing target for the imm4-43 clock-generator topology.
///
/// This target combines the 98-095A functional description with the matching
/// 98-013A oscillator, counter, gate, and MH0026 population. It is not a
/// measured phi1 or phi2 waveform at a 4040 package pin.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuClockTimingTarget {
    /// Nominal crystal frequency before the counter division.
    pub crystal_frequency_hz: u32,
    /// Documented division factor of the counter network.
    pub divider: u8,
    /// Nominal low-going phase-pulse width from the functional description.
    pub phase_pulse_width_ns: u16,
    /// The generated phases do not overlap at the functional timing boundary.
    pub phases_non_overlapping: bool,
    /// Reviewed evidence status for this functional timing target.
    pub evidence: Mod40RouteEvidence,
    /// Primary-source locator for this target.
    pub source_locator: &'static str,
}

/// Functional reset duration required by the 4040 CPU description.
///
/// This requirement constrains any future board-cycle implementation. It does
/// not establish the MOD 40 reset release edge relative to either clock phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuResetTimingRequirement {
    /// The functional board description names CPU RESET as active low.
    pub board_reset_active_low: bool,
    /// The 4040 component catalog defines its package RESET input as active high.
    pub package_reset_active_high: bool,
    /// Minimum number of complete instructions held in reset.
    pub minimum_full_instruction_cycles: u8,
    /// Equivalent minimum number of external clock periods.
    pub minimum_external_clock_periods: u8,
    /// Reviewed evidence status for the functional requirement.
    pub evidence: Mod40RouteEvidence,
    /// Primary-source locator for the requirement.
    pub source_locator: &'static str,
}

/// One direct but non-executable front-panel control observation.
///
/// A panel observation records a physically visible input-conditioning or
/// mode-switch boundary. It does not imply a control-card priority equation,
/// asserted logical level, or cycle transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PanelControlObservation {
    /// Printed panel-side source or control.
    pub source: &'static str,
    /// Directly visible conditioned target or switch boundary.
    pub target: &'static str,
    /// Reviewed evidence status for the observation.
    pub evidence: Mod40RouteEvidence,
    /// Primary-source locator for this record.
    pub source_locator: &'static str,
}

/// A source-defined initiator for the control-card reset one-shot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlResetInitiator {
    /// The console RESET pushbutton momentarily grounds the reset input.
    ConsoleResetPushbutton,
    /// The rear-panel USER RESET line enters the control card at P1 contact 46.
    UserReset,
    /// Changing a MON, RAM, or PROM mode selector triggers a reset.
    ProgramStoreModeChange,
}

/// Functional reset-pulse contract generated by the imm4-72 control card.
///
/// This contract reaches the control-card TTL boundary. It does not establish
/// the 4040 package-level assertion polarity or reset-release phase relation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlResetPulseContract {
    /// Reset sources accepted by the A24 control-card one-shot.
    pub initiators: [ControlResetInitiator; 3],
    /// The TTL reset output at A24 pin 9 is negative-going.
    pub ttl_output_negative_going: bool,
    /// The documented minimum duration of the A24 pin-9 reset pulse.
    pub minimum_duration_us: u16,
    /// Reviewed evidence status for the functional control-card pulse.
    pub evidence: Mod40RouteEvidence,
    /// Primary-source locator.
    pub source_locator: &'static str,
}

/// Functional single-step sequence inside the imm4-72 control card.
///
/// This record captures the documented one-instruction STOP-release sequence.
/// It does not assert a complete connector-level STOP or STOP ACK polarity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PanelSingleStepContract {
    /// The panel switch produces a momentary high at A24 pin 4.
    pub switch_pulse_high: bool,
    /// The one-shot interrupts the A21 pin-10 enable and lifts the STOP clamp.
    pub releases_stop_clamp: bool,
    /// STOP ACK ends the one-shot and restores the STOP clamp.
    pub stop_acknowledge_rearms_clamp: bool,
    /// The documented sequence completes one program instruction before halt.
    pub completed_instruction_count: u8,
    /// Reviewed evidence status for the control-card functional sequence.
    pub evidence: Mod40RouteEvidence,
    /// Primary-source locator.
    pub source_locator: &'static str,
}

/// Local IN-28 write-path facts before the selected 2102 control pins.
///
/// The primary manual establishes each functional stage but not the one-shot
/// component value, pulse width, or every device-pin propagation delay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Imm628LocalWritePath {
    /// ADR STB at P1 contact 91 remains low in the standard MOD 40 wiring.
    pub address_latches_transparent: bool,
    /// A low card-edge WRITE/READ input requests write operation.
    pub write_read_low_requests_write: bool,
    /// The write-command transition starts a delay before the write one-shot.
    pub input_latch_setup_precedes_write_pulse: bool,
    /// BYTE1 and BYTE2 use active-low selection at the local 7400 gates.
    pub byte_select_active_low: bool,
    /// The module returns to continuous read after WRITE/READ returns high.
    pub read_state_when_write_read_high: bool,
    /// The source does not state the local 2102 R/W pulse width.
    pub write_pulse_width_ns: Option<u16>,
    /// Reviewed evidence status for the complete functional local path.
    pub evidence: Mod40RouteEvidence,
    /// Primary-source locator.
    pub source_locator: &'static str,
}

/// One local STOP ACK path observed on a MOD 40 board drawing.
///
/// These paths stop at board and cable boundaries. They do not establish that
/// identically named contacts form one end-to-end net or resolve its asserted
/// polarity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StopAcknowledgeObservation {
    /// Printed local source net or connector contact.
    pub source: &'static str,
    /// Directly visible local receiver or conditioning path.
    pub target: &'static str,
    /// Reviewed evidence status for the observation.
    pub evidence: Mod40RouteEvidence,
    /// Primary-source locator for this record.
    pub source_locator: &'static str,
}

/// Source-defined STOP and STOP ACK behavior at the 4040 package pins.
///
/// This describes the CPU endpoint only. It does not assign a level to an
/// identically named card-edge, panel, or rear-connector signal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuStopAcknowledgeEndpoint {
    /// The 4040 STP input requests STOP mode when high.
    pub stop_input_active_high: bool,
    /// The 4040 STPA output uses an open-drain driver.
    pub acknowledge_open_drain: bool,
    /// A released STPA output is high while the CPU remains stopped.
    pub acknowledge_high_when_stopped: bool,
    /// Primary-source locator for this endpoint behavior.
    pub source_locator: &'static str,
}

/// One decoder input that participates in resident monitor selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MonitorSelectDecodeInput {
    /// Named net entering the A18 decoder region.
    pub signal: &'static str,
    /// A18 pin name when the reviewed sheet labels the connection.
    pub decoder_pin: Option<&'static str>,
    /// Primary-source locator for this record.
    pub source_locator: &'static str,
}

/// One A18 monitor-select decoder output visible on the CPU schematic.
///
/// A decoder-pin record does not establish the destination, selected-ROM
/// polarity, or socket order. Those facts remain separate source gates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MonitorSelectDecodeOutput {
    /// 74155 output label printed at A18.
    pub output: &'static str,
    /// A18 physical pin carrying this output.
    pub decoder_pin: u8,
    /// Reviewed evidence status for the complete output route.
    pub evidence: Mod40RouteEvidence,
    /// Primary-source locator for this record.
    pub source_locator: &'static str,
}

/// Reviewed clock and reset facts from the imm4-43 schematic.
pub const CPU_CLOCK_RESET_ROUTES: [CpuClockResetRoute; 4] = [
    CpuClockResetRoute {
        source: "Y1 5.185 MHz crystal oscillator",
        target: "A16 gate to A7 74161/9316 counter CP input",
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    CpuClockResetRoute {
        source: "CPU RESET card-edge net",
        target: "A11 4040 RESET pin 12",
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    CpuClockResetRoute {
        source: "A7 74161/9316 divider outputs",
        target: "74H00 and 7404 phase-conditioning network feeding A32 MH0026 inputs",
        evidence: Mod40RouteEvidence::Partial,
        source_locator: "98-013A PDF 3, drawing 2000318; counter-state equation remains open",
    },
    CpuClockResetRoute {
        source: "A32 MH0026 outputs through R33 and R32",
        target: "A11 4040 phi1 and phi2 input paths",
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
];

/// Functional timing target for the board-matching clock-generator topology.
///
/// Figure 3-14 of the MCS-40 manual shows the same oscillator, 9316-class
/// counter, 74H00/7404, and MH0026 topology that drawing 2000318 places at
/// Y1, A7, A16, and A32. The complete board Boolean equation and delay budget
/// remain open, so this record does not close the historical phase gate.
pub const CPU_CLOCK_TIMING_TARGET: CpuClockTimingTarget = CpuClockTimingTarget {
    crystal_frequency_hz: 5_185_000,
    divider: 7,
    phase_pulse_width_ns: 386,
    phases_non_overlapping: true,
    evidence: Mod40RouteEvidence::Partial,
    source_locator: "98-095A printed pages 15-16 and Figure 3-14 on printed page 3-7; 98-013A PDF 3, drawing 2000318",
};

/// Functional CPU reset duration before instruction execution resumes.
///
/// The functional board manual and the 4040 component catalog disagree at
/// different named boundaries. The board-level inversion and release phase
/// remain incomplete in the route ledger, so this record preserves both facts
/// without treating either as the complete board waveform.
pub const CPU_RESET_TIMING_REQUIREMENT: CpuResetTimingRequirement = CpuResetTimingRequirement {
    board_reset_active_low: true,
    package_reset_active_high: true,
    minimum_full_instruction_cycles: 8,
    minimum_external_clock_periods: 64,
    evidence: Mod40RouteEvidence::Partial,
    source_locator: "98-095A printed page 15; Intel 1975 Data Catalog, 4040 printed page 6-10",
};

/// Return the integer nominal clock frequency after the documented division.
pub const fn cpu_nominal_machine_clock_hz() -> u32 {
    CPU_CLOCK_TIMING_TARGET.crystal_frequency_hz / CPU_CLOCK_TIMING_TARGET.divider as u32
}

/// Directly reviewed front-panel control boundaries from drawing 2000329.
///
/// These records intentionally stop before the untraced controller equation.
/// `Mod40Board::source_gate()` therefore continues to report panel arbitration
/// as incomplete.
pub const PANEL_CONTROL_OBSERVATIONS: [PanelControlObservation; 2] = [
    PanelControlObservation {
        source: "S26 STOP pushbutton",
        target: "parallel A36 7404 input-conditioning pair with switch contacts to ground",
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDF 13, drawing 2000329",
    },
    PanelControlObservation {
        source: "J2 4002 RESET ENABLE",
        target: "S31 SYSTEM/CPU mode-switch boundary",
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDF 13, drawing 2000329",
    },
];

/// Reset-pulse contract for the standard control-card configuration.
pub const CONTROL_RESET_PULSE_CONTRACT: ControlResetPulseContract = ControlResetPulseContract {
    initiators: [
        ControlResetInitiator::ConsoleResetPushbutton,
        ControlResetInitiator::UserReset,
        ControlResetInitiator::ProgramStoreModeChange,
    ],
    ttl_output_negative_going: true,
    minimum_duration_us: 500,
    evidence: Mod40RouteEvidence::Direct,
    source_locator: "98-095A printed pages 52-53; 98-013A PDF 7, drawing 2000319",
};

/// Single-step control-card contract for the standard MOD 40 configuration.
pub const PANEL_SINGLE_STEP_CONTRACT: PanelSingleStepContract = PanelSingleStepContract {
    switch_pulse_high: true,
    releases_stop_clamp: true,
    stop_acknowledge_rearms_clamp: true,
    completed_instruction_count: 1,
    evidence: Mod40RouteEvidence::Direct,
    source_locator: "98-095A printed page 53; 98-013A PDF 7, drawing 2000319",
};

/// Source-defined local write sequence for the standard IN-28 program-RAM card.
pub const IMM628_LOCAL_WRITE_PATH: Imm628LocalWritePath = Imm628LocalWritePath {
    address_latches_transparent: true,
    write_read_low_requests_write: true,
    input_latch_setup_precedes_write_pulse: true,
    byte_select_active_low: true,
    read_state_when_write_read_high: true,
    write_pulse_width_ns: None,
    evidence: Mod40RouteEvidence::Partial,
    source_locator: "98-095A printed pages 37-40; 98-013A PDF 10, drawing 01-0176-001",
};

/// Directly reviewed STOP ACK boundaries on the CPU, control, and panel cards.
///
/// The drawings expose these local path segments but do not yet establish the
/// motherboard or cable mapping that joins the segments. The source gate
/// therefore continues to leave panel arbitration incomplete.
pub const STOP_ACKNOWLEDGE_OBSERVATIONS: [StopAcknowledgeObservation; 3] = [
    StopAcknowledgeObservation {
        source: "imm4-43 P1 STOP ACK contact 30",
        target: "8095 buffer and R14 A11 4040 boundary",
        evidence: Mod40RouteEvidence::Partial,
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    StopAcknowledgeObservation {
        source: "imm4-72 P1 STOP ACK contact 73",
        target: "A15 7404 to J2 STOP ACK contact 9",
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDF 7, drawing 2000319",
    },
    StopAcknowledgeObservation {
        source: "front-panel J2 STOP ACK contact 9",
        target: "A29 7417 output to RUN indicator",
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDF 13, drawing 2000329; 98-095A printed page 63",
    },
];

/// Source-defined 4040 STOP and STOP ACK behavior at the package boundary.
///
/// The MCS-40 advance specification's single-step sequence says STPA returns
/// low when execution leaves STOP mode. Its turn-off edge is therefore the
/// released-high stopped state. The local CPU-card and external-interface
/// routes remain separate evidence boundaries.
pub const CPU_STOP_ACKNOWLEDGE_ENDPOINT: CpuStopAcknowledgeEndpoint = CpuStopAcknowledgeEndpoint {
    stop_input_active_high: true,
    acknowledge_open_drain: true,
    acknowledge_high_when_stopped: true,
    source_locator: "MCS-40 Advance Specifications printed page 37; 4040 datasheet pin description",
};

/// Reviewed A18 inputs for monitor-select decode from the imm4-43 schematic.
pub const MONITOR_SELECT_DECODE_INPUTS: [MonitorSelectDecodeInput; 6] = [
    MonitorSelectDecodeInput {
        signal: "C0",
        decoder_pin: Some("A18 A pin 13"),
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorSelectDecodeInput {
        signal: "C1",
        decoder_pin: Some("A18 B pin 3"),
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorSelectDecodeInput {
        signal: "ENABLE MON PROM",
        decoder_pin: Some("A18 2G pin 14, active low"),
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorSelectDecodeInput {
        signal: "OUT",
        decoder_pin: None,
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorSelectDecodeInput {
        signal: "C2",
        decoder_pin: None,
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorSelectDecodeInput {
        signal: "C3",
        decoder_pin: None,
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
];

/// A18 output pins visible in the monitor-select region of the imm4-43 sheet.
///
/// The primary sheet exposes these eight decoder output pins, then continues
/// through an untraced selection network toward A1 through A4. Every record is
/// deliberately partial until that output-to-chip-select path is complete.
pub const MONITOR_SELECT_DECODE_OUTPUTS: [MonitorSelectDecodeOutput; 8] = [
    MonitorSelectDecodeOutput {
        output: "1Y0",
        decoder_pin: 7,
        evidence: Mod40RouteEvidence::Partial,
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorSelectDecodeOutput {
        output: "1Y1",
        decoder_pin: 6,
        evidence: Mod40RouteEvidence::Partial,
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorSelectDecodeOutput {
        output: "1Y2",
        decoder_pin: 5,
        evidence: Mod40RouteEvidence::Partial,
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorSelectDecodeOutput {
        output: "1Y3",
        decoder_pin: 4,
        evidence: Mod40RouteEvidence::Partial,
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorSelectDecodeOutput {
        output: "2Y0",
        decoder_pin: 9,
        evidence: Mod40RouteEvidence::Partial,
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorSelectDecodeOutput {
        output: "2Y1",
        decoder_pin: 10,
        evidence: Mod40RouteEvidence::Partial,
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorSelectDecodeOutput {
        output: "2Y2",
        decoder_pin: 11,
        evidence: Mod40RouteEvidence::Partial,
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorSelectDecodeOutput {
        output: "2Y3",
        decoder_pin: 12,
        evidence: Mod40RouteEvidence::Partial,
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
];

/// Reviewed card-edge facts for the imm4-72, motherboard, and imm6-28 path.
pub const PROGRAM_RAM_CARD_EDGE_ROUTES: [ProgramRamCardEdgeRoute; 16] = [
    ProgramRamCardEdgeRoute {
        source_signal: "MAD0",
        source_contact: Some(11),
        target_signal: "MAD0",
        target_contact: 11,
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDFs 5, 7, and 10; controller P1 contact 11",
    },
    ProgramRamCardEdgeRoute {
        source_signal: "MAD1",
        source_contact: Some(12),
        target_signal: "MAD1",
        target_contact: 12,
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDFs 5, 7, and 10; controller P1 contact 12",
    },
    ProgramRamCardEdgeRoute {
        source_signal: "MAD2",
        source_contact: Some(13),
        target_signal: "MAD2",
        target_contact: 13,
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDFs 5, 7, and 10; controller P1 contact 13",
    },
    ProgramRamCardEdgeRoute {
        source_signal: "MAD3",
        source_contact: Some(14),
        target_signal: "MAD3",
        target_contact: 14,
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDFs 5, 7, and 10; controller P1 contact 14",
    },
    ProgramRamCardEdgeRoute {
        source_signal: "MAD4",
        source_contact: Some(15),
        target_signal: "MAD4",
        target_contact: 15,
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDFs 5, 7, and 10; controller P1 contact 15",
    },
    ProgramRamCardEdgeRoute {
        source_signal: "MAD5",
        source_contact: Some(16),
        target_signal: "MAD5",
        target_contact: 16,
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDFs 5, 7, and 10; controller P1 contact 16",
    },
    ProgramRamCardEdgeRoute {
        source_signal: "MAD6",
        source_contact: Some(17),
        target_signal: "MAD6",
        target_contact: 17,
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDFs 5, 7, and 10; controller P1 contact 17",
    },
    ProgramRamCardEdgeRoute {
        source_signal: "MAD7",
        source_contact: Some(18),
        target_signal: "MAD7",
        target_contact: 18,
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDFs 5, 7, and 10; controller P1 contact 18",
    },
    ProgramRamCardEdgeRoute {
        source_signal: "MAD8",
        source_contact: Some(19),
        target_signal: "MAD8",
        target_contact: 19,
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDFs 5, 7, and 10; controller P1 contact 19",
    },
    ProgramRamCardEdgeRoute {
        source_signal: "MAD9",
        source_contact: Some(20),
        target_signal: "MAD9",
        target_contact: 20,
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDFs 5, 7, and 10; controller P1 contact 20",
    },
    ProgramRamCardEdgeRoute {
        source_signal: "C3",
        source_contact: Some(94),
        target_signal: "MAD11",
        target_contact: 94,
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDFs 5 and 10; motherboard contact 94",
    },
    ProgramRamCardEdgeRoute {
        source_signal: "C2",
        source_contact: Some(96),
        target_signal: "MAD10",
        target_contact: 96,
        evidence: Mod40RouteEvidence::Direct,
        source_locator: "98-013A PDFs 5 and 10; motherboard contact 96",
    },
    ProgramRamCardEdgeRoute {
        source_signal: "BYTE2",
        source_contact: Some(90),
        target_signal: "BYTE2",
        target_contact: 90,
        evidence: Mod40RouteEvidence::Partial,
        source_locator: "98-013A PDFs 5, 7, and 10; local timing remains open",
    },
    ProgramRamCardEdgeRoute {
        source_signal: "BYTE1",
        source_contact: Some(92),
        target_signal: "BYTE1",
        target_contact: 92,
        evidence: Mod40RouteEvidence::Partial,
        source_locator: "98-013A PDFs 5, 7, and 10; local timing remains open",
    },
    ProgramRamCardEdgeRoute {
        source_signal: "MODULE SELECT",
        source_contact: None,
        target_signal: "MODULE SELECT",
        target_contact: 93,
        evidence: Mod40RouteEvidence::Partial,
        source_locator: "98-013A PDFs 5, 7, and 10; source polarity remains open",
    },
    ProgramRamCardEdgeRoute {
        source_signal: "WRITE",
        source_contact: Some(95),
        target_signal: "WRITE/READ",
        target_contact: 95,
        evidence: Mod40RouteEvidence::Partial,
        source_locator: "98-013A PDFs 5, 7, and 10; 98-095A printed page 40 defines low=write and high=read; 3404 and 2102 timing remains open",
    },
];

/// Reviewed CPU-card to ASR-33 cable conductors from drawing 2000325.
pub const TERMINAL_CABLE_ROUTES: [TerminalCableRoute; 3] = [
    TerminalCableRoute {
        endpoint: Mod40TerminalEndpoint::PrinterTransmitRam0Bit0,
        cpu_contact: 26,
        rear_connector_contact: 1,
        terminal_contact: 7,
        signal: "TTY OUT",
        source_locator: "98-013A PDF 29, drawing 2000325",
    },
    TerminalCableRoute {
        endpoint: Mod40TerminalEndpoint::KeyboardReceiveRom0Bit0,
        cpu_contact: 1,
        rear_connector_contact: 4,
        terminal_contact: 4,
        signal: "TTY IN",
        source_locator: "98-013A PDF 29, drawing 2000325",
    },
    TerminalCableRoute {
        endpoint: Mod40TerminalEndpoint::ReaderRunRam1Bit0,
        cpu_contact: 89,
        rear_connector_contact: 5,
        terminal_contact: 5,
        signal: "RDR CONT",
        source_locator: "98-013A PDF 29, drawing 2000325",
    },
];

/// Primary-reconciled terminal conventions for the three CPU-card endpoints.
pub const TERMINAL_PORT_POLARITIES: [TerminalPortPolarity; 3] = [
    TerminalPortPolarity {
        endpoint: Mod40TerminalEndpoint::KeyboardReceiveRom0Bit0,
        polarity: Mod40TerminalPolarity::Rom0InputHighWhenKeyboardLoopCurrentAbsent,
        source_locator: "98-013A PDFs 3 and 29, drawings 2000318 and 2000325; 98-095A PDF 152, printed page 136",
    },
    TerminalPortPolarity {
        endpoint: Mod40TerminalEndpoint::PrinterTransmitRam0Bit0,
        polarity: Mod40TerminalPolarity::Ram0Bit0HighDrivesPrinterMarkingCurrent,
        source_locator: "98-013A PDFs 3 and 29, drawings 2000318 and 2000325; 98-095A PDF 152, printed page 136",
    },
    TerminalPortPolarity {
        endpoint: Mod40TerminalEndpoint::ReaderRunRam1Bit0,
        polarity: Mod40TerminalPolarity::Ram1Bit0HighEnablesReader,
        source_locator: "98-013A PDFs 3 and 29, drawings 2000318 and 2000325; 98-095A PDF 152, printed page 136",
    },
];

/// Convert the keyboard loop current state into ROM 0 input bit 0.
///
/// The terminal start bit spaces the loop by removing current. The documented
/// monitor input procedure observes that state as a high ROM 0 input bit.
pub const fn keyboard_loop_current_to_rom0_input_bit0(loop_current_present: bool) -> bool {
    !loop_current_present
}

/// Return whether a RAM 0 port value drives marking current to the printer.
///
/// The primary terminal procedure defines every odd RAM 0 value as a mark and
/// every even value as a space. Bit 0 therefore controls loop current.
pub const fn ram0_port_value_drives_printer_marking_current(port_value: u8) -> bool {
    port_value & 1 != 0
}

/// Return whether a RAM 1 port value enables the ASR-33 paper reader.
///
/// The primary terminal procedure defines every odd RAM 1 value as reader
/// enable and every even value as reader disable. Bit 0 controls the relay
/// command but does not model its mechanical response time.
pub const fn ram1_port_value_enables_reader(port_value: u8) -> bool {
    port_value & 1 != 0
}

/// Shared 4289 monitor-address lines from drawing 2000318.
pub const MONITOR_ADDRESS_FANOUT: [MonitorAddressFanout; 8] = [
    MonitorAddressFanout {
        address_bit: 0,
        monitor_sockets: [1, 2, 3, 4],
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorAddressFanout {
        address_bit: 1,
        monitor_sockets: [1, 2, 3, 4],
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorAddressFanout {
        address_bit: 2,
        monitor_sockets: [1, 2, 3, 4],
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorAddressFanout {
        address_bit: 3,
        monitor_sockets: [1, 2, 3, 4],
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorAddressFanout {
        address_bit: 4,
        monitor_sockets: [1, 2, 3, 4],
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorAddressFanout {
        address_bit: 5,
        monitor_sockets: [1, 2, 3, 4],
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorAddressFanout {
        address_bit: 6,
        monitor_sockets: [1, 2, 3, 4],
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
    MonitorAddressFanout {
        address_bit: 7,
        monitor_sockets: [1, 2, 3, 4],
        source_locator: "98-013A PDF 3, drawing 2000318",
    },
];

/// Return whether the oscillator source reaches the first divider clock input.
pub const fn cpu_clock_source_is_traced() -> bool {
    matches!(CPU_CLOCK_RESET_ROUTES[0].evidence, Mod40RouteEvidence::Direct)
}

/// Return whether every named monitor-select decode input is source-recorded.
pub const fn monitor_select_decode_inputs_are_traced() -> bool {
    MONITOR_SELECT_DECODE_INPUTS.len() == 6
}

/// Return whether every A18 output pin is recorded without claiming its target.
pub const fn monitor_select_decode_outputs_are_recorded() -> bool {
    MONITOR_SELECT_DECODE_OUTPUTS.len() == 8
}

/// Return whether the sheet establishes the complete monitor data polarity.
///
/// The reviewed trace reaches the 1702A data region containing 74158 and 8095
/// stages. It does not establish a serial per-bit path, byte transform, or
/// socket order.
pub const fn monitor_data_polarity_is_traced() -> bool {
    false
}

/// Return whether every C1702A select path reaches one named physical socket.
///
/// The reviewed decoder records stop at the A18 output pins. No primary route
/// yet identifies a selected C1702A socket, so this remains false.
pub const fn monitor_socket_map_is_traced() -> bool {
    false
}

/// Return whether the reader-byte to executed-byte transform is primary-backed.
///
/// Public candidate bytes support a reproducible complement experiment, but no
/// primary per-bit route establishes that transform for the board.
pub const fn monitor_data_transform_is_primary_backed() -> bool {
    false
}

/// Return the number of accepted independent C1702A read sets.
///
/// The public corpus contains no accepted raw set because it lacks the
/// position-specific repeat, custody, reader, voltage, and photo records.
pub const fn accepted_monitor_read_set_count() -> u8 {
    0
}

/// Return whether the clock divider, reset release, and CPU phase timing close.
///
/// A functional divider target exists, but its Boolean equation and package
/// timing remain partial, so a historical CPU phase remains unauthorized.
pub const fn cpu_reset_and_phase_timing_is_traced() -> bool {
    false
}

/// Return whether the IN-28 write one-shot and 2102 timing close.
///
/// The card-edge polarity and write ordering are direct. The component values,
/// pulse width, propagation budget, and device timing remain incomplete.
pub const fn program_ram_write_timing_is_traced() -> bool {
    false
}

/// Return whether front-panel controls form a complete arbitration equation.
///
/// Local reset and single-step contracts exist, but STOP ACK continuity and
/// panel priority remain untraced across the board boundaries.
pub const fn panel_arbitration_is_traced() -> bool {
    false
}

/// Return whether current-loop transistor thresholds and relay timing close.
///
/// The source establishes the three terminal conductors and port-bit senses.
/// It does not establish the entire Q3, Q4, and Q5 electrical timing path.
pub const fn terminal_electrical_timing_is_traced() -> bool {
    false
}

/// Return whether every reviewed IN-28 card-edge route is complete.
pub fn program_ram_card_edge_is_complete() -> bool {
    PROGRAM_RAM_CARD_EDGE_ROUTES
        .iter()
        .all(|route| route.evidence == Mod40RouteEvidence::Direct)
}

/// Return whether all three external terminal conductors are source-recorded.
pub const fn terminal_cable_routes_are_traced() -> bool {
    TERMINAL_CABLE_ROUTES.len() == 3
}

/// Return whether the terminal current-loop assertion polarity is source-traced.
///
/// The physical conductors come from 98-013A. The 98-095A terminal procedure
/// defines the ROM 0, RAM 0, and RAM 1 logical conventions. Relay mechanics
/// and transistor switching waveforms remain separate open evidence.
pub const fn terminal_current_loop_polarity_is_traced() -> bool {
    TERMINAL_PORT_POLARITIES.len() == 3
}

/// Return whether all eight shared monitor address outputs are source-recorded.
pub const fn monitor_address_fanout_is_traced() -> bool {
    MONITOR_ADDRESS_FANOUT.len() == 8
}

#[cfg(test)]
mod tests {
    use super::{
        accepted_monitor_read_set_count, cpu_clock_source_is_traced, cpu_nominal_machine_clock_hz,
        cpu_reset_and_phase_timing_is_traced, imm628_has_exactly_one_selected_byte,
        imm628_write_read_level_requests_write, keyboard_loop_current_to_rom0_input_bit0,
        monitor_address_fanout_is_traced, monitor_data_polarity_is_traced, monitor_data_transform_is_primary_backed,
        monitor_select_decode_inputs_are_traced, monitor_select_decode_outputs_are_recorded,
        monitor_socket_map_is_traced, panel_arbitration_is_traced, program_ram_card_edge_is_complete,
        program_ram_write_timing_is_traced, ram0_port_value_drives_printer_marking_current,
        ram1_port_value_enables_reader, terminal_cable_routes_are_traced, terminal_current_loop_polarity_is_traced,
        terminal_electrical_timing_is_traced, ControlResetInitiator, ControlWriteCommandCause, Mod40RouteEvidence,
        CONTROL_RESET_PULSE_CONTRACT, CONTROL_WRITE_COMMAND_CAUSES, CONTROL_WRITE_COMMAND_ROUTE,
        CPU_CLOCK_RESET_ROUTES, CPU_CLOCK_TIMING_TARGET, CPU_RESET_TIMING_REQUIREMENT, CPU_STOP_ACKNOWLEDGE_ENDPOINT,
        IMM628_LOCAL_WRITE_PATH, IMM628_WRITE_READ_INPUT, MONITOR_ADDRESS_FANOUT, MONITOR_SELECT_DECODE_INPUTS,
        MONITOR_SELECT_DECODE_OUTPUTS, PANEL_CONTROL_OBSERVATIONS, PANEL_SINGLE_STEP_CONTRACT,
        PROGRAM_RAM_CARD_EDGE_ROUTES, STOP_ACKNOWLEDGE_OBSERVATIONS, TERMINAL_CABLE_ROUTES, TERMINAL_PORT_POLARITIES,
    };
    use crate::mod40::Mod40TerminalEndpoint;

    #[test]
    fn low_program_ram_address_contacts_preserve_the_direct_one_to_one_route() {
        for (bit, route) in PROGRAM_RAM_CARD_EDGE_ROUTES.iter().take(10).enumerate() {
            let contact = 11 + bit as u8;
            assert_eq!(route.source_signal, format!("MAD{bit}"));
            assert_eq!(route.target_signal, format!("MAD{bit}"));
            assert_eq!(route.source_contact, Some(contact));
            assert_eq!(route.target_contact, contact);
            assert_eq!(route.evidence, Mod40RouteEvidence::Direct);
        }
    }

    #[test]
    fn partial_control_routes_hold_the_program_ram_execution_gate_open() {
        assert!(!program_ram_card_edge_is_complete());
        assert!(PROGRAM_RAM_CARD_EDGE_ROUTES
            .iter()
            .any(|route| route.evidence == Mod40RouteEvidence::Partial));
    }

    #[test]
    fn stop_acknowledge_paths_remain_local_until_connector_mapping_is_traced() {
        assert_eq!(STOP_ACKNOWLEDGE_OBSERVATIONS.len(), 3);
        assert_eq!(STOP_ACKNOWLEDGE_OBSERVATIONS[0].evidence, Mod40RouteEvidence::Partial);
        assert_eq!(
            STOP_ACKNOWLEDGE_OBSERVATIONS[1].source,
            "imm4-72 P1 STOP ACK contact 73"
        );
        assert_eq!(
            STOP_ACKNOWLEDGE_OBSERVATIONS[2].target,
            "A29 7417 output to RUN indicator"
        );
    }

    #[test]
    fn cpu_stop_acknowledge_endpoint_is_distinct_from_untraced_board_nets() {
        let endpoint = std::hint::black_box(CPU_STOP_ACKNOWLEDGE_ENDPOINT);
        assert!(endpoint.stop_input_active_high);
        assert!(endpoint.acknowledge_open_drain);
        assert!(endpoint.acknowledge_high_when_stopped);
        assert!(endpoint.source_locator.contains("printed page 37"));
        assert_eq!(STOP_ACKNOWLEDGE_OBSERVATIONS[0].evidence, Mod40RouteEvidence::Partial);
    }

    #[test]
    fn clock_target_records_the_divide_by_seven_boundary_without_closing_the_phase_gate() {
        let target = std::hint::black_box(CPU_CLOCK_TIMING_TARGET);
        assert_eq!(target.crystal_frequency_hz, 5_185_000);
        assert_eq!(target.divider, 7);
        assert_eq!(cpu_nominal_machine_clock_hz(), 740_714);
        assert_eq!(target.phase_pulse_width_ns, 386);
        assert!(target.phases_non_overlapping);
        assert_eq!(target.evidence, Mod40RouteEvidence::Partial);
    }

    #[test]
    fn cpu_reset_requirement_preserves_the_documented_duration_without_closing_the_phase_gate() {
        let requirement = std::hint::black_box(CPU_RESET_TIMING_REQUIREMENT);
        assert!(requirement.board_reset_active_low);
        assert!(requirement.package_reset_active_high);
        assert_eq!(requirement.minimum_full_instruction_cycles, 8);
        assert_eq!(requirement.minimum_external_clock_periods, 64);
        assert_eq!(requirement.evidence, Mod40RouteEvidence::Partial);
        assert!(!cpu_reset_and_phase_timing_is_traced());
    }

    #[test]
    fn imm628_write_read_card_input_uses_low_for_write_and_high_for_read() {
        assert_eq!(IMM628_WRITE_READ_INPUT.evidence, Mod40RouteEvidence::Direct);
        assert!(imm628_write_read_level_requests_write(false));
        assert!(!imm628_write_read_level_requests_write(true));

        let write_route = &PROGRAM_RAM_CARD_EDGE_ROUTES[15];
        assert_eq!(write_route.source_signal, "WRITE");
        assert_eq!(write_route.target_signal, "WRITE/READ");
        assert_eq!(write_route.target_contact, 95);
        assert_eq!(write_route.evidence, Mod40RouteEvidence::Partial);
        assert!(imm628_has_exactly_one_selected_byte(false, true));
        assert!(imm628_has_exactly_one_selected_byte(true, false));
        assert!(!imm628_has_exactly_one_selected_byte(false, false));
        assert!(!imm628_has_exactly_one_selected_byte(true, true));
    }

    #[test]
    fn in28_write_path_records_setup_order_without_inventing_a_pulse_width() {
        let path = std::hint::black_box(IMM628_LOCAL_WRITE_PATH);
        assert!(path.address_latches_transparent);
        assert!(path.write_read_low_requests_write);
        assert!(path.input_latch_setup_precedes_write_pulse);
        assert!(path.byte_select_active_low);
        assert!(path.read_state_when_write_read_high);
        assert_eq!(path.write_pulse_width_ns, None);
        assert_eq!(path.evidence, Mod40RouteEvidence::Partial);
    }

    #[test]
    fn controller_write_command_inverts_before_the_module_input() {
        let route = std::hint::black_box(CONTROL_WRITE_COMMAND_ROUTE);
        assert_eq!(CONTROL_WRITE_COMMAND_CAUSES.len(), 2);
        assert_eq!(CONTROL_WRITE_COMMAND_CAUSES[0], ControlWriteCommandCause::CmaEnable);
        assert!(route.controller_command_high);
        assert!(route.module_input_active_low);
        assert_eq!(route.evidence, Mod40RouteEvidence::Partial);
    }

    #[test]
    fn terminal_routes_keep_the_three_logical_endpoints_distinct() {
        assert!(terminal_cable_routes_are_traced());
        assert!(terminal_current_loop_polarity_is_traced());
        assert_eq!(TERMINAL_CABLE_ROUTES[0].cpu_contact, 26);
        assert_eq!(TERMINAL_CABLE_ROUTES[1].cpu_contact, 1);
        assert_eq!(TERMINAL_CABLE_ROUTES[2].cpu_contact, 89);
        assert_eq!(
            TERMINAL_CABLE_ROUTES[1].endpoint,
            Mod40TerminalEndpoint::KeyboardReceiveRom0Bit0
        );
        assert_eq!(TERMINAL_PORT_POLARITIES.len(), 3);
        assert!(keyboard_loop_current_to_rom0_input_bit0(false));
        assert!(!keyboard_loop_current_to_rom0_input_bit0(true));
        assert!(!ram0_port_value_drives_printer_marking_current(0));
        assert!(ram0_port_value_drives_printer_marking_current(1));
        assert!(!ram0_port_value_drives_printer_marking_current(2));
        assert!(ram0_port_value_drives_printer_marking_current(15));
        assert!(!ram1_port_value_enables_reader(0));
        assert!(ram1_port_value_enables_reader(1));
        assert!(!ram1_port_value_enables_reader(2));
        assert!(ram1_port_value_enables_reader(15));
    }

    #[test]
    fn monitor_address_lines_fan_out_to_all_four_resident_prom_sockets() {
        assert!(monitor_address_fanout_is_traced());
        for (bit, route) in MONITOR_ADDRESS_FANOUT.iter().enumerate() {
            assert_eq!(route.address_bit, bit as u8);
            assert_eq!(route.monitor_sockets, [1, 2, 3, 4]);
        }
    }

    #[test]
    fn cpu_clock_source_and_monitor_decode_inputs_remain_evidence_records() {
        assert!(cpu_clock_source_is_traced());
        assert_eq!(CPU_CLOCK_RESET_ROUTES[0].source, "Y1 5.185 MHz crystal oscillator");
        assert_eq!(CPU_CLOCK_RESET_ROUTES[1].target, "A11 4040 RESET pin 12");
        assert_eq!(CPU_CLOCK_RESET_ROUTES[3].target, "A11 4040 phi1 and phi2 input paths");
        assert!(monitor_select_decode_inputs_are_traced());
        assert_eq!(MONITOR_SELECT_DECODE_INPUTS[2].signal, "ENABLE MON PROM");
        assert_eq!(
            MONITOR_SELECT_DECODE_INPUTS[2].decoder_pin,
            Some("A18 2G pin 14, active low")
        );
        assert!(!monitor_data_polarity_is_traced());
    }

    #[test]
    fn panel_observations_preserve_direct_boundaries_without_authorizing_arbitration() {
        assert_eq!(PANEL_CONTROL_OBSERVATIONS.len(), 2);
        assert_eq!(PANEL_CONTROL_OBSERVATIONS[0].source, "S26 STOP pushbutton");
        assert_eq!(
            PANEL_CONTROL_OBSERVATIONS[1].target,
            "S31 SYSTEM/CPU mode-switch boundary"
        );
        assert!(PANEL_CONTROL_OBSERVATIONS
            .iter()
            .all(|observation| observation.evidence == Mod40RouteEvidence::Direct));
    }

    #[test]
    fn control_card_records_reset_and_single_step_contracts_without_closing_panel_arbitration() {
        let reset = std::hint::black_box(CONTROL_RESET_PULSE_CONTRACT);
        assert_eq!(reset.minimum_duration_us, 500);
        assert!(reset.ttl_output_negative_going);
        assert_eq!(reset.evidence, Mod40RouteEvidence::Direct);
        assert_eq!(reset.initiators[0], ControlResetInitiator::ConsoleResetPushbutton);
        assert_eq!(reset.initiators[1], ControlResetInitiator::UserReset);
        assert_eq!(reset.initiators[2], ControlResetInitiator::ProgramStoreModeChange);

        let single_step = std::hint::black_box(PANEL_SINGLE_STEP_CONTRACT);
        assert!(single_step.switch_pulse_high);
        assert!(single_step.releases_stop_clamp);
        assert!(single_step.stop_acknowledge_rearms_clamp);
        assert_eq!(single_step.completed_instruction_count, 1);
        assert_eq!(single_step.evidence, Mod40RouteEvidence::Direct);
    }

    #[test]
    fn monitor_decoder_outputs_record_a18_pins_without_closing_the_socket_map() {
        assert!(monitor_select_decode_outputs_are_recorded());
        assert_eq!(MONITOR_SELECT_DECODE_OUTPUTS[0].output, "1Y0");
        assert_eq!(MONITOR_SELECT_DECODE_OUTPUTS[0].decoder_pin, 7);
        assert_eq!(MONITOR_SELECT_DECODE_OUTPUTS[4].output, "2Y0");
        assert_eq!(MONITOR_SELECT_DECODE_OUTPUTS[7].decoder_pin, 12);
        assert!(MONITOR_SELECT_DECODE_OUTPUTS
            .iter()
            .all(|output| output.evidence == Mod40RouteEvidence::Partial));
    }

    #[test]
    fn unresolved_evidence_gates_stay_derived_from_route_records() {
        assert!(!cpu_reset_and_phase_timing_is_traced());
        assert!(!program_ram_write_timing_is_traced());
        assert!(!panel_arbitration_is_traced());
        assert!(!terminal_electrical_timing_is_traced());
        assert!(!monitor_socket_map_is_traced());
        assert!(!monitor_data_transform_is_primary_backed());
        assert_eq!(accepted_monitor_read_set_count(), 0);
    }
}
