//! Source-bound Intellec 4/MOD 40 card composition.
//!
//! This module owns the documented imm4-43, imm4-72, and imm6-28 card
//! inventory. It remains deliberately non-executable until the retained
//! schematics establish the clock, control, program-RAM, panel, and terminal
//! electrical nets required for a historical phase.

use mcs4_chips::{i1702::I1702A, i4002::I4002, i4040::I4040, i4289::I4289};

/// Canonical MOD 40 documentary gate identifiers generated from the ledger.
pub const MOD40_EVIDENCE_GATE_IDS: [&str; 6] = crate::mod40_evidence_generated::MOD40_EVIDENCE_GATE_IDS;
use crate::{
    console::ProgramMemoryMode,
    imm6_28::Imm628,
    mod40_evidence_generated::MOD40_EVIDENCE_GATE_CLOSED,
    mod40_routes::{
        accepted_monitor_read_set_count, cpu_clock_source_is_traced, cpu_reset_and_phase_timing_is_traced,
        monitor_address_fanout_is_traced, monitor_data_polarity_is_traced, monitor_data_transform_is_primary_backed,
        monitor_select_decode_inputs_are_traced, monitor_socket_map_is_traced, panel_arbitration_is_traced,
        program_ram_card_edge_is_complete, program_ram_write_timing_is_traced, terminal_cable_routes_are_traced,
        terminal_current_loop_polarity_is_traced, terminal_electrical_timing_is_traced,
    },
    profile::{SourceReference, MOD40_SOURCES},
};

/// Typed status for one documentary gate in the MOD 40 route ledger.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mod40EvidenceGateStatus {
    /// Canonical route-ledger identifier.
    pub id: &'static str,
    /// True when the board still lacks the evidence required by this gate.
    pub blocked: bool,
}

/// Monotonic source and implementation fidelity reached by a MOD 40 board.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Mod40FidelityLevel {
    /// Required physical inventory facts are not source-bound.
    Unbound,
    /// Primary sources establish the fitted cards and device population.
    DocumentedInventory,
    /// CPU, program-RAM, and monitor electrical routes are source-verified.
    VerifiedCoreElectricalRoutes,
    /// The verified electrical routes participate in one board-cycle model.
    VerifiedBoardCycle,
    /// Monitor bytes have position-specific, independently repeated provenance.
    ProvenanceCompleteMonitor,
    /// Panel arbitration drives the verified board-cycle model.
    PanelOperatedSystem,
    /// The source-bound terminal electrical path exposes observable operation.
    TerminalObservableSystem,
    /// A source-gated FPGA wrapper passes the common equivalence trace.
    FpgaComparableSystem,
}

impl Mod40FidelityLevel {
    /// Return the stable user-facing name for this fidelity boundary.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unbound => "unbound",
            Self::DocumentedInventory => "documented inventory",
            Self::VerifiedCoreElectricalRoutes => "verified core electrical routes",
            Self::VerifiedBoardCycle => "verified board cycle",
            Self::ProvenanceCompleteMonitor => "provenance-complete monitor",
            Self::PanelOperatedSystem => "panel-operated system",
            Self::TerminalObservableSystem => "terminal-observable system",
            Self::FpgaComparableSystem => "FPGA-comparable system",
        }
    }
}

/// Immutable MOD 40 evidence and implementation state for user interfaces.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mod40EvidenceSnapshot {
    /// Highest monotonic fidelity level whose prerequisites are satisfied.
    pub fidelity_level: Mod40FidelityLevel,
    /// True only when the complete historical software model may execute.
    pub execution_authorized: bool,
    /// True when the reconciled electrical routes participate in CPU cycles.
    pub board_cycle_wiring_implemented: bool,
    /// True when a source-gated FPGA wrapper participates in equivalence.
    pub fpga_wrapper_implemented: bool,
    /// Logical program store selected by the imm4-72 control card.
    pub selected_store: ProgramStoreId,
    /// Number of fitted resident monitor sockets.
    pub monitor_socket_count: usize,
    /// Number of fitted 2102 devices on the imm6-28 card.
    pub program_ram_chip_count: usize,
    /// Number of accepted independently acquired monitor read sets.
    pub accepted_monitor_read_set_count: u8,
    /// Canonical documentary gate states in ledger order.
    pub gates: [Mod40EvidenceGateStatus; 6],
    /// Canonical identifiers for every blocked documentary gate.
    pub blocked_gate_ids: Vec<&'static str>,
}

