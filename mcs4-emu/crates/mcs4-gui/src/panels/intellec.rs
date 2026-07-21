//! Source-gated Intellec console and ASR-33 workspace.

use eframe::egui;
use mcs4_intellec::{
    IntellecEvent, IntellecMachine, IntellecModel, IntellecProfile, Mod40Board, Mod40EvidenceSnapshot, PanelControl,
    PanelInput, ProgramMemoryMode, ResetScope,
};
use mcs4_system::Mcs4System;

/// Interactive source-gated Intellec 4 visual workspace.
pub struct IntellecWorkspace {
    machine: IntellecMachine<Mcs4System>,
    terminal_entry: String,
    paper: String,
    punch: Vec<u8>,
    fault: Option<String>,
}

impl IntellecWorkspace {
    /// Construct the original 4004 profile and expose its evidence gate.
    pub fn new() -> Self {
        Self {
            machine: IntellecMachine::new(Mcs4System::standard(), IntellecProfile::intellec4()),
            terminal_entry: String::new(),
            paper: String::new(),
            punch: Vec::new(),
            fault: None,
        }
    }

    /// Render console controls, lamps, and the host visual ASR-33 terminal.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.heading("Intellec 4 console");
        ui.label("4004 profile; terminal appears as an external paper teletype, not a CRT.");

        match self.machine.profile().validate_boot_evidence() {
            Ok(()) => ui.colored_label(egui::Color32::GREEN, "source gate permits monitor boot"),
            Err(error) => ui.colored_label(
                egui::Color32::YELLOW,
                format!("historical monitor boot blocked: {}", error.missing.join(", ")),
            ),
        };

        let snapshot = self.machine.panel_snapshot();
        let mut address_data = snapshot.address_data_switches;
        let mut write_data = snapshot.write_data_switches;
        ui.horizontal(|ui| {
            ui.label("ADDRESS/DATA");
            if ui
                .add(
                    egui::DragValue::new(&mut address_data)
                        .range(0..=0x0fff)
                        .hexadecimal(3, false, true),
                )
                .changed()
            {
                self.machine
                    .apply_event(IntellecEvent::Panel(PanelInput::SetAddressData(address_data)));
            }
            ui.label("WRITE");
            if ui
                .add(
                    egui::DragValue::new(&mut write_data)
                        .range(0..=0xff)
                        .hexadecimal(2, false, true),
                )
                .changed()
            {
                self.machine
                    .apply_event(IntellecEvent::Panel(PanelInput::SetWriteData(write_data)));
            }
        });

        ui.horizontal(|ui| {
            for (label, mode) in [
                ("MON", ProgramMemoryMode::Monitor),
                ("RAM", ProgramMemoryMode::Ram),
                ("PROM", ProgramMemoryMode::Prom),
            ] {
                if ui
                    .selectable_label(snapshot.program_memory_mode == mode, label)
                    .clicked()
                {
                    self.machine
                        .apply_event(IntellecEvent::Panel(PanelInput::SetProgramMemoryMode(mode)));
                }
            }
            if ui.button("RUN").clicked() {
                self.machine
                    .apply_event(IntellecEvent::Panel(PanelInput::Control(PanelControl::Run)));
            }
            if ui.button("ONE CYCLE").clicked() {
                self.machine
                    .apply_event(IntellecEvent::Panel(PanelInput::Control(PanelControl::SingleStep)));
            }
            if ui.button("STOP").clicked() {
                self.machine
                    .apply_event(IntellecEvent::Panel(PanelInput::Control(PanelControl::Stop)));
            }
        });

        ui.horizontal(|ui| {
            if ui.button("TEST PULSE").clicked() {
                self.machine
                    .apply_event(IntellecEvent::Panel(PanelInput::Control(PanelControl::TestOneShot)));
            }
            if ui.button("RESET CPU").clicked() {
                self.machine
                    .apply_event(IntellecEvent::Panel(PanelInput::Control(PanelControl::Reset(
                        ResetScope::Cpu,
                    ))));
            }
            if ui.button("RESET SYSTEM").clicked() {
                self.machine
                    .apply_event(IntellecEvent::Panel(PanelInput::Control(PanelControl::Reset(
                        ResetScope::System,
                    ))));
            }
            if ui
                .selectable_label(snapshot.console_memory_access_enabled, "CMA ENABLE")
                .clicked()
            {
                self.machine.apply_event(IntellecEvent::Panel(PanelInput::Control(
                    PanelControl::ConsoleMemoryAccess(!snapshot.console_memory_access_enabled),
                )));
            }
            if ui.button("CMA WRITE").clicked() {
                self.machine.apply_event(IntellecEvent::Panel(PanelInput::Control(
                    PanelControl::ConsoleMemoryWrite,
                )));
            }
        });

        ui.separator();
        let snapshot = self.machine.panel_snapshot();
        ui.monospace(format!(
            "ADDR {:03X} EXEC {:X} INSTR {:02X} ROM {:?} RAM {:?} RUN {}",
            snapshot.lamps.address,
            snapshot.lamps.execution,
            snapshot.lamps.instruction,
            snapshot.lamps.active_rom_bank,
            snapshot.lamps.active_ram_bank,
            snapshot.lamps.cpu_running
        ));

