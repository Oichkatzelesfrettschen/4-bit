//! Trace-provenance inspector for every frame displayed by the GUI.

use eframe::egui;
use mcs4_system::TraceFrame;

/// Panel that exposes the evidence boundary of the selected trace frame.
#[derive(Default)]
pub struct ProvenancePanel;

impl ProvenancePanel {
    /// Render the provenance attached to the latest retained frame.
    pub fn show(&mut self, ui: &mut egui::Ui, frame: Option<&TraceFrame>) {
        ui.heading("Trace Provenance");
        let Some(frame) = frame else {
            ui.label("No trace frame is selected.");
            return;
        };

        egui::Grid::new("trace_provenance_grid")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                property(
                    ui,
                    "Frame",
                    &format!("run {} sequence {}", frame.run_id, frame.sequence),
                );
                property(ui, "Input event", &frame.input_event_id.to_string());
                property(ui, "Logical tick", &frame.logical_tick.to_string());
                property(
                    ui,
                    "Physical time",
                    &frame
                        .physical_time_ps
                        .map_or_else(|| "not declared".to_owned(), |time| format!("{time} ps")),
                );
                property(ui, "Backend", &format!("{:?}", frame.provenance.backend));
                property(ui, "Fidelity", &format!("{:?}", frame.provenance.fidelity));
                property(ui, "Model", &frame.provenance.model_id);
                property(
                    ui,
                    "Model SHA-256",
                    frame.provenance.model_sha256.as_deref().unwrap_or("not sealed"),
                );
                property(
                    ui,
                    "Stimulus SHA-256",
                    frame.provenance.stimulus_sha256.as_deref().unwrap_or("not declared"),
                );
                property(
                    ui,
                    "Evidence status",
                    &format!("{:?}", frame.provenance.evidence_status),
                );
                property(ui, "Observed signals", &frame.signals.len().to_string());
            });

        if frame.physical_time_ps.is_none() {
            ui.small("Logical sequence does not establish calibrated physical time.");
        }
    }
}

fn property(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(label);
    ui.monospace(value);
    ui.end_row();
}

#[cfg(test)]
mod tests {
    use mcs4_system::{Mcs4System, ReplaySession};

    #[test]
    fn behavioral_frame_exposes_unsealed_provenance() {
        let mut session = ReplaySession::<Mcs4System>::new();
        let frame = session.step_phase().expect("step phase");
        assert_eq!(frame.provenance.model_id, "mcs4-behavioral");
        assert!(frame.provenance.model_sha256.is_none());
    }
}
