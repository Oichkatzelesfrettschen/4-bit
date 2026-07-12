//! Evidence-bound physical-layout panel.

use eframe::egui;
use mcs4_system::TraceFrame;

/// Panel that reports whether the selected frame can support a die overlay.
pub struct DieViewerPanel;

impl DieViewerPanel {
    /// Construct a panel without inventing unavailable layout data.
    pub const fn new() -> Self {
        Self
    }

    /// Render the physical-layout evidence boundary for one trace frame.
    pub fn ui(&mut self, ui: &mut egui::Ui, frame: Option<&TraceFrame>) {
        ui.heading("Die Evidence");
        let Some(frame) = frame else {
            ui.label("No trace frame is selected.");
            return;
        };

        ui.monospace(format!(
            "{} / {:?} / {:?}",
            frame.provenance.model_id, frame.provenance.backend, frame.provenance.fidelity
        ));
        ui.label("The selected frame has no coordinate-bearing transistor observations.");
        ui.label("The GUI does not draw a photomicrograph or active-transistor overlay without a registered physical netlist and coordinate map.");
        ui.small("Required input: verified transistor identifiers, node mapping, and die coordinates linked to the selected model SHA-256.");
    }
}

impl Default for DieViewerPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_constructs_without_a_physical_overlay() {
        let panel = DieViewerPanel::new();
        let _ = panel;
    }
}
