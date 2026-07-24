//! Memory display panel for ROM and RAM viewing

use eframe::egui::{self, Color32};
use mcs4_runtime::{MemoryRegion, MemorySnapshot};

/// Hex dump width (bytes per row)
const BYTES_PER_ROW: usize = 16;

/// Memory display panel
pub struct MemoryPanel {
    snapshot: MemorySnapshot,
    prev_data: Vec<u8>,
    selected_region: MemoryRegion,
}

impl MemoryPanel {
    pub fn new() -> Self {
        Self {
            snapshot: MemorySnapshot {
                region: MemoryRegion::Rom,
                base_addr: 0,
                data: Vec::new(),
            },
            prev_data: Vec::new(),
            selected_region: MemoryRegion::Rom,
        }
    }

    pub fn update(&mut self, snapshot: MemorySnapshot) {
        self.prev_data = std::mem::take(&mut self.snapshot.data);
        self.snapshot = snapshot;
    }

    pub fn snapshot(&self) -> &MemorySnapshot {
        &self.snapshot
    }

    pub fn selected_region(&self) -> MemoryRegion {
        self.selected_region
    }

    pub fn set_region(&mut self, region: MemoryRegion) {
        self.selected_region = region;
    }

    /// Check if byte at offset changed since last update
    pub fn byte_changed(&self, offset: usize) -> bool {
        let cur = self.snapshot.data.get(offset).copied().unwrap_or(0);
        let prev = self.prev_data.get(offset).copied().unwrap_or(0);
        cur != prev
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let title = match self.snapshot.region {
            MemoryRegion::Rom => "ROM",
            MemoryRegion::Ram => "RAM",
        };
        ui.heading(title);

        // Region selector
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.selected_region == MemoryRegion::Rom, "ROM")
                .clicked()
            {
                self.selected_region = MemoryRegion::Rom;
            }
            if ui
                .selectable_label(self.selected_region == MemoryRegion::Ram, "RAM")
                .clicked()
            {
                self.selected_region = MemoryRegion::Ram;
            }
        });

        ui.separator();

        if self.snapshot.data.is_empty() {
            ui.label("No data");
            return;
        }

        // Hex dump
        egui::ScrollArea::vertical().show(ui, |ui| {
            let row_count = self.snapshot.data.len().div_ceil(BYTES_PER_ROW);

            for row in 0..row_count {
                let addr = self.snapshot.base_addr + (row * BYTES_PER_ROW) as u16;
                let offset_start = row * BYTES_PER_ROW;

                ui.horizontal(|ui| {
                    // Address label
                    ui.label(
                        egui::RichText::new(format!("{:04X}:", addr))
                            .color(Color32::GRAY)
                            .monospace(),
                    );

                    // Hex bytes
                    for col in 0..BYTES_PER_ROW {
                        let offset = offset_start + col;
                        if offset < self.snapshot.data.len() {
                            let byte = self.snapshot.data[offset];
                            let color = if self.byte_changed(offset) {
                                Color32::LIGHT_RED
                            } else {
                                Color32::WHITE
                            };
                            ui.label(egui::RichText::new(format!("{:02X}", byte)).color(color).monospace());
                        }
                    }
                });
            }
        });
    }
}

impl Default for MemoryPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_panel_is_empty() {
        let panel = MemoryPanel::new();
        assert!(panel.snapshot().is_empty());
        assert_eq!(panel.selected_region(), MemoryRegion::Rom);
    }

    #[test]
    fn update_stores_snapshot() {
        let mut panel = MemoryPanel::new();
        let snap = MemorySnapshot::from_rom(&[0x00, 0xD5, 0xF2]);
        panel.update(snap);

        assert_eq!(panel.snapshot().len(), 3);
        assert_eq!(panel.snapshot().byte_at(0), Some(0x00));
        assert_eq!(panel.snapshot().byte_at(1), Some(0xD5));
        assert_eq!(panel.snapshot().byte_at(2), Some(0xF2));
    }

    #[test]
    fn byte_change_detection() {
        let mut panel = MemoryPanel::new();

        panel.update(MemorySnapshot::from_rom(&[0x00, 0x01, 0x02]));
        panel.update(MemorySnapshot::from_rom(&[0x00, 0xFF, 0x02]));

        assert!(!panel.byte_changed(0));
        assert!(panel.byte_changed(1));
        assert!(!panel.byte_changed(2));
    }

    #[test]
    fn region_switching() {
        let mut panel = MemoryPanel::new();
        assert_eq!(panel.selected_region(), MemoryRegion::Rom);

        panel.set_region(MemoryRegion::Ram);
        assert_eq!(panel.selected_region(), MemoryRegion::Ram);
    }

    #[test]
    fn default_is_new() {
        let panel = MemoryPanel::default();
        assert!(panel.snapshot().is_empty());
    }
}
