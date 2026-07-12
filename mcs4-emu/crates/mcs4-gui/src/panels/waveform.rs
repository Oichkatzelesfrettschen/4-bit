//! Provenance-aware waveform display for retained trace frames.

use eframe::egui::{self, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Stroke};
use mcs4_system::{TraceFrame, TraceLogic, TraceValue};

use crate::signal_trace::{FrameId, SignalTrace};

/// Signal group categories for organizing the waveform display.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalGroup {
    /// Bus-phase labels.
    Phase,
    /// Multi-bit values.
    Data,
    /// Four-state control signals.
    Control,
    /// Architectural state observations.
    State,
}

impl std::fmt::Display for SignalGroup {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Phase => formatter.write_str("Phase"),
            Self::Data => formatter.write_str("Data"),
            Self::Control => formatter.write_str("Control"),
            Self::State => formatter.write_str("State"),
        }
    }
}

/// Stable cursor identity for phase-level measurements.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CursorState {
    /// Frame selected by pointer hover or click.
    pub time_cursor: Option<FrameId>,
    /// First measurement endpoint.
    pub marker_a: Option<FrameId>,
    /// Second measurement endpoint.
    pub marker_b: Option<FrameId>,
}

/// Delta between two frames from one run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MarkerDelta {
    /// Absolute number of phase frames between endpoints.
    pub phase_delta: u64,
    /// First endpoint.
    pub marker_a: FrameId,
    /// Second endpoint.
    pub marker_b: FrameId,
}

impl CursorState {
    /// Set the active cursor to a stable frame identity.
    pub fn set_time_cursor(&mut self, frame: FrameId) {
        self.time_cursor = Some(frame);
    }

    /// Clear the active cursor.
    pub fn clear_time_cursor(&mut self) {
        self.time_cursor = None;
    }

    /// Set the first measurement endpoint.
    pub fn set_marker_a(&mut self, frame: FrameId) {
        self.marker_a = Some(frame);
    }

    /// Set the second measurement endpoint.
    pub fn set_marker_b(&mut self, frame: FrameId) {
        self.marker_b = Some(frame);
    }

    /// Clear both measurement endpoints.
    pub fn clear_markers(&mut self) {
        self.marker_a = None;
        self.marker_b = None;
    }