/// One program-store selection owned by the imm4-72 control card.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgramStoreId {
    /// The resident monitor set on the imm4-43 CPU card.
    Monitor1702A,
    /// The imm6-28 2102 program-RAM card.
    ProgramRam2102,
    /// An optional imm6-26 PROM-memory card.
    PromMemory,
}

/// One logical terminal endpoint on the imm4-43 CPU card.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mod40TerminalEndpoint {
    /// Q5 receives keyboard serial data through ROM 0 input bit 0.
    KeyboardReceiveRom0Bit0,
    /// Q4 transmits printer serial data through RAM 0 output bit 0.
    PrinterTransmitRam0Bit0,
    /// Q3 drives reader-run control through RAM 1 output bit 0.
    ReaderRunRam1Bit0,
}

/// A failure that prevents a source-faithful MOD 40 phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mod40AssemblyError {
    /// The source set does not yet authorize historical execution.
    MissingEvidence(&'static str),
    /// The clock, reset, STOP, and phase nets remain unextracted.
    ClockNetMapIncomplete,
    /// The control-card to program-RAM transaction nets remain unextracted.
    ProgramRamNetMapIncomplete,
    /// The panel control and console-memory nets remain unextracted.
    PanelNetMapIncomplete,
    /// The terminal electrical polarity and current-loop nets remain unextracted.
    TerminalElectricalMapIncomplete,
}

/// Source-backed conditions required before a MOD 40 can execute a board phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mod40SourceGate {
    /// Retained primary sources establish the standard fitted card inventory.
    pub primary_board_population_bound: bool,
    /// The imm4-43 physically modeled monitor sockets A1 through A4 exist.
    pub monitor_slots_present: [bool; 4],
    /// The imm6-28 retains all 32 documented 2102 devices.
    pub program_ram_chip_count: usize,
    /// Every source-visible program-RAM card-edge route is complete.
    pub program_ram_card_edges_traced: bool,
    /// All four monitor sockets share the documented eight address outputs.
    pub monitor_address_fanout_traced: bool,
    /// The three external TTY cable conductors have source-recorded endpoints.
    pub terminal_cable_routes_traced: bool,
    /// The TTY and reader current-loop logical polarities are source-traced.
    pub terminal_current_loop_polarity_traced: bool,
    /// The terminal transistor thresholds and reader-relay timing are traced.
    pub terminal_electrical_timing_traced: bool,
    /// The CPU-card oscillator source reaches the documented divider input.
    pub cpu_clock_source_traced: bool,
    /// Named monitor-select inputs reach the documented A18 decoder region.
    pub monitor_select_decode_inputs_traced: bool,
    /// The complete C1702A output-data polarity is source-traced.
    pub monitor_data_polarity_traced: bool,
    /// Every C1702A socket has a primary-backed select and address-block map.
    pub monitor_socket_map_traced: bool,
    /// The raw-reader to executed-byte transform is primary-backed.
    pub monitor_data_transform_primary_backed: bool,
    /// Number of accepted independent, position-specific C1702A read sets.
    pub accepted_monitor_read_set_count: u8,
    /// The CPU reset assertion, divider outputs, and 4040 phase timing are traced.
    pub cpu_reset_and_phase_timing_traced: bool,
    /// The imm4-72 to imm6-28 write pulse, latch phase, and timing are traced.
    pub program_ram_write_timing_traced: bool,
    /// Panel STOP, reset, step, and program-store arbitration are traced.
    pub panel_arbitration_traced: bool,
    /// CPU-cycle electrical wiring is implemented from a reconciled net ledger.
    pub board_cycle_wiring_implemented: bool,
    /// A dedicated MOD 40 FPGA wrapper participates in a source-gated trace.
    pub fpga_wrapper_implemented: bool,
    /// Four monitor images have verified physical-read lineage and transforms.
    pub monitor_media_verified: bool,
}

