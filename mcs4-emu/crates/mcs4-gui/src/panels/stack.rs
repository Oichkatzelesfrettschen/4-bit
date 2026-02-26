//! Stack display panel for 4004/4040 call stack

use eframe::egui::{self, Color32};

use super::registers::CpuMode;

/// Maximum stack depth for each CPU mode
const STACK_DEPTH_4004: usize = 3;
const STACK_DEPTH_4040: usize = 7;

/// Snapshot of the call stack for display
#[derive(Clone, Debug, Default)]
pub struct StackSnapshot {
    /// Stack entries (address values)
    pub entries: Vec<u16>,
    /// Current stack pointer
    pub sp: u8,
}

impl StackSnapshot {
    /// Create a stack snapshot from raw stack array and pointer
    pub fn new(entries: &[u16], sp: u8) -> Self {
        Self {
            entries: entries.to_vec(),
            sp,
        }
    }

    /// Number of stack slots
    pub fn depth(&self) -> usize {
        self.entries.len()
    }

    /// True if the stack has no entries
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the entry at a given level, if it exists
    pub fn entry_at(&self, level: usize) -> Option<u16> {
        self.entries.get(level).copied()
    }
}

/// Stack display panel
pub struct StackPanel {
    mode: CpuMode,
    snapshot: StackSnapshot,
    prev_snapshot: StackSnapshot,
}

impl StackPanel {
    pub fn new(mode: CpuMode) -> Self {
        Self {
            mode,
            snapshot: StackSnapshot::default(),
            prev_snapshot: StackSnapshot::default(),
        }
    }

    /// Update with new stack state
    pub fn update(&mut self, snapshot: StackSnapshot) {
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

    /// Current snapshot
    pub fn snapshot(&self) -> &StackSnapshot {
        &self.snapshot
    }

    /// Maximum stack depth for current mode
    pub fn max_depth(&self) -> usize {
        match self.mode {
            CpuMode::I4004 => STACK_DEPTH_4004,
            CpuMode::I4040 => STACK_DEPTH_4040,
        }
    }

    /// Check if an entry changed since last update
    pub fn entry_changed(&self, level: usize) -> bool {
        let cur = self.snapshot.entries.get(level).copied().unwrap_or(0);
        let prev = self.prev_snapshot.entries.get(level).copied().unwrap_or(0);
        cur != prev
    }

    /// Check if SP changed since last update
    pub fn sp_changed(&self) -> bool {
        self.snapshot.sp != self.prev_snapshot.sp
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let title = match self.mode {
            CpuMode::I4004 => "4004 Stack (3-level)",
            CpuMode::I4040 => "4040 Stack (7-level)",
        };
        ui.heading(title);
        ui.separator();

        // Stack pointer
        let sp_color = if self.sp_changed() {
            Color32::LIGHT_RED
        } else {
            Color32::WHITE
        };
        ui.label(egui::RichText::new(format!("SP: {}", self.snapshot.sp)).color(sp_color));

        ui.separator();

        if self.snapshot.is_empty() {
            ui.label("Stack empty");
            return;
        }

        // Show stack entries from top (SP) down
        let max = self.max_depth().min(self.snapshot.depth());
        egui::Grid::new("stack_grid").show(ui, |ui| {
            for level in 0..max {
                let is_top = level == self.snapshot.sp as usize;
                let addr = self.snapshot.entries.get(level).copied().unwrap_or(0);

                // Marker for current stack top
                let marker = if is_top { ">" } else { " " };
                ui.label(egui::RichText::new(marker).color(Color32::YELLOW).monospace());

                // Level label
                ui.label(
                    egui::RichText::new(format!("S{}:", level))
                        .color(Color32::GRAY)
                        .monospace(),
                );

                // Address value with change highlighting
                let addr_color = if self.entry_changed(level) {
                    Color32::LIGHT_RED
                } else if is_top {
                    Color32::LIGHT_BLUE
                } else {
                    Color32::WHITE
                };
                ui.label(
                    egui::RichText::new(format!("{:03X}", addr))
                        .color(addr_color)
                        .monospace(),
                );
                ui.end_row();
            }
        });

        // Usage indicator
        ui.separator();
        let used = self.snapshot.sp as usize;
        let total = self.max_depth();
        let usage_color = if used >= total {
            Color32::RED
        } else if used >= total - 1 {
            Color32::YELLOW
        } else {
            Color32::GRAY
        };
        ui.label(egui::RichText::new(format!("Usage: {}/{}", used, total)).color(usage_color));
    }
}

impl Default for StackPanel {
    fn default() -> Self {
        Self::new(CpuMode::I4004)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_panel_has_empty_stack() {
        let panel = StackPanel::new(CpuMode::I4004);
        assert!(panel.snapshot().is_empty());
        assert_eq!(panel.snapshot().sp, 0);
        assert_eq!(panel.max_depth(), 3);
    }

    #[test]
    fn i4040_mode_has_7_level_stack() {
        let panel = StackPanel::new(CpuMode::I4040);
        assert_eq!(panel.max_depth(), 7);
    }

    #[test]
    fn update_stores_snapshot() {
        let mut panel = StackPanel::new(CpuMode::I4004);
        let snap = StackSnapshot::new(&[0x100, 0x200, 0x000], 1);
        panel.update(snap);

        assert_eq!(panel.snapshot().depth(), 3);
        assert_eq!(panel.snapshot().sp, 1);
        assert_eq!(panel.snapshot().entry_at(0), Some(0x100));
        assert_eq!(panel.snapshot().entry_at(1), Some(0x200));
    }

    #[test]
    fn entry_change_detection() {
        let mut panel = StackPanel::new(CpuMode::I4004);

        panel.update(StackSnapshot::new(&[0x100, 0x200, 0x000], 1));
        panel.update(StackSnapshot::new(&[0x100, 0x300, 0x000], 1));

        assert!(!panel.entry_changed(0));
        assert!(panel.entry_changed(1)); // 0x200 -> 0x300
        assert!(!panel.entry_changed(2));
    }

    #[test]
    fn sp_change_detection() {
        let mut panel = StackPanel::new(CpuMode::I4004);

        panel.update(StackSnapshot::new(&[0x100, 0x000, 0x000], 1));
        panel.update(StackSnapshot::new(&[0x100, 0x200, 0x000], 2));

        assert!(panel.sp_changed());
    }

    #[test]
    fn sp_no_change_when_same() {
        let mut panel = StackPanel::new(CpuMode::I4004);

        panel.update(StackSnapshot::new(&[0x100, 0x000, 0x000], 1));
        panel.update(StackSnapshot::new(&[0x100, 0x000, 0x000], 1));

        assert!(!panel.sp_changed());
    }

    #[test]
    fn mode_switching() {
        let mut panel = StackPanel::new(CpuMode::I4004);
        assert_eq!(panel.mode(), CpuMode::I4004);
        assert_eq!(panel.max_depth(), 3);

        panel.set_mode(CpuMode::I4040);
        assert_eq!(panel.mode(), CpuMode::I4040);
        assert_eq!(panel.max_depth(), 7);
    }

    #[test]
    fn default_is_4004() {
        let panel = StackPanel::default();
        assert_eq!(panel.mode(), CpuMode::I4004);
        assert!(panel.snapshot().is_empty());
    }

    #[test]
    fn entry_at_out_of_bounds() {
        let snap = StackSnapshot::new(&[0x100], 1);
        assert_eq!(snap.entry_at(0), Some(0x100));
        assert_eq!(snap.entry_at(1), None);
        assert_eq!(snap.entry_at(99), None);
    }
}