    /// Return a phase-frame delta only when both markers share one run.
    pub fn marker_delta(&self) -> Option<MarkerDelta> {
        let (marker_a, marker_b) = (self.marker_a?, self.marker_b?);
        (marker_a.run_id == marker_b.run_id).then_some(MarkerDelta {
            phase_delta: marker_a.sequence.abs_diff(marker_b.sequence),
            marker_a,
            marker_b,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SignalKind {
    Phase,
    Bits(&'static str),
    Logic(&'static str),
}

/// Signal visibility and drawing configuration.
#[derive(Clone, Debug)]
pub struct SignalConfig {
    /// Human-readable row label.
    pub name: &'static str,
    /// Group used by the controls summary.
    pub group: SignalGroup,
    /// Whether the row is visible.
    pub visible: bool,
    /// Row color.
    pub color: Color32,
    kind: SignalKind,
}

/// Default signals exposed by the behavioral MCS-4 trace contract.
pub fn default_signal_configs() -> Vec<SignalConfig> {
    vec![
        SignalConfig {
            name: "PHASE",
            group: SignalGroup::Phase,
            visible: true,
            color: Color32::LIGHT_BLUE,
            kind: SignalKind::Phase,
        },
        SignalConfig {
            name: "BUS",
            group: SignalGroup::Data,
            visible: true,
            color: Color32::GREEN,
            kind: SignalKind::Bits("mcs4.bus"),
        },
        SignalConfig {
            name: "BUS VALID",
            group: SignalGroup::Control,
            visible: true,
            color: Color32::LIGHT_GREEN,
            kind: SignalKind::Logic("mcs4.bus.valid"),
        },
        SignalConfig {
            name: "BUS CONTENTION",
            group: SignalGroup::Control,
            visible: true,
            color: Color32::RED,
            kind: SignalKind::Logic("mcs4.bus.contention"),
        },
        SignalConfig {
            name: "CM-ROM",
            group: SignalGroup::Control,
            visible: true,
            color: Color32::YELLOW,
            kind: SignalKind::Bits("mcs4.control.rom"),
        },
        SignalConfig {
            name: "CM-RAM",
            group: SignalGroup::Control,
            visible: true,
            color: Color32::from_rgb(255, 128, 0),
            kind: SignalKind::Bits("mcs4.control.ram"),
        },
        SignalConfig {
            name: "PC",
            group: SignalGroup::State,
            visible: true,
            color: Color32::from_rgb(180, 180, 255),
            kind: SignalKind::Bits("mcs4.cpu.pc"),
        },
        SignalConfig {
            name: "ACC",
            group: SignalGroup::State,
            visible: true,
            color: Color32::from_rgb(255, 180, 255),
            kind: SignalKind::Bits("mcs4.cpu.accumulator"),
        },
    ]
}

/// Waveform panel that renders immutable `TraceFrame` values.
pub struct WaveformPanel {
    zoom: f32,
    scroll_x: f32,
    row_height: f32,
    /// Stable measurement cursor state.
    pub cursors: CursorState,
    /// Configurable visible signal rows.
    pub signals: Vec<SignalConfig>,
}

const LABEL_WIDTH: f32 = 118.0;

impl WaveformPanel {
    /// Build a panel with the standard MCS-4 frame rows.
    pub fn new() -> Self {
        Self {
            zoom: 52.0,
            scroll_x: 0.0,
            row_height: 28.0,
            cursors: CursorState::default(),
            signals: default_signal_configs(),
        }
    }

    /// Return the current pixel width assigned to one frame.
    pub const fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Set the frame width, retaining a usable visual range.
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(16.0, 240.0);
    }

    /// Return the horizontal viewport position in pixels.
    pub const fn scroll_x(&self) -> f32 {
        self.scroll_x
    }

    /// Convert one viewport pixel coordinate into a retained frame offset.
    pub fn pixel_to_frame_offset(&self, pixel_x: f32) -> usize {
        ((self.scroll_x + pixel_x) / self.zoom).max(0.0) as usize
    }

    /// Return the count of enabled waveform rows.
    pub fn visible_signal_count(&self) -> usize {
        self.signals.iter().filter(|signal| signal.visible).count()
    }

    /// Toggle visibility by its stable human-readable row name.
    pub fn toggle_signal(&mut self, name: &str) {
        if let Some(signal) = self.signals.iter_mut().find(|signal| signal.name == name) {
            signal.visible = !signal.visible;
        }
    }

    /// Return signal configurations in one display group.
    pub fn signals_in_group(&self, group: SignalGroup) -> Vec<&SignalConfig> {
        self.signals.iter().filter(|signal| signal.group == group).collect()
    }

    /// Render the current retained trace without taking ownership of it.
    pub fn show(&mut self, ui: &mut egui::Ui, trace: &SignalTrace) {
        ui.heading("Waveform Viewer");
        self.show_controls(ui, trace);

        if trace.is_empty() {
            ui.label("No post-phase trace frames are retained.");
            return;
        }

        let visible_rows = self.visible_signal_count();
        let height = (visible_rows.max(1) as f32 * self.row_height + 40.0).max(180.0);
        let (response, painter) = ui.allocate_painter(egui::vec2(ui.available_width(), height), Sense::drag());

        if response.dragged() {
            self.scroll_x = (self.scroll_x - response.drag_delta().x).max(0.0);
        }

        let start_offset = self.pixel_to_frame_offset(0.0);
        let visible_frame_count = (self.data_width(response.rect) / self.zoom).ceil() as usize + 2;
        let frames: Vec<_> = trace
            .iter()
            .skip(start_offset)
            .take(visible_frame_count)
            .cloned()
            .collect();
        if frames.is_empty() {
            painter.text(
                response.rect.center(),
                Align2::CENTER_CENTER,
                "Viewport is beyond retained trace frames",
                FontId::proportional(14.0),
                Color32::YELLOW,
            );
            return;
        }

        if let Some(position) = response.hover_pos() {
            if let Some(frame_offset) = self.frame_offset_at_pointer(response.rect, position.x) {
                if let Some(frame) = frames.get(frame_offset) {
                    self.cursors.set_time_cursor(FrameId::from(frame));
                }
            }
        }

        self.draw_frames(&painter, response.rect, &frames);
    }

    fn data_origin(&self, rect: Rect) -> f32 {
        rect.min.x + LABEL_WIDTH
    }

    fn data_width(&self, rect: Rect) -> f32 {
        (rect.max.x - self.data_origin(rect)).max(self.zoom)
    }

    fn frame_offset_at_pointer(&self, rect: Rect, pointer_x: f32) -> Option<usize> {
        let data_x = pointer_x - self.data_origin(rect);
        (data_x >= 0.0).then(|| (data_x / self.zoom).floor() as usize)
    }

    fn show_controls(&mut self, ui: &mut egui::Ui, trace: &SignalTrace) {
        ui.horizontal(|ui| {
            if ui.button("- Zoom").clicked() {
                self.set_zoom(self.zoom * 0.8);
            }
            if ui.button("+ Zoom").clicked() {
                self.set_zoom(self.zoom * 1.25);
            }
            ui.label(format!("{:.0} px/frame", self.zoom));
            ui.separator();
            if ui.button("Set A").clicked() {
                if let Some(frame) = self.cursors.time_cursor {
                    self.cursors.set_marker_a(frame);
                }
            }
            if ui.button("Set B").clicked() {
                if let Some(frame) = self.cursors.time_cursor {
                    self.cursors.set_marker_b(frame);
                }
            }
            if ui.button("Clear markers").clicked() {
                self.cursors.clear_markers();
            }

            match self.cursors.marker_delta() {
                Some(delta) => ui.label(format!(
                    "A r{}:{} B r{}:{} delta {} phases",
                    delta.marker_a.run_id,
                    delta.marker_a.sequence,
                    delta.marker_b.run_id,
                    delta.marker_b.sequence,
                    delta.phase_delta
                )),
                None if self.cursors.marker_a.is_some() && self.cursors.marker_b.is_some() => {
                    ui.colored_label(Color32::YELLOW, "markers span different runs")
                }
                None => ui.label("set two markers for a phase delta"),
            };
        });

        ui.horizontal_wrapped(|ui| {
            for group in [
                SignalGroup::Phase,
                SignalGroup::Data,
                SignalGroup::Control,
                SignalGroup::State,
            ] {
                let signals = self.signals_in_group(group);
                let visible = signals.iter().filter(|signal| signal.visible).count();
                ui.label(format!("{group}: {visible}/{}", signals.len()));
            }
            let retention = trace.retention();
            if retention.dropped_frame_count != 0 {
                ui.colored_label(
                    Color32::YELLOW,
                    format!("{} frames evicted from UI retention", retention.dropped_frame_count),
                );
            }
        });

        ui.horizontal_wrapped(|ui| {
            for signal in &mut self.signals {
                ui.checkbox(&mut signal.visible, signal.name);
            }
        });

        if let Some(cursor) = self.cursors.time_cursor {
            if trace.frame(cursor).is_none() {
                ui.colored_label(
                    Color32::YELLOW,
                    format!(
                        "cursor r{}:{} is outside retained history",
                        cursor.run_id, cursor.sequence
                    ),
                );
            }
        }
    }

    fn draw_frames(&self, painter: &Painter, rect: Rect, frames: &[TraceFrame]) {
        let rows: Vec<_> = self.signals.iter().filter(|signal| signal.visible).collect();
        let data_origin = self.data_origin(rect);
        let clipped_frame_count = (self.data_width(rect) / self.zoom).ceil() as usize;
        let frames = &frames[..frames.len().min(clipped_frame_count)];

        painter.rect_filled(rect, 2.0, Color32::from_gray(18));
        painter.line_segment(
            [Pos2::new(data_origin, rect.min.y), Pos2::new(data_origin, rect.max.y)],
            Stroke::new(1.0, Color32::DARK_GRAY),
        );

        for (row_index, signal) in rows.iter().enumerate() {
            let row_top = rect.min.y + row_index as f32 * self.row_height;
            let row_rect = Rect::from_min_max(
                Pos2::new(rect.min.x, row_top),
                Pos2::new(rect.max.x, row_top + self.row_height),
            );
            painter.line_segment(
                [
                    Pos2::new(rect.min.x, row_rect.max.y),
                    Pos2::new(rect.max.x, row_rect.max.y),
                ],
                Stroke::new(1.0, Color32::from_gray(45)),
            );
            painter.text(
                Pos2::new(rect.min.x + 4.0, row_rect.center().y),
                Align2::LEFT_CENTER,
                signal.name,
                FontId::monospace(11.0),
                signal.color,
            );
            self.draw_signal_row(painter, row_rect, data_origin, frames, signal);
        }

        self.draw_cursor(painter, rect, data_origin, frames);
    }

    fn draw_signal_row(
        &self,
        painter: &Painter,
        row_rect: Rect,
        data_origin: f32,
        frames: &[TraceFrame],
        signal: &SignalConfig,
    ) {
        for (index, frame) in frames.iter().enumerate() {
            let x_start = data_origin + index as f32 * self.zoom;
            let x_end = x_start + self.zoom;
            painter.line_segment(
                [Pos2::new(x_start, row_rect.min.y), Pos2::new(x_start, row_rect.max.y)],
                Stroke::new(1.0, Color32::from_gray(38)),
            );
            match signal.kind {
                SignalKind::Phase => self.draw_phase(painter, row_rect, x_start, frame, signal.color),
                SignalKind::Bits(path) => self.draw_bits(painter, row_rect, x_start, x_end, frame, path, signal.color),
                SignalKind::Logic(path) => {
                    self.draw_logic(painter, row_rect, x_start, x_end, frame, path, signal.color)
                }
            }
        }
    }

    fn draw_phase(&self, painter: &Painter, row: Rect, x_start: f32, frame: &TraceFrame, color: Color32) {
        let label = phase_label(frame);
        painter.text(
            Pos2::new(x_start + 4.0, row.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::monospace(11.0),
            color,
        );
    }

    fn draw_bits(
        &self,
        painter: &Painter,
        row: Rect,
        x_start: f32,
        _x_end: f32,
        frame: &TraceFrame,
        path: &str,
        color: Color32,
    ) {
        let label = match frame.signal(path).map(|signal| &signal.value) {
            Some(TraceValue::Bits { width, value }) => {
                format!("{value:0width$X}", width = usize::from(*width).div_ceil(4))
            }
            Some(TraceValue::Unavailable { .. }) => "-".to_owned(),
            Some(TraceValue::Logic { value }) => logic_label(*value).to_owned(),
            Some(TraceValue::Voltage { volts }) => format!("{volts:.3} V"),
            None => "?".to_owned(),
        };
        painter.text(
            Pos2::new(x_start + 4.0, row.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::monospace(11.0),
            color,
        );
    }

    fn draw_logic(
        &self,
        painter: &Painter,
        row: Rect,
        x_start: f32,
        x_end: f32,
        frame: &TraceFrame,
        path: &str,
        color: Color32,
    ) {
        let value = match frame.signal(path).map(|signal| &signal.value) {
            Some(TraceValue::Logic { value }) => *value,
            _ => TraceLogic::X,
        };
        let y = match value {
            TraceLogic::One => row.min.y + 6.0,
            TraceLogic::Zero => row.max.y - 6.0,
            TraceLogic::X | TraceLogic::Z => row.center().y,
        };
        let row_color = match value {
            TraceLogic::X | TraceLogic::Z => Color32::YELLOW,
            TraceLogic::Zero | TraceLogic::One => color,
        };
        painter.line_segment(
            [Pos2::new(x_start, y), Pos2::new(x_end, y)],
            Stroke::new(2.0, row_color),
        );
        if matches!(value, TraceLogic::X | TraceLogic::Z) {
            painter.text(
                Pos2::new(x_start + 4.0, y),
                Align2::LEFT_CENTER,
                logic_label(value),
                FontId::monospace(10.0),
                row_color,
            );
        }
    }

    fn draw_cursor(&self, painter: &Painter, rect: Rect, data_origin: f32, frames: &[TraceFrame]) {
        let Some(cursor) = self.cursors.time_cursor else {
            return;
        };
        let Some(index) = frames.iter().position(|frame| FrameId::from(frame) == cursor) else {
            return;
        };
        let x = data_origin + index as f32 * self.zoom + self.zoom * 0.5;
        painter.line_segment(
            [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
            Stroke::new(1.0, Color32::WHITE),
        );
    }
}

impl Default for WaveformPanel {
    fn default() -> Self {
        Self::new()
    }
}

fn logic_label(value: TraceLogic) -> &'static str {
    match value {
        TraceLogic::Zero => "0",
        TraceLogic::One => "1",
        TraceLogic::X => "X",
        TraceLogic::Z => "Z",
    }
}

fn phase_label(frame: &TraceFrame) -> String {
    if let Some(phase) = frame.phase.as_ref() {
        return format!("{:?}", phase.completed_phase);
    }
    match frame.signal("mcs4.phase").map(|signal| &signal.value) {
        Some(TraceValue::Bits { width: 3, value }) => match value {
            0 => "A1".to_owned(),
            1 => "A2".to_owned(),
            2 => "A3".to_owned(),
            3 => "M1".to_owned(),
            4 => "M2".to_owned(),
            5 => "X1".to_owned(),
            6 => "X2".to_owned(),
            7 => "X3".to_owned(),
            _ => "?".to_owned(),
        },
        _ => "-".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use mcs4_system::{Mcs4System, ReplaySession};

    use super::*;

    fn frames(count: usize) -> Vec<TraceFrame> {
        let mut session = ReplaySession::<Mcs4System>::new();
        (0..count).map(|_| session.step_phase().expect("step phase")).collect()
    }

    #[test]
    fn marker_delta_uses_phase_unique_frame_identity() {
        let frames = frames(2);
        let mut cursors = CursorState::default();
        cursors.set_marker_a(FrameId::from(&frames[0]));
        cursors.set_marker_b(FrameId::from(&frames[1]));

        assert_eq!(cursors.marker_delta().expect("delta").phase_delta, 1);
    }

    #[test]
    fn marker_delta_rejects_cross_run_measurements() {
        let mut cursors = CursorState::default();
        cursors.set_marker_a(FrameId { run_id: 1, sequence: 8 });
        cursors.set_marker_b(FrameId { run_id: 2, sequence: 1 });

        assert_eq!(cursors.marker_delta(), None);
    }

    #[test]
    fn panel_uses_frame_offsets_not_machine_cycle_ticks() {
        let mut panel = WaveformPanel::new();
        panel.set_zoom(40.0);
        assert_eq!(panel.pixel_to_frame_offset(0.0), 0);
        assert_eq!(panel.pixel_to_frame_offset(119.0), 2);
        assert_eq!(panel.pixel_to_frame_offset(120.0), 3);
    }

    #[test]
    fn pointer_hit_testing_excludes_the_label_column() {
        let panel = WaveformPanel::new();
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(400.0, 180.0));

        assert_eq!(panel.frame_offset_at_pointer(rect, LABEL_WIDTH - 1.0), None);
        assert_eq!(panel.frame_offset_at_pointer(rect, LABEL_WIDTH), Some(0));
        assert_eq!(panel.frame_offset_at_pointer(rect, LABEL_WIDTH + panel.zoom()), Some(1));
    }

    #[test]
    fn panel_exposes_expected_trace_rows() {
        let panel = WaveformPanel::new();
        assert!(panel.visible_signal_count() >= 8);
        assert_eq!(panel.signals_in_group(SignalGroup::Phase).len(), 1);
    }

    #[test]
    fn system_adapter_phase_signal_renders_without_a_behavioral_phase_record() {
        let frame: TraceFrame = serde_json::from_str(include_str!(
            "../../../mcs4-system/fixtures/traces/mcs4-system-verilator-frame-v1.jsonl"
        ))
        .expect("parse system adapter frame");
        assert_eq!(phase_label(&frame), "A1");
    }
}