/// Documented state on the imm4-43 central processor module.
pub struct Imm443 {
    cpu: I4040,
    memory_interface: I4289,
    monitor_prom: [I1702A; 4],
    data_ram: [I4002; 4],
}

impl Imm443 {
    fn new() -> Self {
        Self {
            cpu: I4040::new(),
            memory_interface: I4289::new(),
            monitor_prom: std::array::from_fn(|socket| I1702A::new(socket as u8)),
            data_ram: std::array::from_fn(|chip_id| I4002::new(chip_id as u8, 0)),
        }
    }

    /// Return the documented 4040 core.
    pub const fn cpu(&self) -> &I4040 {
        &self.cpu
    }

    /// Return the documented 4289 interface.
    pub const fn memory_interface(&self) -> &I4289 {
        &self.memory_interface
    }

    /// Return the number of resident 1702A monitor sockets.
    pub const fn monitor_socket_count(&self) -> usize {
        self.monitor_prom.len()
    }

    /// Return whether every resident monitor device has established byte values.
    ///
    /// Byte knowledge does not establish the independent physical-read lineage
    /// required to close the historical MOD 40 monitor-media gate.
    pub fn monitor_media_is_fully_known(&self) -> bool {
        self.monitor_prom.iter().all(I1702A::is_fully_known)
    }

    /// Return the number of resident 4002 data-RAM sockets.
    pub const fn data_ram_socket_count(&self) -> usize {
        self.data_ram.len()
    }

    /// Identify the documented terminal role without exposing a generic port map.
    pub const fn terminal_endpoint(&self, endpoint: Mod40TerminalEndpoint) -> Mod40TerminalEndpoint {
        endpoint
    }
}

/// Program-memory selection state owned by the imm4-72 control module.
pub struct Imm472 {
    selected_store: ProgramStoreId,
}

impl Imm472 {
    fn new() -> Self {
        Self {
            selected_store: ProgramStoreId::Monitor1702A,
        }
    }

    /// Apply a typed panel memory-mode request to the control-card selection latch.
    pub fn apply_panel_mode(&mut self, mode: ProgramMemoryMode) {
        self.selected_store = match mode {
            ProgramMemoryMode::Monitor => ProgramStoreId::Monitor1702A,
            ProgramMemoryMode::Ram => ProgramStoreId::ProgramRam2102,
            ProgramMemoryMode::Prom => ProgramStoreId::PromMemory,
        };
    }

    /// Return the one selected logical program store.
    pub const fn selected_store(&self) -> ProgramStoreId {
        self.selected_store
    }
}

/// Source-bound composition of the three standard MOD 40 cards.
pub struct Mod40Board {
    imm443: Imm443,
    imm472: Imm472,
    imm628: Imm628,
}

impl Mod40Board {
    /// Construct the documented card inventory without authorizing execution.
    pub fn new() -> Self {
        Self {
            imm443: Imm443::new(),
            imm472: Imm472::new(),
            imm628: Imm628::new(),
        }
    }

