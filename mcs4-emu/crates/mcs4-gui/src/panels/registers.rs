//! Register display panel for 4004/4040 CPUs

use eframe::egui::{self, Color32};
use mcs4_runtime::CpuSnapshot;

/// CPU type for register panel layout
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuMode {
    /// 4004: 16 registers, 3-level stack
    I4004,
    /// 4040: 24 registers (bank 0 + bank 1), 7-level stack
    I4040,
}

/// Register display panel
pub struct RegisterPanel {
    mode: CpuMode,
    snapshot: CpuSnapshot,
    prev_snapshot: CpuSnapshot,
}

impl RegisterPanel {
    pub fn new(mode: CpuMode) -> Self {
        Self {
            mode,
            snapshot: CpuSnapshot::default(),
            prev_snapshot: CpuSnapshot::default(),
        }
    }

    /// Update with new CPU state. Tracks changes for highlighting.
    pub fn update(&mut self, snapshot: CpuSnapshot) {
        self.prev_snapshot = std::mem::replace(&mut self.snapshot, snapshot);
    }

    /// Current CPU mode
    pub fn mode(&self) -> CpuMode {
        self.mode
    }

    /// Set CPU mode
    pub fn set_mode(&mut self, mode: CpuMode) {
        self.mode = mode;
    }

    /// Current snapshot (for testing)
    pub fn snapshot(&self) -> &CpuSnapshot {
        &self.snapshot
    }

    /// Check if a register value changed since last update
    pub fn register_changed(&self, idx: usize) -> bool {
        let cur = self.snapshot.registers.get(idx).copied().unwrap_or(0);
        let prev = self.prev_snapshot.registers.get(idx).copied().unwrap_or(0);
        cur != prev
    }

    /// Check if accumulator changed since last update
    pub fn accumulator_changed(&self) -> bool {
        self.snapshot.accumulator != self.prev_snapshot.accumulator
    }

    /// Check if carry changed since last update
    pub fn carry_changed(&self) -> bool {
        self.snapshot.carry != self.prev_snapshot.carry
    }

    /// Check if PC changed since last update
    pub fn pc_changed(&self) -> bool {
        self.snapshot.pc != self.prev_snapshot.pc
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let title = match self.mode {
            CpuMode::I4004 => "4004 Registers",
            CpuMode::I4040 => "4040 Registers",
        };
        ui.heading(title);
        ui.separator();

        // Accumulator + carry
        ui.horizontal(|ui| {
            let acc_color = if self.accumulator_changed() {
                Color32::LIGHT_RED
            } else {
                Color32::WHITE
            };
            ui.label(egui::RichText::new(format!("ACC: {:X}", self.snapshot.accumulator)).color(acc_color));

            let carry_color = if self.carry_changed() {
                Color32::LIGHT_RED
            } else {
                Color32::WHITE
            };
            let carry_str = if self.snapshot.carry { "1" } else { "0" };
            ui.label(egui::RichText::new(format!("CY: {}", carry_str)).color(carry_color));
        });

        // PC
        let pc_color = if self.pc_changed() {
            Color32::LIGHT_RED
        } else {
            Color32::WHITE
        };
        ui.label(egui::RichText::new(format!("PC: {:03X}", self.snapshot.pc)).color(pc_color));

        // Status flags
        if self.mode == CpuMode::I4040 {
            ui.horizontal(|ui| {
                if self.snapshot.halted {
                    ui.label(egui::RichText::new("HALTED").color(Color32::YELLOW));
                }
                if self.snapshot.interrupt_enabled {
                    ui.label(egui::RichText::new("IE").color(Color32::GREEN));
                }
            });
        }

        ui.separator();

        // Index registers in pairs
        let reg_count = self.snapshot.registers.len().min(24);
        let pair_count = reg_count / 2;

        egui::Grid::new("register_grid").show(ui, |ui| {
            for pair in 0..pair_count {
                let r0_idx = pair * 2;
                let r1_idx = pair * 2 + 1;

                let r0 = self.snapshot.registers.get(r0_idx).copied().unwrap_or(0);
                let r1 = self.snapshot.registers.get(r1_idx).copied().unwrap_or(0);

                let r0_color = if self.register_changed(r0_idx) {
                    Color32::LIGHT_RED
                } else {
                    Color32::WHITE
                };
                let r1_color = if self.register_changed(r1_idx) {
                    Color32::LIGHT_RED
                } else {
                    Color32::WHITE
                };

                ui.label(egui::RichText::new(format!("P{}", pair)).color(Color32::GRAY));
                ui.label(egui::RichText::new(format!("R{:X}:{:X}", r0_idx, r0)).color(r0_color));
                ui.label(egui::RichText::new(format!("R{:X}:{:X}", r1_idx, r1)).color(r1_color));
                ui.end_row();
            }
        });

        // Stack
        if !self.snapshot.stack.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new(format!("SP: {}", self.snapshot.sp)).color(Color32::GRAY));
            for (i, &addr) in self.snapshot.stack.iter().enumerate() {
                let marker = if i == self.snapshot.sp as usize { ">" } else { " " };
                ui.label(egui::RichText::new(format!("{} S{}: {:03X}", marker, i, addr)).color(Color32::GRAY));
            }
        }
    }
}

