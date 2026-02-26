//! Waveform display panel

use std::sync::{Arc, RwLock};

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Sense, Stroke};

use crate::signal_trace::SignalTrace;

/// Signal group categories for organizing waveform display
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalGroup {
    Clock,
    Data,
    Control,
}

impl std::fmt::Display for SignalGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignalGroup::Clock => write!(f, "Clock"),
            SignalGroup::Data => write!(f, "Data"),
            SignalGroup::Control => write!(f, "Control"),
        }
    }
}

/// Measurement cursor for time-delta calculations
#[derive(Clone, Copy, Debug, Default)]
pub struct CursorState {
    /// Time cursor: current tick position (follows mouse or set by click)
    pub time_cursor: Option<u64>,
    /// Marker A for delta measurement
    pub marker_a: Option<u64>,
    /// Marker B for delta measurement
    pub marker_b: Option<u64>,
}

impl CursorState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the time cursor position
    pub fn set_time_cursor(&mut self, tick: u64) {
        self.time_cursor = Some(tick);
    }

    /// Clear the time cursor
    pub fn clear_time_cursor(&mut self) {
        self.time_cursor = None;
    }

    /// Set marker A
    pub fn set_marker_a(&mut self, tick: u64) {
        self.marker_a = Some(tick);
    }

    /// Set marker B
    pub fn set_marker_b(&mut self, tick: u64) {
        self.marker_b = Some(tick);
    }

    /// Clear both markers
    pub fn clear_markers(&mut self) {
        self.marker_a = None;
        self.marker_b = None;
    }

    /// Calculate delta between markers A and B.
    /// Returns Some((abs_delta, a_tick, b_tick)) if both markers are set.
    pub fn marker_delta(&self) -> Option<(u64, u64, u64)> {
        match (self.marker_a, self.marker_b) {
            (Some(a), Some(b)) => {
                let delta = a.abs_diff(b);
                Some((delta, a, b))
            }
            _ => None,
        }
    }

    /// Calculate frequency from marker delta at a given clock rate.
    /// Returns frequency in Hz if both markers are set and delta > 0.
    pub fn frequency_from_delta(&self, clock_hz: f64) -> Option<f64> {
        self.marker_delta().and_then(|(delta, _, _)| {
            if delta == 0 {
                None
            } else {
                Some(clock_hz / delta as f64)
            }
        })
    }
}

/// Signal visibility configuration
#[derive(Clone, Debug)]
pub struct SignalConfig {
    pub name: &'static str,
    pub group: SignalGroup,
    pub visible: bool,
    pub color: Color32,
}

/// Default signal configuration for MCS-4 waveform display
pub fn default_signal_configs() -> Vec<SignalConfig> {
    vec![
        SignalConfig {
            name: "PHI1",
            group: SignalGroup::Clock,
            visible: true,
            color: Color32::LIGHT_BLUE,
        },
        SignalConfig {
            name: "PHI2",
            group: SignalGroup::Clock,
            visible: true,
            color: Color32::LIGHT_BLUE,
        },
        SignalConfig {
            name: "SYNC",
            group: SignalGroup::Control,
            visible: true,
            color: Color32::YELLOW,
        },
        SignalConfig {
            name: "DATA",
            group: SignalGroup::Data,
            visible: true,
            color: Color32::GREEN,
        },
        SignalConfig {
            name: "CM-ROM",
            group: SignalGroup::Control,
            visible: true,
            color: Color32::RED,
        },
        SignalConfig {
            name: "CM-RAM",
            group: SignalGroup::Control,
            visible: true,
            color: Color32::from_rgb(255, 128, 0),
        },
    ]
}

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
    pub cursors: CursorState,
    pub signals: Vec<SignalConfig>,
}

impl WaveformPanel {
    pub fn new(trace: Arc<RwLock<SignalTrace>>) -> Self {
        Self {
            trace,
            zoom: 10.0,
            scroll_x: 0.0,
            row_height: 20.0,
            cursors: CursorState::new(),
            signals: default_signal_configs(),
        }
    }

