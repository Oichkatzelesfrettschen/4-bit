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

/// Return whether an imm6-28 card-input logic level requests a write.
pub const fn imm628_write_read_level_requests_write(level_high: bool) -> bool {
    !level_high
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
        cpu_clock_source_is_traced, imm628_write_read_level_requests_write, keyboard_loop_current_to_rom0_input_bit0,
        monitor_address_fanout_is_traced, monitor_data_polarity_is_traced, monitor_select_decode_inputs_are_traced,
        monitor_select_decode_outputs_are_recorded, program_ram_card_edge_is_complete,
        ram0_port_value_drives_printer_marking_current, ram1_port_value_enables_reader,
        terminal_cable_routes_are_traced, terminal_current_loop_polarity_is_traced, Mod40RouteEvidence,
        CPU_CLOCK_RESET_ROUTES, IMM628_WRITE_READ_INPUT, MONITOR_ADDRESS_FANOUT, MONITOR_SELECT_DECODE_INPUTS,
        MONITOR_SELECT_DECODE_OUTPUTS, PANEL_CONTROL_OBSERVATIONS, PROGRAM_RAM_CARD_EDGE_ROUTES, TERMINAL_CABLE_ROUTES,
        TERMINAL_PORT_POLARITIES,
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
    fn imm628_write_read_card_input_uses_low_for_write_and_high_for_read() {
        assert_eq!(IMM628_WRITE_READ_INPUT.evidence, Mod40RouteEvidence::Direct);
        assert!(imm628_write_read_level_requests_write(false));
        assert!(!imm628_write_read_level_requests_write(true));

        let write_route = &PROGRAM_RAM_CARD_EDGE_ROUTES[15];
        assert_eq!(write_route.source_signal, "WRITE");
        assert_eq!(write_route.target_signal, "WRITE/READ");
        assert_eq!(write_route.target_contact, 95);
        assert_eq!(write_route.evidence, Mod40RouteEvidence::Partial);
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
}
