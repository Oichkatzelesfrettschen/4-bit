//! Breakpoint management panel for debugger

use eframe::egui::{self, Color32};

/// Unique breakpoint identifier
pub type BreakpointId = u32;

/// Type of breakpoint trigger condition
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BreakpointKind {
    /// Break when PC reaches this address
    Address(u16),
    /// Break when a register equals a value: (register index, value)
    Register(u8, u8),
    /// Break when memory at address equals value: (address, value)
    Memory(u16, u8),
}

impl std::fmt::Display for BreakpointKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BreakpointKind::Address(addr) => write!(f, "PC={:03X}", addr),
            BreakpointKind::Register(idx, val) => write!(f, "R{:X}={:X}", idx, val),
            BreakpointKind::Memory(addr, val) => write!(f, "M[{:03X}]={:02X}", addr, val),
        }
    }
}

/// A single breakpoint entry
#[derive(Clone, Debug)]
pub struct Breakpoint {
    pub id: BreakpointId,
    pub kind: BreakpointKind,
    pub enabled: bool,
    pub hit_count: u64,
}

impl Breakpoint {
    pub fn new(id: BreakpointId, kind: BreakpointKind) -> Self {
        Self {
            id,
            kind,
            enabled: true,
            hit_count: 0,
        }
    }
}

/// Breakpoint manager -- stores and evaluates breakpoints
pub struct BreakpointPanel {
    breakpoints: Vec<Breakpoint>,
    next_id: BreakpointId,
    /// Pending address input for new breakpoint (hex string)
    input_addr: String,
}

impl BreakpointPanel {
    pub fn new() -> Self {
        Self {
            breakpoints: Vec::new(),
            next_id: 1,
            input_addr: String::new(),
        }
    }

    /// Add a breakpoint, returning its ID
    pub fn add(&mut self, kind: BreakpointKind) -> BreakpointId {
        let id = self.next_id;
        self.next_id += 1;
        self.breakpoints.push(Breakpoint::new(id, kind));
        id
    }

    /// Remove a breakpoint by ID. Returns true if found and removed.
    pub fn remove(&mut self, id: BreakpointId) -> bool {
        let before = self.breakpoints.len();
        self.breakpoints.retain(|bp| bp.id != id);
        self.breakpoints.len() < before
    }

    /// Toggle enabled state for a breakpoint by ID
    pub fn toggle(&mut self, id: BreakpointId) {
        if let Some(bp) = self.breakpoints.iter_mut().find(|bp| bp.id == id) {
            bp.enabled = !bp.enabled;
        }
    }

    /// Enable or disable a breakpoint by ID
    pub fn set_enabled(&mut self, id: BreakpointId, enabled: bool) {
        if let Some(bp) = self.breakpoints.iter_mut().find(|bp| bp.id == id) {
            bp.enabled = enabled;
        }
    }

    /// Number of breakpoints
    pub fn len(&self) -> usize {
        self.breakpoints.len()
    }

    /// True if no breakpoints set
    pub fn is_empty(&self) -> bool {
        self.breakpoints.is_empty()
    }

    /// Get breakpoint by ID
    pub fn get(&self, id: BreakpointId) -> Option<&Breakpoint> {
        self.breakpoints.iter().find(|bp| bp.id == id)
    }

    /// Iterator over all breakpoints
    pub fn iter(&self) -> impl Iterator<Item = &Breakpoint> {
        self.breakpoints.iter()
    }

    /// Check if any enabled breakpoint triggers for the given CPU state.
    /// Returns the ID of the first triggered breakpoint, or None.
    /// Increments hit_count on triggered breakpoints.
    pub fn check_triggers(
        &mut self,
        pc: u16,
        registers: &[u8],
        memory_read: &dyn Fn(u16) -> u8,
    ) -> Option<BreakpointId> {
        for bp in &mut self.breakpoints {
            if !bp.enabled {
                continue;
            }
            let triggered = match &bp.kind {
                BreakpointKind::Address(addr) => pc == *addr,
                BreakpointKind::Register(idx, val) => registers.get(*idx as usize).copied().unwrap_or(0) == *val,
                BreakpointKind::Memory(addr, val) => memory_read(*addr) == *val,
            };
            if triggered {
                bp.hit_count += 1;
                return Some(bp.id);
            }
        }
        None
    }

    /// Remove all breakpoints
    pub fn clear(&mut self) {
        self.breakpoints.clear();
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.heading("Breakpoints");
        ui.separator();

        // Add breakpoint input
        ui.horizontal(|ui| {
            ui.label("Addr:");
            let response = ui.text_edit_singleline(&mut self.input_addr);
            if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || ui.button("Add").clicked() {
                if let Ok(addr) = u16::from_str_radix(self.input_addr.trim(), 16) {
                    self.add(BreakpointKind::Address(addr & 0x0FFF));
                    self.input_addr.clear();
                }
            }
        });

        ui.separator();

        if self.breakpoints.is_empty() {
            ui.label("No breakpoints set");
            return;
        }

        // List breakpoints
        let mut to_remove = None;
        let mut to_toggle = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for bp in &self.breakpoints {
                ui.horizontal(|ui| {
                    // Enable/disable checkbox
                    let mut enabled = bp.enabled;
                    if ui.checkbox(&mut enabled, "").changed() {
                        to_toggle = Some(bp.id);
                    }

                    // Breakpoint description
                    let color = if bp.enabled { Color32::WHITE } else { Color32::GRAY };
                    ui.label(
                        egui::RichText::new(format!("#{}: {}", bp.id, bp.kind))
                            .color(color)
                            .monospace(),
                    );

                    // Hit count
                    if bp.hit_count > 0 {
                        ui.label(egui::RichText::new(format!("({}x)", bp.hit_count)).color(Color32::YELLOW));
                    }

                    // Remove button
                    if ui.button("X").clicked() {
                        to_remove = Some(bp.id);
                    }
                });
            }
        });

        if let Some(id) = to_toggle {
            self.toggle(id);
        }
        if let Some(id) = to_remove {
            self.remove(id);
        }

        // Clear all button
        ui.separator();
        if ui.button("Clear All").clicked() {
            self.clear();
        }
    }
}

