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
        target_signal: "WRITE",
        target_contact: 95,
        evidence: Mod40RouteEvidence::Partial,
        source_locator: "98-013A PDFs 5, 7, and 10; 3404 and 2102 write timing remains open",
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

/// Return whether all eight shared monitor address outputs are source-recorded.
pub const fn monitor_address_fanout_is_traced() -> bool {
    MONITOR_ADDRESS_FANOUT.len() == 8
}

#[cfg(test)]
mod tests {
    use super::{
        monitor_address_fanout_is_traced, program_ram_card_edge_is_complete, terminal_cable_routes_are_traced,
        Mod40RouteEvidence, MONITOR_ADDRESS_FANOUT, PROGRAM_RAM_CARD_EDGE_ROUTES, TERMINAL_CABLE_ROUTES,
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
    fn terminal_routes_keep_the_three_logical_endpoints_distinct() {
        assert!(terminal_cable_routes_are_traced());
        assert_eq!(TERMINAL_CABLE_ROUTES[0].cpu_contact, 26);
        assert_eq!(TERMINAL_CABLE_ROUTES[1].cpu_contact, 1);
        assert_eq!(TERMINAL_CABLE_ROUTES[2].cpu_contact, 89);
        assert_eq!(
            TERMINAL_CABLE_ROUTES[1].endpoint,
            Mod40TerminalEndpoint::KeyboardReceiveRom0Bit0
        );
    }

    #[test]
    fn monitor_address_lines_fan_out_to_all_four_resident_prom_sockets() {
        assert!(monitor_address_fanout_is_traced());
        for (bit, route) in MONITOR_ADDRESS_FANOUT.iter().enumerate() {
            assert_eq!(route.address_bit, bit as u8);
            assert_eq!(route.monitor_sockets, [1, 2, 3, 4]);
        }
    }
}
