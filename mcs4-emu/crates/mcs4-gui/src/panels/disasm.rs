//! Disassembly panel

use eframe::egui::{self, Color32};
use mcs4_chips::disasm::{CpuType, DisasmLine, Disassembler};

pub struct DisasmPanel {
    lines: Vec<DisasmLine>,
    pc: u16,
}

impl DisasmPanel {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            pc: 0,
        }
    }

    pub fn update(&mut self, rom: &[u8], pc: u16) {
        // Only update if PC changed significantly or rom changed (simplified)
        // For now, simple full refresh on demand
        self.pc = pc;
        let disasm = Disassembler::new(CpuType::I4004);
        // Disassemble a window around PC
        // Finding the start of an instruction is hard in variable length ISA without scanning from 0
        // We scan from 0 for now (fast enough for small 4KB ROM)
        let end_addr = if rom.is_empty() { 0 } else { rom.len() as u16 - 1 };
        self.lines = disasm.disasm_range(rom, 0, end_addr);
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.heading("Disassembly");

        egui::ScrollArea::vertical().show(ui, |ui| {
            for line in &self.lines {
                let is_current = line.address == self.pc;

                let bg_color = if is_current {
                    Color32::DARK_BLUE
                } else {
                    Color32::TRANSPARENT
                };

                egui::Frame::none().fill(bg_color).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{:03X}:", line.address)).color(Color32::GRAY));
                        ui.label(egui::RichText::new(&line.mnemonic).color(Color32::YELLOW).strong());
                        ui.label(egui::RichText::new(&line.operands).color(Color32::WHITE));
                        if let Some(comment) = &line.comment {
                            ui.label(egui::RichText::new(format!("; {}", comment)).color(Color32::GREEN));
                        }
                    });
                });

                if is_current {
                    ui.scroll_to_cursor(Some(egui::Align::Center));
                }
            }
        });
    }
}

impl Default for DisasmPanel {
    fn default() -> Self {
        Self::new()
    }
}
