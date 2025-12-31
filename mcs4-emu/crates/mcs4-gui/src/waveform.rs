// Waveform renderer scaffolding (flat path)
use eframe::egui::{self, Color32};
use crate::signal_trace::SignalTrace;

pub struct WaveformPanel {
    pub trace: SignalTrace,
    pub zoom: f32,
    pub scroll_x: u64,
}

impl WaveformPanel {
    pub fn new(trace: SignalTrace) -> Self { Self { trace, zoom: 1.0, scroll_x: 0 } }
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let painter = ui.painter();
        let y = 10.0;
        self.draw_digital(painter, &self.trace.phi1, y, Color32::LIGHT_BLUE);
        self.draw_digital(painter, &self.trace.phi2, y+20.0, Color32::LIGHT_GREEN);
    }
    fn draw_digital(&self, painter: &egui::Painter, signal: &[bool], y: f32, color: Color32) {
        let mut x = 10.0;
        for &s in signal.iter() {
            let y0 = if s { y } else { y+10.0 };
            painter.rect_filled(egui::Rect::from_min_size(egui::pos2(x, y0), egui::vec2(5.0, 10.0)), 0.0, color);
            x += 6.0;
        }
    }
}
