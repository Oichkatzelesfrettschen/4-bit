//! Source-bound Intellec 4/MOD 40 card composition.
//!
//! This module owns the documented imm4-43, imm4-72, and imm6-28 card
//! inventory. It remains deliberately non-executable until the retained
//! schematics establish the clock, control, program-RAM, panel, and terminal
//! electrical nets required for a historical phase.

use mcs4_chips::{i1702::I1702A, i4002::I4002, i4040::I4040, i4289::I4289};

use crate::{
    console::ProgramMemoryMode,
    imm6_28::Imm628,
    mod40_routes::{
        accepted_monitor_read_set_count, cpu_clock_source_is_traced, cpu_reset_and_phase_timing_is_traced,
        monitor_address_fanout_is_traced, monitor_data_polarity_is_traced, monitor_data_transform_is_primary_backed,
        monitor_select_decode_inputs_are_traced, monitor_socket_map_is_traced, panel_arbitration_is_traced,
        program_ram_card_edge_is_complete, program_ram_write_timing_is_traced, terminal_cable_routes_are_traced,
        terminal_current_loop_polarity_is_traced, terminal_electrical_timing_is_traced,
    },
    profile::{SourceReference, MOD40_SOURCES},
};

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
            monitor_media_verified: accepted_monitor_read_set_count >= 2
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

impl Default for Mod40Board {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Mod40AssemblyError, Mod40Board, Mod40TerminalEndpoint, ProgramStoreId};
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
        assert!(!board.historical_execution_is_authorized());
        assert!(board
            .sources()
            .iter()
            .any(|source| source.id == "intellec4-mod40-reference-manual-98-095a"));
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