impl Default for RegisterPanel {
    fn default() -> Self {
        Self::new(CpuMode::I4004)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_panel_has_empty_snapshot() {
        let panel = RegisterPanel::new(CpuMode::I4004);
        assert_eq!(panel.snapshot().registers.len(), 0);
        assert_eq!(panel.snapshot().accumulator, 0);
        assert!(!panel.snapshot().carry);
        assert_eq!(panel.snapshot().pc, 0);
    }

    #[test]
    fn update_stores_snapshot() {
        let mut panel = RegisterPanel::new(CpuMode::I4004);
        let mut regs = [0u8; 16];
        regs[5] = 0xA;
        let snap = CpuSnapshot::from_4004(&regs, 0x7, true, 0x123);
        panel.update(snap);

        assert_eq!(panel.snapshot().registers[5], 0xA);
        assert_eq!(panel.snapshot().accumulator, 0x7);
        assert!(panel.snapshot().carry);
        assert_eq!(panel.snapshot().pc, 0x123);
    }

    #[test]
    fn change_detection_registers() {
        let mut panel = RegisterPanel::new(CpuMode::I4004);

        let regs1 = [0u8; 16];
        panel.update(CpuSnapshot::from_4004(&regs1, 0, false, 0));

        let mut regs2 = [0u8; 16];
        regs2[3] = 0x5;
        panel.update(CpuSnapshot::from_4004(&regs2, 0, false, 0));

        assert!(!panel.register_changed(0));
        assert!(panel.register_changed(3));
        assert!(!panel.register_changed(15));
    }

    #[test]
    fn change_detection_accumulator_carry_pc() {
        let mut panel = RegisterPanel::new(CpuMode::I4004);
        let regs = [0u8; 16];

        panel.update(CpuSnapshot::from_4004(&regs, 0, false, 0));
        panel.update(CpuSnapshot::from_4004(&regs, 5, true, 0x100));

        assert!(panel.accumulator_changed());
        assert!(panel.carry_changed());
        assert!(panel.pc_changed());
    }

    #[test]
    fn no_change_when_same() {
        let mut panel = RegisterPanel::new(CpuMode::I4004);
        let regs = [0u8; 16];

        panel.update(CpuSnapshot::from_4004(&regs, 3, true, 0x10));
        panel.update(CpuSnapshot::from_4004(&regs, 3, true, 0x10));

        assert!(!panel.accumulator_changed());
        assert!(!panel.carry_changed());
        assert!(!panel.pc_changed());
    }

    #[test]
    fn mode_switching() {
        let mut panel = RegisterPanel::new(CpuMode::I4004);
        assert_eq!(panel.mode(), CpuMode::I4004);

        panel.set_mode(CpuMode::I4040);
        assert_eq!(panel.mode(), CpuMode::I4040);
    }

    #[test]
    fn default_panel_is_4004() {
        let panel = RegisterPanel::default();
        assert_eq!(panel.mode(), CpuMode::I4004);
    }
}