        ui.separator();
        ui.heading("ASR-33 terminal");
        ui.horizontal(|ui| {
            let response =
                ui.add(egui::TextEdit::singleline(&mut self.terminal_entry).hint_text("ASCII keyboard input"));
            if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                self.enqueue_terminal_text();
            }
            if ui.button("SEND").clicked() {
                self.enqueue_terminal_text();
            }
            if ui.button("PUNCH ON").clicked() {
                self.machine.apply_event(IntellecEvent::PunchEnabled(true));
            }
            if ui.button("PUNCH OFF").clicked() {
                self.machine.apply_event(IntellecEvent::PunchEnabled(false));
            }
        });
        egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
            ui.monospace(if self.paper.is_empty() {
                "[printer idle]"
            } else {
                &self.paper
            });
        });
        ui.monospace(format!("punch bytes: {}", self.punch.len()));

        if ui.button("STEP SOURCE-GATED MACHINE").clicked() {
            if let Err(error) = self.machine.profile().validate_boot_evidence() {
                self.fault = Some(format!(
                    "cannot execute historical profile: {}",
                    error.missing.join(", ")
                ));
            } else if let Err(error) = self.machine.step_phase() {
                self.fault = Some(format!("machine step: {error:?}"));
            } else {
                self.collect_terminal_output();
            }
        }
        if let Some(fault) = self.fault.as_deref() {
            ui.colored_label(egui::Color32::RED, fault);
        }
    }

    fn enqueue_terminal_text(&mut self) {
        let text = std::mem::take(&mut self.terminal_entry);
        for byte in text.bytes() {
            if byte.is_ascii() {
                self.machine.apply_event(IntellecEvent::TerminalKey(byte));
            }
        }
    }

    fn collect_terminal_output(&mut self) {
        let printed = self.machine.drain_printed_terminal_bytes();
        self.paper.push_str(&String::from_utf8_lossy(&printed));
        self.punch.extend(self.machine.drain_punched_terminal_bytes());
    }

    /// Return the selected model identity for test and status panels.
    pub const fn model(&self) -> IntellecModel {
        self.machine.profile().model()
    }
}

impl Default for IntellecWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

/// Read-only MOD 40 inventory, fidelity, and documentary-gate workspace.
pub struct Mod40EvidenceWorkspace {
    board: Mod40Board,
}

impl Mod40EvidenceWorkspace {
    /// Construct the documented card inventory without an execution surface.
    pub fn new() -> Self {
        Self {
            board: Mod40Board::new(),
        }
    }

    /// Return immutable state used by the dashboard and tests.
    pub fn snapshot(&self) -> Mod40EvidenceSnapshot {
        self.board.evidence_snapshot()
    }

    /// Render source-gate state without exposing run, step, or memory mutation.
    pub fn show(&self, ui: &mut egui::Ui) {
        let snapshot = self.snapshot();
        ui.heading("Intellec 4/MOD 40 evidence");
        ui.label("4040 profile; source-gated board composition remains non-executable.");
        ui.monospace(format!(
            "imm4-43 MON sockets {} | imm6-28 2102 devices {} | store {:?}",
            snapshot.monitor_socket_count, snapshot.program_ram_chip_count, snapshot.selected_store
        ));
        ui.label(format!("Fidelity: {}", snapshot.fidelity_level.label()));
        ui.label(format!(
            "Accepted independent monitor read sets: {}",
            snapshot.accepted_monitor_read_set_count
        ));

        ui.separator();
        ui.label("Documentary gates");
        for gate in snapshot.gates {
            let state = if gate.blocked { "BLOCKED" } else { "CLOSED" };
            let color = if gate.blocked {
                egui::Color32::YELLOW
            } else {
                egui::Color32::GREEN
            };
            ui.colored_label(color, format!("{}: {state}", gate.id));
        }

        ui.separator();
        ui.label(format!(
            "Board-cycle wiring: {}",
            if snapshot.board_cycle_wiring_implemented {
                "implemented"
            } else {
                "blocked"
            }
        ));
        ui.label(format!(
            "FPGA wrapper: {}",
            if snapshot.fpga_wrapper_implemented {
                "implemented"
            } else {
                "blocked"
            }
        ));
        ui.add_enabled(false, egui::Button::new("RUN MOD 40"))
            .on_disabled_hover_text(snapshot.blocked_gate_ids.join(", "));
    }
}

impl Default for Mod40EvidenceWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use mcs4_intellec::{IntellecModel, Mod40FidelityLevel, MOD40_EVIDENCE_GATE_IDS};

    use super::{IntellecWorkspace, Mod40EvidenceWorkspace};

    #[test]
    fn workspace_selects_the_original_profile() {
        assert_eq!(IntellecWorkspace::new().model(), IntellecModel::Intellec4);
    }

    #[test]
    fn mod40_dashboard_owns_a_non_executable_board() {
        let snapshot = Mod40EvidenceWorkspace::new().snapshot();

        assert_eq!(snapshot.fidelity_level, Mod40FidelityLevel::DocumentedInventory);
        assert!(!snapshot.execution_authorized);
        assert_eq!(snapshot.blocked_gate_ids, MOD40_EVIDENCE_GATE_IDS);
    }

    #[test]
    fn mod40_dashboard_exposes_all_canonical_gate_ids() {
        let snapshot = Mod40EvidenceWorkspace::new().snapshot();
        let gate_ids = snapshot.gates.map(|gate| gate.id);

        assert_eq!(gate_ids, MOD40_EVIDENCE_GATE_IDS);
    }
}
