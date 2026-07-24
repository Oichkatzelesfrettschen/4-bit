//! Source-gated Intellec console and ASR-33 workspace.
//!
//! The console is a view over the one shared worker machine. It renders the
//! console snapshot the worker publishes and sends panel commands back; it never
//! owns a machine of its own. The debugger's Run/Step are authoritative for
//! execution, so this surface observes the shared machine and its evidence gate
//! governs only panel-originated stepping.

use eframe::egui;
use mcs4_intellec::{
    IntellecModel, Mod40Board, Mod40EvidenceSnapshot, PanelControl, PanelInput, ProgramMemoryMode, ResetScope,
};

use crate::session::{IntellecConsoleSnapshot, SimulationCommand, SimulationSession};

/// Interactive source-gated Intellec 4 view over the shared worker machine.
#[derive(Default)]
pub struct IntellecWorkspace {
    console: Option<IntellecConsoleSnapshot>,
    terminal_entry: String,
}

impl IntellecWorkspace {
    /// Construct an empty view; the worker supplies console state on first frame.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store the latest console snapshot published by the worker.
    pub fn set_console(&mut self, snapshot: IntellecConsoleSnapshot) {
        self.console = Some(snapshot);
    }

    /// Return the observed model, or the original profile before the first snapshot.
    pub fn model(&self) -> IntellecModel {
        self.console
            .as_ref()
            .map_or(IntellecModel::Intellec4, |console| console.model)
    }

    /// Render console controls and lamps, sending panel commands to the worker.
    pub fn show(&mut self, ui: &mut egui::Ui, simulation: &SimulationSession) {
        ui.heading("Intellec 4 console");
        ui.label("Observing the shared machine. The debugger drives execution; this gate governs panel steps.");

        let Some(console) = self.console.clone() else {
            ui.label("connecting to the shared machine...");
            return;
        };

        if console.boot_gate_ok {
            ui.colored_label(egui::Color32::GREEN, "source gate permits monitor boot");
        } else {
            ui.colored_label(
                egui::Color32::YELLOW,
                format!(
                    "historical monitor boot blocked: {}",
                    console.boot_gate_missing.join(", ")
                ),
            );
        }

        let panel = console.panel;
        let mut address_data = panel.address_data_switches;
        let mut write_data = panel.write_data_switches;
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
                self.send(simulation, PanelInput::SetAddressData(address_data));
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
                self.send(simulation, PanelInput::SetWriteData(write_data));
            }
        });

        ui.horizontal(|ui| {
            for (label, mode) in [
                ("MON", ProgramMemoryMode::Monitor),
                ("RAM", ProgramMemoryMode::Ram),
                ("PROM", ProgramMemoryMode::Prom),
            ] {
                if ui.selectable_label(panel.program_memory_mode == mode, label).clicked() {
                    self.send(simulation, PanelInput::SetProgramMemoryMode(mode));
                }
            }
            if ui.button("RUN").clicked() {
                self.send(simulation, PanelInput::Control(PanelControl::Run));
            }
            if ui.button("ONE CYCLE").clicked() {
                let _ = simulation.send(SimulationCommand::IntellecStep { phases: 8 });
            }
            if ui.button("STOP").clicked() {
                self.send(simulation, PanelInput::Control(PanelControl::Stop));
            }
        });

        ui.horizontal(|ui| {
            if ui.button("TEST PULSE").clicked() {
                self.send(simulation, PanelInput::Control(PanelControl::TestOneShot));
            }
            if ui.button("RESET CPU").clicked() {
                self.send(simulation, PanelInput::Control(PanelControl::Reset(ResetScope::Cpu)));
            }
            if ui.button("RESET SYSTEM").clicked() {
                self.send(simulation, PanelInput::Control(PanelControl::Reset(ResetScope::System)));
            }
            if ui
                .selectable_label(panel.console_memory_access_enabled, "CMA ENABLE")
                .clicked()
            {
                self.send(
                    simulation,
                    PanelInput::Control(PanelControl::ConsoleMemoryAccess(!panel.console_memory_access_enabled)),
                );
            }
            if ui.button("CMA WRITE").clicked() {
                self.send(simulation, PanelInput::Control(PanelControl::ConsoleMemoryWrite));
            }
        });

        ui.separator();
        ui.monospace(format!(
            "ADDR {:03X} EXEC {:X} INSTR {:02X} ROM {:?} RAM {:?} RUN {}",
            panel.lamps.address,
            panel.lamps.execution,
            panel.lamps.instruction,
            panel.lamps.active_rom_bank,
            panel.lamps.active_ram_bank,
            panel.lamps.cpu_running
        ));

        if ui.button("STEP (gated)").clicked() {
            let _ = simulation.send(SimulationCommand::IntellecStep { phases: 1 });
        }
        if let Some(fault) = console.panel_step_fault.as_deref() {
            ui.colored_label(egui::Color32::RED, fault);
        }

        ui.separator();
        ui.heading("ASR-33 terminal");
        ui.horizontal(|ui| {
            let response =
                ui.add(egui::TextEdit::singleline(&mut self.terminal_entry).hint_text("ASCII keyboard input"));
            let submit = response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter))
                || ui.button("SEND").clicked();
            if submit {
                self.enqueue_terminal_text(simulation);
            }
            if ui.button("PUNCH ON").clicked() {
                let _ = simulation.send(SimulationCommand::IntellecPunch(true));
            }
            if ui.button("PUNCH OFF").clicked() {
                let _ = simulation.send(SimulationCommand::IntellecPunch(false));
            }
        });
        egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
            ui.monospace(if console.paper.is_empty() {
                "[printer idle]"
            } else {
                console.paper.as_str()
            });
        });
        ui.monospace(format!("punch bytes: {}", console.punch_len));
    }

    fn send(&self, simulation: &SimulationSession, input: PanelInput) {
        let _ = simulation.send(SimulationCommand::IntellecPanelInput(input));
    }

    fn enqueue_terminal_text(&mut self, simulation: &SimulationSession) {
        let text = std::mem::take(&mut self.terminal_entry);
        for byte in text.bytes() {
            if byte.is_ascii() {
                let _ = simulation.send(SimulationCommand::IntellecTerminalKey(byte));
            }
        }
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
    fn workspace_defaults_to_the_original_profile() {
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
