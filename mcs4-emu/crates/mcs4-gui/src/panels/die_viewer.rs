//! Die Viewer Panel for visualizing silicon-level simulation.
//!
//! WHY: Provides a visual bridge between the abstract netlist simulation
//! and the physical reality of the Intel 4004 die. Highlights active
//! transistors and nodes directly on the photomicrograph.

use eframe::egui;
use mcs4_core::nodal_solver::NodalSolver;

pub struct DieViewerPanel {
    /// Zoom level for the die image.
    zoom: f32,
    /// Whether to highlight active transistors.
    highlight_active: bool,
}

impl Default for DieViewerPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl DieViewerPanel {
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            highlight_active: true,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, solver: Option<&NodalSolver>) {
        ui.heading("Die Viewer (Digital Twin)");

        ui.horizontal(|ui| {
            ui.label("Zoom:");
            ui.add(egui::Slider::new(&mut self.zoom, 0.1..=10.0));
            ui.checkbox(&mut self.highlight_active, "Highlight Active");
        });

        egui::Frame::canvas(ui.style()).show(ui, |ui| {
            let (_rect, _response) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), 400.0), egui::Sense::drag());

            // PLANNED (debt phase D1.5.3): render the photomicrograph from
            // `docs/photomicrographs/` via egui textures and overlay
            // schematic-anchor bounding boxes from
            // `docs/evidence/schematic_layout_anchors_v1.json`.
            ui.label("Photomicrograph overlay placeholder");

            if let Some(_solver) = solver {
                // PLANNED (debt phase D1.5.3): when a NodalSolver is attached,
                // colour each transistor bbox by its on/off / saturation state
                // using `solver.voltage(node_id)` and the device-model thresholds.
                ui.label("(Active solver detected)");
            } else {
                ui.label("(Running at Behavioral level)");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_creates_panel() {
        let panel = DieViewerPanel::default();
        // Default should be constructable without panics
        let _ = panel;
    }

    #[test]
    fn test_new_default_zoom() {
        let panel = DieViewerPanel::new();
        assert!((panel.zoom - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_highlight_active_default() {
        let panel = DieViewerPanel::new();
        assert!(panel.highlight_active, "highlight_active should default to true");
    }
}