    /// Return the source references that establish the card inventory.
    pub const fn sources(&self) -> &'static [SourceReference] {
        MOD40_SOURCES
    }

    /// Return the CPU card.
    pub const fn imm443(&self) -> &Imm443 {
        &self.imm443
    }

    /// Return the control card.
    pub const fn imm472(&self) -> &Imm472 {
        &self.imm472
    }

    /// Return the program-RAM card.
    pub const fn imm628(&self) -> &Imm628 {
        &self.imm628
    }

    /// Apply a panel memory-mode request only through the control card.
    pub fn apply_panel_memory_mode(&mut self, mode: ProgramMemoryMode) {
        self.imm472.apply_panel_mode(mode);
    }

    /// Return the source-gate state without silently enabling a board phase.
    pub fn source_gate(&self) -> Mod40SourceGate {
        let accepted_monitor_read_set_count = accepted_monitor_read_set_count();
        let monitor_socket_map_traced = monitor_socket_map_is_traced();
        let monitor_data_transform_primary_backed = monitor_data_transform_is_primary_backed();
        Mod40SourceGate {
            primary_board_population_bound: true,
            monitor_slots_present: [true; 4],
            program_ram_chip_count: self.imm628.device_count(),
            program_ram_card_edges_traced: program_ram_card_edge_is_complete(),
            monitor_address_fanout_traced: monitor_address_fanout_is_traced(),
            terminal_cable_routes_traced: terminal_cable_routes_are_traced(),
            terminal_current_loop_polarity_traced: terminal_current_loop_polarity_is_traced(),
            terminal_electrical_timing_traced: terminal_electrical_timing_is_traced(),
            cpu_clock_source_traced: cpu_clock_source_is_traced(),
            monitor_select_decode_inputs_traced: monitor_select_decode_inputs_are_traced(),
            monitor_data_polarity_traced: monitor_data_polarity_is_traced(),
            monitor_socket_map_traced,
            monitor_data_transform_primary_backed,
            accepted_monitor_read_set_count,
            cpu_reset_and_phase_timing_traced: cpu_reset_and_phase_timing_is_traced(),
            program_ram_write_timing_traced: program_ram_write_timing_is_traced(),
            panel_arbitration_traced: panel_arbitration_is_traced(),
            board_cycle_wiring_implemented: false,
            fpga_wrapper_implemented: false,
            monitor_media_verified: MOD40_EVIDENCE_GATE_CLOSED[5]
                && accepted_monitor_read_set_count >= 2
                && monitor_socket_map_traced
                && monitor_data_transform_primary_backed,
        }
    }

    /// Return whether every source condition and board-cycle implementation exists.
    pub fn historical_execution_is_authorized(&self) -> bool {
        let gate = self.source_gate();
        gate.cpu_reset_and_phase_timing_traced
            && gate.program_ram_write_timing_traced
            && gate.panel_arbitration_traced
            && gate.terminal_electrical_timing_traced
            && gate.monitor_media_verified
            && gate.board_cycle_wiring_implemented
    }

    /// Return the highest monotonic fidelity level authorized by current facts.
    pub fn fidelity_level(&self) -> Mod40FidelityLevel {
        fidelity_level_from_gate(
            self.source_gate(),
            self.imm443.monitor_socket_count(),
            self.imm628.device_count(),
        )
    }

    /// Return immutable evidence state for a non-executable status surface.
    pub fn evidence_snapshot(&self) -> Mod40EvidenceSnapshot {
        let gate = self.source_gate();
        Mod40EvidenceSnapshot {
            fidelity_level: fidelity_level_from_gate(
                gate,
                self.imm443.monitor_socket_count(),
                self.imm628.device_count(),
            ),
            execution_authorized: self.historical_execution_is_authorized(),
            board_cycle_wiring_implemented: gate.board_cycle_wiring_implemented,
            fpga_wrapper_implemented: gate.fpga_wrapper_implemented,
            selected_store: self.imm472.selected_store(),
            monitor_socket_count: self.imm443.monitor_socket_count(),
            program_ram_chip_count: self.imm628.device_count(),
            accepted_monitor_read_set_count: gate.accepted_monitor_read_set_count,
            gates: self.evidence_gate_statuses(),
            blocked_gate_ids: self.blocked_evidence_gate_ids(),
        }
    }

    /// Return canonical evidence-gate IDs that still block source-faithful execution.
    ///
    /// Board-cycle wiring is an implementation condition, not an evidence-ledger
    /// gate, so it does not appear in this list.
    pub fn blocked_evidence_gate_ids(&self) -> Vec<&'static str> {
        self.evidence_gate_statuses()
            .into_iter()
            .filter_map(|status| status.blocked.then_some(status.id))
            .collect()
    }

    /// Return all typed documentary gate states in canonical ledger order.
    pub fn evidence_gate_statuses(&self) -> [Mod40EvidenceGateStatus; 6] {
        std::array::from_fn(|index| Mod40EvidenceGateStatus {
            id: MOD40_EVIDENCE_GATE_IDS[index],
            blocked: !MOD40_EVIDENCE_GATE_CLOSED[index],
        })
    }

    /// Reject a historical phase until every required board evidence gate closes.
    pub fn validate_historical_execution(&self) -> Result<(), Mod40AssemblyError> {
        let source_gate = self.source_gate();
        if !source_gate.cpu_reset_and_phase_timing_traced {
            return Err(Mod40AssemblyError::ClockNetMapIncomplete);
        }
        if !source_gate.program_ram_write_timing_traced {
            return Err(Mod40AssemblyError::ProgramRamNetMapIncomplete);
        }
        if !source_gate.panel_arbitration_traced {
            return Err(Mod40AssemblyError::PanelNetMapIncomplete);
        }
        if !source_gate.terminal_electrical_timing_traced {
            return Err(Mod40AssemblyError::TerminalElectricalMapIncomplete);
        }
        if !source_gate.monitor_media_verified {
            return Err(Mod40AssemblyError::MissingEvidence("intellec4-mod40-monitor-prom-set"));
        }
        if !source_gate.board_cycle_wiring_implemented {
            return Err(Mod40AssemblyError::MissingEvidence(
                "intellec4-mod40-board-cycle-wiring",
            ));
        }
        Ok(())
    }
}