impl Default for BreakpointPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_panel_is_empty() {
        let panel = BreakpointPanel::new();
        assert!(panel.is_empty());
        assert_eq!(panel.len(), 0);
    }

    #[test]
    fn add_returns_unique_ids() {
        let mut panel = BreakpointPanel::new();
        let id1 = panel.add(BreakpointKind::Address(0x100));
        let id2 = panel.add(BreakpointKind::Address(0x200));
        assert_ne!(id1, id2);
        assert_eq!(panel.len(), 2);
    }

    #[test]
    fn remove_by_id() {
        let mut panel = BreakpointPanel::new();
        let id1 = panel.add(BreakpointKind::Address(0x100));
        let id2 = panel.add(BreakpointKind::Address(0x200));

        assert!(panel.remove(id1));
        assert_eq!(panel.len(), 1);
        assert!(panel.get(id1).is_none());
        assert!(panel.get(id2).is_some());
    }

    #[test]
    fn remove_nonexistent_returns_false() {
        let mut panel = BreakpointPanel::new();
        assert!(!panel.remove(999));
    }

    #[test]
    fn toggle_enabled() {
        let mut panel = BreakpointPanel::new();
        let id = panel.add(BreakpointKind::Address(0x100));

        assert!(panel.get(id).expect("bp exists").enabled);
        panel.toggle(id);
        assert!(!panel.get(id).expect("bp exists").enabled);
        panel.toggle(id);
        assert!(panel.get(id).expect("bp exists").enabled);
    }

    #[test]
    fn set_enabled() {
        let mut panel = BreakpointPanel::new();
        let id = panel.add(BreakpointKind::Address(0x100));

        panel.set_enabled(id, false);
        assert!(!panel.get(id).expect("bp exists").enabled);
        panel.set_enabled(id, true);
        assert!(panel.get(id).expect("bp exists").enabled);
    }

    #[test]
    fn check_address_trigger() {
        let mut panel = BreakpointPanel::new();
        let id = panel.add(BreakpointKind::Address(0x100));
        let regs = [0u8; 16];
        let mem = |_: u16| 0u8;

        // Should not trigger at different PC
        assert!(panel.check_triggers(0x050, &regs, &mem).is_none());

        // Should trigger at matching PC
        assert_eq!(panel.check_triggers(0x100, &regs, &mem), Some(id));
        assert_eq!(panel.get(id).expect("bp exists").hit_count, 1);
    }

    #[test]
    fn check_register_trigger() {
        let mut panel = BreakpointPanel::new();
        let id = panel.add(BreakpointKind::Register(3, 0xA));
        let mut regs = [0u8; 16];
        let mem = |_: u16| 0u8;

        // Not triggered yet
        assert!(panel.check_triggers(0, &regs, &mem).is_none());

        // Set register to match
        regs[3] = 0xA;
        assert_eq!(panel.check_triggers(0, &regs, &mem), Some(id));
    }

    #[test]
    fn check_memory_trigger() {
        let mut panel = BreakpointPanel::new();
        let id = panel.add(BreakpointKind::Memory(0x040, 0xFF));
        let regs = [0u8; 16];

        // Memory returns 0 -- no trigger
        assert!(panel.check_triggers(0, &regs, &|_| 0u8).is_none());

        // Memory returns matching value
        let mem = |addr: u16| -> u8 {
            if addr == 0x040 {
                0xFF
            } else {
                0
            }
        };
        assert_eq!(panel.check_triggers(0, &regs, &mem), Some(id));
    }

    #[test]
    fn disabled_breakpoint_does_not_trigger() {
        let mut panel = BreakpointPanel::new();
        let id = panel.add(BreakpointKind::Address(0x100));
        panel.set_enabled(id, false);
        let regs = [0u8; 16];
        let mem = |_: u16| 0u8;

        assert!(panel.check_triggers(0x100, &regs, &mem).is_none());
    }

    #[test]
    fn clear_removes_all() {
        let mut panel = BreakpointPanel::new();
        panel.add(BreakpointKind::Address(0x100));
        panel.add(BreakpointKind::Address(0x200));
        panel.add(BreakpointKind::Register(0, 5));

        panel.clear();
        assert!(panel.is_empty());
        assert_eq!(panel.len(), 0);
    }

    #[test]
    fn breakpoint_kind_display() {
        assert_eq!(format!("{}", BreakpointKind::Address(0x1FF)), "PC=1FF");
        assert_eq!(format!("{}", BreakpointKind::Register(5, 0xA)), "R5=A");
        assert_eq!(format!("{}", BreakpointKind::Memory(0x040, 0xFF)), "M[040]=FF");
    }

    #[test]
    fn default_is_new() {
        let panel = BreakpointPanel::default();
        assert!(panel.is_empty());
    }
}
