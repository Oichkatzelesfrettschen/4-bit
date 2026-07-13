//! Source-gated Intellec console and ASR-33 workspace.

use eframe::egui;
use mcs4_intellec::{
    IntellecEvent, IntellecMachine, IntellecModel, IntellecProfile, PanelControl, PanelInput, ProgramMemoryMode,
    ResetScope,
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

#[cfg(test)]
mod tests {
    use mcs4_intellec::IntellecModel;

    use super::IntellecWorkspace;

    #[test]
    fn workspace_selects_the_original_profile() {
        assert_eq!(IntellecWorkspace::new().model(), IntellecModel::Intellec4);
    }
}