fn fidelity_level_from_gate(
    gate: Mod40SourceGate,
    monitor_socket_count: usize,
    program_ram_chip_count: usize,
) -> Mod40FidelityLevel {
    let inventory_complete = gate.primary_board_population_bound
        && gate.monitor_slots_present == [true; 4]
        && monitor_socket_count == 4
        && program_ram_chip_count == 32;
    if !inventory_complete {
        return Mod40FidelityLevel::Unbound;
    }

    let core_electrical_routes = gate.cpu_reset_and_phase_timing_traced
        && gate.program_ram_write_timing_traced
        && gate.monitor_socket_map_traced
        && gate.monitor_data_transform_primary_backed;
    if !core_electrical_routes {
        return Mod40FidelityLevel::DocumentedInventory;
    }
    if !gate.board_cycle_wiring_implemented {
        return Mod40FidelityLevel::VerifiedCoreElectricalRoutes;
    }
    if !gate.monitor_media_verified {
        return Mod40FidelityLevel::VerifiedBoardCycle;
    }
    if !gate.panel_arbitration_traced {
        return Mod40FidelityLevel::ProvenanceCompleteMonitor;
    }
    if !gate.terminal_electrical_timing_traced {
        return Mod40FidelityLevel::PanelOperatedSystem;
    }
    if !gate.fpga_wrapper_implemented {
        return Mod40FidelityLevel::TerminalObservableSystem;
    }
    Mod40FidelityLevel::FpgaComparableSystem
}

