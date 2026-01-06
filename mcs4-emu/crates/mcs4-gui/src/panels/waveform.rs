//! Waveform display panel

use std::sync::{Arc, RwLock};

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Sense, Stroke};

use crate::signal_trace::SignalTrace;

struct SignalDrawContext<'a> {
    painter: &'a Painter,
    samples: &'a [crate::signal_trace::Sample],
    x_start: f32,
    y: f32,
    color: Color32,
    zoom: f32,
    row_height: f32,
}

pub struct WaveformPanel {
    trace: Arc<RwLock<SignalTrace>>,
    zoom: f32, // pixels per tick
    scroll_x: f32,
    row_height: f32,
}

impl WaveformPanel {
    pub fn new(trace: Arc<RwLock<SignalTrace>>) -> Self {
        Self {
            trace,
            zoom: 10.0,
            scroll_x: 0.0,
            row_height: 20.0,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.heading("Waveform Viewer");

        // Controls
        ui.horizontal(|ui| {
            if ui.button("- Zoom").clicked() {
                self.zoom = (self.zoom * 0.8).max(0.1);
            }
            if ui.button("+ Zoom").clicked() {
                self.zoom = (self.zoom * 1.25).min(100.0);
            }
            ui.label(format!("Zoom: {:.1} px/tick", self.zoom));
        });

        // Drawing area
        let available_size = ui.available_size();
        let (response, painter) = ui.allocate_painter(available_size, Sense::drag());

        if response.dragged() {
            self.scroll_x -= response.drag_delta().x;
            self.scroll_x = self.scroll_x.max(0.0);
        }

        let trace = self.trace.read().expect("signal trace lock poisoned");
        if trace.is_empty() {
            ui.label("No signal data captured.");
            return;
        }

        let start_sample_idx = (self.scroll_x / self.zoom) as usize;
        let visible_samples = (available_size.x / self.zoom) as usize + 2;

        if start_sample_idx >= trace.len() {
            return;
        }

        let samples: Vec<_> = trace
            .iter()
            .skip(start_sample_idx)
            .take(visible_samples)
            .cloned()
            .collect();

        let y_start = response.rect.min.y + 10.0;
        let x_start = response.rect.min.x;

        let mut ctx = SignalDrawContext {
            painter: &painter,
            samples: &samples,
            x_start,
            y: y_start,
            color: Color32::LIGHT_BLUE,
            zoom: self.zoom,
            row_height: self.row_height,
        };

        // Draw Signals
        self.draw_digital_signal("PHI1", |s| s.phi1, &ctx);
        ctx.y += self.row_height * 1.5;
        self.draw_digital_signal("PHI2", |s| s.phi2, &ctx);
        ctx.y += self.row_height * 1.5;
        ctx.color = Color32::YELLOW;
        self.draw_digital_signal("SYNC", |s| s.sync, &ctx);
        ctx.y += self.row_height * 1.5;

        ctx.color = Color32::GREEN;
        self.draw_bus_signal("DATA", |s| s.data, &ctx);
        ctx.y += self.row_height * 1.5;
        ctx.color = Color32::RED;
        self.draw_bus_signal("CM-ROM", |s| s.cm_rom, &ctx);
        ctx.y += self.row_height * 1.5;
        ctx.color = Color32::from_rgb(255, 128, 0);
        self.draw_bus_signal("CM-RAM", |s| s.cm_ram, &ctx);

        // Draw Cycle Phases
        ctx.y += self.row_height * 1.5;
        self.draw_phases(&painter, &samples, x_start, ctx.y);
    }

    fn draw_digital_signal<F>(&self, name: &str, extract: F, ctx: &SignalDrawContext<'_>)
    where
        F: Fn(&crate::signal_trace::Sample) -> bool,
    {
        let text_pos = Pos2::new(ctx.x_start + 5.0, ctx.y - 10.0);
        ctx.painter.text(
            text_pos,
            egui::Align2::LEFT_BOTTOM,
            name,
            egui::FontId::proportional(12.0),
            Color32::WHITE,
        );

        let mut path = Vec::new();
        let high_y = ctx.y - ctx.row_height;
        let low_y = ctx.y;

        for (i, sample) in ctx.samples.iter().enumerate() {
            let x = ctx.x_start + (i as f32) * ctx.zoom;
            let val = extract(sample);
            let target_y = if val { high_y } else { low_y };

            if i == 0 {
                path.push(Pos2::new(x, target_y));
            } else {
                let prev_val = extract(&ctx.samples[i - 1]);
                if prev_val != val {
                    path.push(Pos2::new(x, if prev_val { high_y } else { low_y }));
                    path.push(Pos2::new(x, target_y));
                }
                path.push(Pos2::new(x + ctx.zoom, target_y));
            }
        }

        ctx.painter.add(egui::Shape::line(path, Stroke::new(1.5, ctx.color)));
    }

    fn draw_bus_signal<F>(&self, name: &str, extract: F, ctx: &SignalDrawContext<'_>)
    where
        F: Fn(&crate::signal_trace::Sample) -> u8,
    {
        let text_pos = Pos2::new(ctx.x_start + 5.0, ctx.y - 10.0);
        ctx.painter.text(
            text_pos,
            egui::Align2::LEFT_BOTTOM,
            name,
            egui::FontId::proportional(12.0),
            Color32::WHITE,
        );

        let top_y = ctx.y - ctx.row_height;
        let bottom_y = ctx.y;
        let mid_y = ctx.y - ctx.row_height / 2.0;

        for (i, sample) in ctx.samples.iter().enumerate() {
            let x = ctx.x_start + (i as f32) * ctx.zoom;
            let val = extract(sample);
            let prev_val = if i > 0 {
                Some(extract(&ctx.samples[i - 1]))
            } else {
                None
            };

            let rect = Rect::from_min_max(Pos2::new(x, top_y), Pos2::new(x + ctx.zoom, bottom_y));
            let stroke = Stroke::new(1.0, ctx.color);

            if let Some(pv) = prev_val
                && pv != val
            {
                ctx.painter
                    .line_segment([Pos2::new(x, top_y), Pos2::new(x, bottom_y)], stroke);
            }

            ctx.painter
                .line_segment([rect.min, Pos2::new(rect.max.x, rect.min.y)], stroke);
            ctx.painter
                .line_segment([Pos2::new(rect.min.x, rect.max.y), rect.max], stroke);

            if ctx.zoom > 20.0 {
                ctx.painter.text(
                    Pos2::new(x + ctx.zoom / 2.0, mid_y),
                    egui::Align2::CENTER_CENTER,
                    format!("{:X}", val),
                    egui::FontId::monospace(10.0),
                    Color32::WHITE,
                );
            }
        }
    }

    fn draw_phases(&self, painter: &Painter, samples: &[crate::signal_trace::Sample], x_start: f32, y: f32) {
        for (i, sample) in samples.iter().enumerate() {
            let x = x_start + (i as f32) * self.zoom;
            if self.zoom > 30.0 {
                let text = format!("{:?}", sample.phase);
                painter.text(
                    Pos2::new(x + self.zoom / 2.0, y - self.row_height / 2.0),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::monospace(9.0),
                    Color32::GRAY,
                );
            }
        }
    }
}
