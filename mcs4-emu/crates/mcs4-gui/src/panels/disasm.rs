//! Disassembly panel

use eframe::egui::{self, Color32};
use mcs4_chips::disasm::{CpuType, DisasmCache, Disassembler};

/// Number of disassembled lines to show before the current PC.
const WINDOW_BEFORE: usize = 32;
/// Number of disassembled lines to show after the current PC.
const WINDOW_AFTER: usize = 32;

pub struct DisasmPanel {
    disasm: Disassembler,
    cache: DisasmCache,
    pc: u16,
}

impl DisasmPanel {
    pub fn new() -> Self {
        Self {
            disasm: Disassembler::new(CpuType::I4004),
            cache: DisasmCache::new(),
            pc: 0,
        }
    }

    pub fn update(&mut self, rom: &[u8], pc: u16) {
        self.pc = pc;
        self.cache.rebuild_if_needed(&self.disasm, rom);
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.heading("Disassembly");

        let (window, pc_offset) = self.cache.window(self.pc, WINDOW_BEFORE, WINDOW_AFTER);

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, line) in window.iter().enumerate() {
                let is_current = pc_offset == Some(i);

                let bg_color = if is_current {
                    Color32::DARK_BLUE
                } else {
                    Color32::TRANSPARENT
                };

                egui::Frame::NONE.fill(bg_color).show(ui, |ui| {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_update_caches_rom() {
        let mut panel = DisasmPanel::new();
        let rom = vec![0x00, 0xD5, 0xF2]; // NOP, LDM 5, IAC
        panel.update(&rom, 0);
        assert!(!panel.cache.is_empty());
        assert_eq!(panel.cache.len(), 3);
    }

    #[test]
    fn panel_window_returns_lines_around_pc() {
        let mut panel = DisasmPanel::new();
        let rom = vec![0x00; 64];
        panel.update(&rom, 10);
        let (window, pc_idx) = panel.cache.window(10, WINDOW_BEFORE, WINDOW_AFTER);
        assert!(pc_idx.is_some());
        assert!(!window.is_empty());
    }

    #[test]
    fn panel_update_empty_rom() {
        let mut panel = DisasmPanel::new();
        panel.update(&[], 0);
        assert!(panel.cache.is_empty());
    }

    #[test]
    fn panel_cache_survives_same_rom() {
        let mut panel = DisasmPanel::new();
        let rom = vec![0x00, 0xD5, 0xF2];
        panel.update(&rom, 0);
        let len1 = panel.cache.len();

        // Second update with same ROM should not rebuild
        panel.update(&rom, 1);
        assert_eq!(panel.cache.len(), len1);
    }

    #[test]
    fn panel_cache_invalidates_on_rom_change() {
        let mut panel = DisasmPanel::new();
        let rom1 = vec![0x00, 0xD5];
        let rom2 = vec![0x00, 0xD5, 0xF2, 0x00];

        panel.update(&rom1, 0);
        assert_eq!(panel.cache.len(), 2);

        panel.update(&rom2, 0);
        assert_eq!(panel.cache.len(), 4);
    }

    #[test]
    fn panel_pc_tracking() {
        let mut panel = DisasmPanel::new();
        let rom = vec![0x00; 8];

        panel.update(&rom, 0);
        assert_eq!(panel.pc, 0);

        panel.update(&rom, 5);
        assert_eq!(panel.pc, 5);
    }
}