impl Default for Mod40Board {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        fidelity_level_from_gate, Mod40AssemblyError, Mod40Board, Mod40FidelityLevel, Mod40TerminalEndpoint,
        ProgramStoreId, MOD40_EVIDENCE_GATE_IDS,
    };
    use crate::{
        console::ProgramMemoryMode,
        imm6_28::{Imm628Output, Imm628Read},
    };

    #[test]
    fn standard_board_inventory_uses_documented_card_owned_devices() {
        let board = Mod40Board::new();

        assert_eq!(board.imm443().monitor_socket_count(), 4);
        assert_eq!(board.imm443().data_ram_socket_count(), 4);
        assert_eq!(board.imm628().device_count(), 32);
        assert_eq!(board.source_gate().monitor_slots_present, [true; 4]);
        assert!(!board.source_gate().program_ram_card_edges_traced);
        assert!(board.source_gate().monitor_address_fanout_traced);
        assert!(board.source_gate().terminal_cable_routes_traced);
        assert!(board.source_gate().terminal_current_loop_polarity_traced);
        assert!(!board.source_gate().terminal_electrical_timing_traced);
        assert!(board.source_gate().cpu_clock_source_traced);
        assert!(board.source_gate().monitor_select_decode_inputs_traced);
        assert!(!board.source_gate().monitor_data_polarity_traced);
        assert!(!board.source_gate().monitor_socket_map_traced);
        assert!(!board.source_gate().monitor_data_transform_primary_backed);
        assert_eq!(board.source_gate().accepted_monitor_read_set_count, 0);
        assert!(!board.source_gate().cpu_reset_and_phase_timing_traced);
        assert!(!board.source_gate().program_ram_write_timing_traced);
        assert!(!board.source_gate().panel_arbitration_traced);
        assert!(!board.source_gate().monitor_media_verified);
        assert!(!board.source_gate().board_cycle_wiring_implemented);
        assert!(!board.source_gate().fpga_wrapper_implemented);
        assert!(!board.historical_execution_is_authorized());
        assert_eq!(board.blocked_evidence_gate_ids(), MOD40_EVIDENCE_GATE_IDS);
        assert!(board.evidence_gate_statuses().iter().all(|status| status.blocked));
        assert!(board
            .sources()
            .iter()
            .any(|source| source.id == "intellec4-mod40-reference-manual-98-095a"));
    }

    #[test]
    fn new_board_reports_documented_inventory_fidelity() {
        let board = Mod40Board::new();
        let snapshot = board.evidence_snapshot();

        assert_eq!(snapshot.fidelity_level, Mod40FidelityLevel::DocumentedInventory);
        assert_eq!(snapshot.monitor_socket_count, 4);
        assert_eq!(snapshot.program_ram_chip_count, 32);
        assert_eq!(snapshot.accepted_monitor_read_set_count, 0);
        assert_eq!(snapshot.blocked_gate_ids, MOD40_EVIDENCE_GATE_IDS);
        assert!(!snapshot.execution_authorized);
    }

    #[test]
    fn fidelity_requires_board_wiring_after_core_electrical_routes() {
        let board = Mod40Board::new();
        let mut gate = board.source_gate();
        gate.cpu_reset_and_phase_timing_traced = true;
        gate.program_ram_write_timing_traced = true;
        gate.monitor_socket_map_traced = true;
        gate.monitor_data_transform_primary_backed = true;

        assert_eq!(
            fidelity_level_from_gate(gate, 4, 32),
            Mod40FidelityLevel::VerifiedCoreElectricalRoutes
        );
        gate.board_cycle_wiring_implemented = true;
        assert_eq!(
            fidelity_level_from_gate(gate, 4, 32),
            Mod40FidelityLevel::VerifiedBoardCycle
        );
    }

    #[test]
    fn monitor_media_does_not_bypass_board_cycle_fidelity() {
        let board = Mod40Board::new();
        let mut gate = board.source_gate();
        gate.monitor_media_verified = true;

        assert_eq!(
            fidelity_level_from_gate(gate, 4, 32),
            Mod40FidelityLevel::DocumentedInventory
        );
    }

    #[test]
    fn panel_mode_changes_only_the_control_card_selection() {
        let mut board = Mod40Board::new();
        assert_eq!(board.imm472().selected_store(), ProgramStoreId::Monitor1702A);

        board.apply_panel_memory_mode(ProgramMemoryMode::Ram);
        assert_eq!(board.imm472().selected_store(), ProgramStoreId::ProgramRam2102);
    }

    #[test]
    fn program_ram_owns_four_kibibytes_of_eight_bit_storage() {
        let mut board = Mod40Board::new();
        board.imm628.write(0x000, 0x12);
        board.imm628.write(0x3ff, 0x34);
        board.imm628.write(0x400, 0x56);
        board.imm628.write(0xfff, 0x78);

        for (address, value) in [(0x000, 0x12), (0x3ff, 0x34), (0x400, 0x56), (0xfff, 0x78)] {
            assert_eq!(
                board.imm628.read(address, true),
                Imm628Output::Driven(Imm628Read {
                    value,
                    known_mask: 0xff,
                })
            );
        }
    }

    #[test]
    fn historical_phase_rejects_unextracted_clock_nets() {
        let board = Mod40Board::new();
        assert_eq!(
            board.validate_historical_execution(),
            Err(Mod40AssemblyError::ClockNetMapIncomplete)
        );
    }

    #[test]
    fn terminal_roles_stay_typed_and_distinct() {
        let board = Mod40Board::new();
        assert_eq!(
            board
                .imm443()
                .terminal_endpoint(Mod40TerminalEndpoint::KeyboardReceiveRom0Bit0),
            Mod40TerminalEndpoint::KeyboardReceiveRom0Bit0
        );
        assert_ne!(
            Mod40TerminalEndpoint::PrinterTransmitRam0Bit0,
            Mod40TerminalEndpoint::ReaderRunRam1Bit0
        );
    }
}