    /// Current zoom level (pixels per tick)
    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Set zoom level, clamped to valid range
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.1, 100.0);
    }

    /// Current scroll offset (pixels)
    pub fn scroll_x(&self) -> f32 {
        self.scroll_x
    }

    /// Count of visible (enabled) signals
    pub fn visible_signal_count(&self) -> usize {
        self.signals.iter().filter(|s| s.visible).count()
    }

    /// Toggle visibility of a signal by name
    pub fn toggle_signal(&mut self, name: &str) {
        if let Some(sig) = self.signals.iter_mut().find(|s| s.name == name) {
            sig.visible = !sig.visible;
        }
    }

    /// Get signals in a specific group
    pub fn signals_in_group(&self, group: SignalGroup) -> Vec<&SignalConfig> {
        self.signals.iter().filter(|s| s.group == group).collect()
    }

    /// Convert a pixel x-coordinate to a tick index (relative to scroll)
    pub fn pixel_to_tick(&self, pixel_x: f32) -> u64 {
        ((self.scroll_x + pixel_x) / self.zoom) as u64
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.heading("Waveform Viewer");

        // Controls row
        ui.horizontal(|ui| {
            if ui.button("- Zoom").clicked() {
                self.zoom = (self.zoom * 0.8).max(0.1);
            }
            if ui.button("+ Zoom").clicked() {
                self.zoom = (self.zoom * 1.25).min(100.0);
            }
            ui.label(format!("Zoom: {:.1} px/tick", self.zoom));

            ui.separator();

            // Marker controls
            if ui.button("Set A").clicked() {
                if let Some(tc) = self.cursors.time_cursor {
                    self.cursors.set_marker_a(tc);
                }
            }
            if ui.button("Set B").clicked() {
                if let Some(tc) = self.cursors.time_cursor {
                    self.cursors.set_marker_b(tc);
                }
            }
            if ui.button("Clear").clicked() {
                self.cursors.clear_markers();
            }

            // Show delta if both markers set
            if let Some((delta, a, b)) = self.cursors.marker_delta() {
                ui.label(
                    egui::RichText::new(format!("A:{} B:{} dt:{}", a, b, delta))
                        .color(Color32::LIGHT_GREEN)
                        .monospace(),
                );
            }
        });

        // Signal group toggles
        ui.horizontal(|ui| {
            for group in [SignalGroup::Clock, SignalGroup::Data, SignalGroup::Control] {
                let count = self.signals_in_group(group).len();
                let visible = self.signals_in_group(group).iter().filter(|s| s.visible).count();
                let label = format!("{} ({}/{})", group, visible, count);
                ui.label(egui::RichText::new(label).color(Color32::GRAY));
            }
        });

        // Drawing area
        let available_size = ui.available_size();
        let (response, painter) = ui.allocate_painter(available_size, Sense::drag());

        if response.dragged() {
            self.scroll_x -= response.drag_delta().x;
            self.scroll_x = self.scroll_x.max(0.0);
        }

        // Update time cursor on hover
        if let Some(hover_pos) = response.hover_pos() {
            let relative_x = hover_pos.x - response.rect.min.x;
            let tick = self.pixel_to_tick(relative_x);
            self.cursors.set_time_cursor(tick);
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

        // Draw visible signals by group order
        // Clock group
        if self.signal_visible("PHI1") {
            ctx.color = self.signal_color("PHI1");
            self.draw_digital_signal("PHI1", |s| s.phi1, &ctx);
            ctx.y += self.row_height * 1.5;
        }
        if self.signal_visible("PHI2") {
            ctx.color = self.signal_color("PHI2");
            self.draw_digital_signal("PHI2", |s| s.phi2, &ctx);
            ctx.y += self.row_height * 1.5;
        }

        // Control group
        if self.signal_visible("SYNC") {
            ctx.color = self.signal_color("SYNC");
            self.draw_digital_signal("SYNC", |s| s.sync, &ctx);
            ctx.y += self.row_height * 1.5;
        }

        // Data group
        if self.signal_visible("DATA") {
            ctx.color = self.signal_color("DATA");
            self.draw_bus_signal("DATA", |s| s.data, &ctx);
            ctx.y += self.row_height * 1.5;
        }
        if self.signal_visible("CM-ROM") {
            ctx.color = self.signal_color("CM-ROM");
            self.draw_bus_signal("CM-ROM", |s| s.cm_rom, &ctx);
            ctx.y += self.row_height * 1.5;
        }
        if self.signal_visible("CM-RAM") {
            ctx.color = self.signal_color("CM-RAM");
            self.draw_bus_signal("CM-RAM", |s| s.cm_ram, &ctx);
            ctx.y += self.row_height * 1.5;
        }

        // Draw Cycle Phases
        self.draw_phases(&painter, &samples, x_start, ctx.y);

        // Draw time cursor
        if let Some(tc) = self.cursors.time_cursor {
            let tc_idx = tc.saturating_sub(start_sample_idx as u64);
            let cursor_x = x_start + (tc_idx as f32) * self.zoom;
            if cursor_x >= response.rect.min.x && cursor_x <= response.rect.max.x {
                painter.line_segment(
                    [
                        Pos2::new(cursor_x, response.rect.min.y),
                        Pos2::new(cursor_x, response.rect.max.y),
                    ],
                    Stroke::new(1.0, Color32::WHITE),
                );
            }
        }

        // Draw marker lines
        for (marker_tick, marker_color) in [
            (self.cursors.marker_a, Color32::LIGHT_GREEN),
            (self.cursors.marker_b, Color32::LIGHT_RED),
        ] {
            if let Some(mt) = marker_tick {
                let mt_idx = mt.saturating_sub(start_sample_idx as u64);
                let marker_x = x_start + (mt_idx as f32) * self.zoom;
                if marker_x >= response.rect.min.x && marker_x <= response.rect.max.x {
                    painter.line_segment(
                        [
                            Pos2::new(marker_x, response.rect.min.y),
                            Pos2::new(marker_x, response.rect.max.y),
                        ],
                        Stroke::new(2.0, marker_color),
                    );
                }
            }
        }
    }

    fn signal_visible(&self, name: &str) -> bool {
        self.signals
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.visible)
            .unwrap_or(false)
    }

    fn signal_color(&self, name: &str) -> Color32 {
        self.signals
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.color)
            .unwrap_or(Color32::WHITE)
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

            if let Some(pv) = prev_val {
                if pv != val {
                    ctx.painter
                        .line_segment([Pos2::new(x, top_y), Pos2::new(x, bottom_y)], stroke);
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_state_defaults() {
        let cs = CursorState::new();
        assert!(cs.time_cursor.is_none());
        assert!(cs.marker_a.is_none());
        assert!(cs.marker_b.is_none());
        assert!(cs.marker_delta().is_none());
    }

    #[test]
    fn cursor_set_and_clear_time() {
        let mut cs = CursorState::new();
        cs.set_time_cursor(42);
        assert_eq!(cs.time_cursor, Some(42));
        cs.clear_time_cursor();
        assert!(cs.time_cursor.is_none());
    }

    #[test]
    fn marker_delta_calculation() {
        let mut cs = CursorState::new();
        cs.set_marker_a(100);
        cs.set_marker_b(150);
        let (delta, a, b) = cs.marker_delta().expect("both markers set");
        assert_eq!(delta, 50);
        assert_eq!(a, 100);
        assert_eq!(b, 150);
    }

    #[test]
    fn marker_delta_reversed() {
        let mut cs = CursorState::new();
        cs.set_marker_a(200);
        cs.set_marker_b(50);
        let (delta, _, _) = cs.marker_delta().expect("both markers set");
        assert_eq!(delta, 150);
    }

    #[test]
    fn marker_delta_none_when_partial() {
        let mut cs = CursorState::new();
        cs.set_marker_a(100);
        assert!(cs.marker_delta().is_none());
    }

    #[test]
    fn marker_clear() {
        let mut cs = CursorState::new();
        cs.set_marker_a(10);
        cs.set_marker_b(20);
        cs.clear_markers();
        assert!(cs.marker_a.is_none());
        assert!(cs.marker_b.is_none());
    }

    #[test]
    fn frequency_from_delta() {
        let mut cs = CursorState::new();
        cs.set_marker_a(0);
        cs.set_marker_b(100);
        // At 740kHz clock, 100 ticks = 7400 Hz
        let freq = cs.frequency_from_delta(740_000.0).expect("valid delta");
        assert!((freq - 7400.0).abs() < 0.01);
    }

    #[test]
    fn frequency_zero_delta_returns_none() {
        let mut cs = CursorState::new();
        cs.set_marker_a(50);
        cs.set_marker_b(50);
        assert!(cs.frequency_from_delta(740_000.0).is_none());
    }

    #[test]
    fn signal_group_display() {
        assert_eq!(format!("{}", SignalGroup::Clock), "Clock");
        assert_eq!(format!("{}", SignalGroup::Data), "Data");
        assert_eq!(format!("{}", SignalGroup::Control), "Control");
    }

    #[test]
    fn default_signal_configs_has_all_signals() {
        let configs = default_signal_configs();
        assert_eq!(configs.len(), 6);
        let names: Vec<&str> = configs.iter().map(|c| c.name).collect();
        assert!(names.contains(&"PHI1"));
        assert!(names.contains(&"PHI2"));
        assert!(names.contains(&"SYNC"));
        assert!(names.contains(&"DATA"));
        assert!(names.contains(&"CM-ROM"));
        assert!(names.contains(&"CM-RAM"));
    }

    #[test]
    fn default_configs_all_visible() {
        let configs = default_signal_configs();
        assert!(configs.iter().all(|c| c.visible));
    }

    #[test]
    fn signal_groups_categorized() {
        let configs = default_signal_configs();
        let clock: Vec<_> = configs.iter().filter(|c| c.group == SignalGroup::Clock).collect();
        let data: Vec<_> = configs.iter().filter(|c| c.group == SignalGroup::Data).collect();
        let ctrl: Vec<_> = configs.iter().filter(|c| c.group == SignalGroup::Control).collect();
        assert_eq!(clock.len(), 2); // PHI1, PHI2
        assert_eq!(data.len(), 1); // DATA
        assert_eq!(ctrl.len(), 3); // SYNC, CM-ROM, CM-RAM
    }

    #[test]
    fn toggle_signal_visibility() {
        let trace = Arc::new(RwLock::new(SignalTrace::new()));
        let mut panel = WaveformPanel::new(trace);

        assert_eq!(panel.visible_signal_count(), 6);
        panel.toggle_signal("PHI1");
        assert_eq!(panel.visible_signal_count(), 5);
        panel.toggle_signal("PHI1");
        assert_eq!(panel.visible_signal_count(), 6);
    }

    #[test]
    fn zoom_clamping() {
        let trace = Arc::new(RwLock::new(SignalTrace::new()));
        let mut panel = WaveformPanel::new(trace);

        panel.set_zoom(0.001);
        assert!((panel.zoom() - 0.1).abs() < f32::EPSILON);

        panel.set_zoom(500.0);
        assert!((panel.zoom() - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn pixel_to_tick_conversion() {
        let trace = Arc::new(RwLock::new(SignalTrace::new()));
        let mut panel = WaveformPanel::new(trace);

        // Default zoom = 10.0 px/tick, scroll = 0
        assert_eq!(panel.pixel_to_tick(0.0), 0);
        assert_eq!(panel.pixel_to_tick(100.0), 10);

        // After scroll
        panel.scroll_x = 50.0;
        assert_eq!(panel.pixel_to_tick(0.0), 5);
        assert_eq!(panel.pixel_to_tick(100.0), 15);
    }

    #[test]
    fn signals_in_group_filter() {
        let trace = Arc::new(RwLock::new(SignalTrace::new()));
        let panel = WaveformPanel::new(trace);

        let clock = panel.signals_in_group(SignalGroup::Clock);
        assert_eq!(clock.len(), 2);
        assert!(clock.iter().all(|s| s.group == SignalGroup::Clock));
    }
}
