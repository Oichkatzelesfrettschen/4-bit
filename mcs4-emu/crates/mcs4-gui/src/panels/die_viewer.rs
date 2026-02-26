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
            let (_rect, _response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 400.0),
                egui::Sense::drag(),
            );

            // TODO: Render die image and overlays
            ui.label("Photomicrograph overlay placeholder");
            
            if let Some(_solver) = solver {
                // TODO: Highlight transistors based on solver state
                ui.label("(Active solver detected)");
            } else {
                ui.label("(Running at Behavioral level)");
            }
        });
    }
}
