//! 7-segment display panel driven by the shared machine's RAM output port.

use eframe::egui::{self, Color32, Rect, Sense, Vec2};

use crate::session::SevenSegView;

/// Segment bit positions, matching `mcs4_periph::seven_seg::segment`.
const SEG_A: u8 = 0x01; // top
const SEG_B: u8 = 0x02; // upper-right
const SEG_C: u8 = 0x04; // lower-right
const SEG_D: u8 = 0x08; // bottom
const SEG_E: u8 = 0x10; // lower-left
const SEG_F: u8 = 0x20; // upper-left
const SEG_G: u8 = 0x40; // middle

/// Renders one 7-segment digit as a lit/unlit glyph.
pub struct SevenSegPanel;

impl SevenSegPanel {
    /// Draw the display for the latest peripheral view.
    pub fn show(&self, ui: &mut egui::Ui, view: &SevenSegView) {
        ui.heading("7-segment display");
        ui.monospace(format!(
            "RAM output port (chip 0): {:X}  ->  '{}'",
            view.value, view.ascii
        ));

        let (rect, _) = ui.allocate_exact_size(Vec2::new(64.0, 104.0), Sense::hover());
        let painter = ui.painter_at(rect);
        let on = Color32::from_rgb(255, 72, 56);
        let off = Color32::from_rgb(52, 28, 26);
        let segments = view.segments;

        let margin = 12.0;
        let thickness = 9.0;
        let left = rect.left() + margin;
        let right = rect.right() - margin;
        let top = rect.top() + margin;
        let middle = rect.center().y;
        let bottom = rect.bottom() - margin;

        let bar = |a: egui::Pos2, b: egui::Pos2, mask: u8| {
            let color = if segments & mask != 0 { on } else { off };
            let bar_rect = Rect::from_two_pos(a, b).expand(thickness / 2.0);
            painter.rect_filled(bar_rect, 2.0, color);
        };

        // Horizontal segments (a top, g middle, d bottom).
        bar(egui::pos2(left, top), egui::pos2(right, top), SEG_A);
        bar(egui::pos2(left, middle), egui::pos2(right, middle), SEG_G);
        bar(egui::pos2(left, bottom), egui::pos2(right, bottom), SEG_D);
        // Vertical segments (f/b upper, e/c lower).
        bar(egui::pos2(left, top), egui::pos2(left, middle), SEG_F);
        bar(egui::pos2(right, top), egui::pos2(right, middle), SEG_B);
        bar(egui::pos2(left, middle), egui::pos2(left, bottom), SEG_E);
        bar(egui::pos2(right, middle), egui::pos2(right, bottom), SEG_C);
    }
}

#[cfg(test)]
mod tests {
    use super::{SEG_A, SEG_B, SEG_C, SEG_D, SEG_E, SEG_F, SEG_G};

    #[test]
    fn segment_masks_match_the_peripheral_layout() {
        // The panel's local masks must equal the driver's segment constants so
        // the glyph geometry follows the published segment byte.
        assert_eq!(
            [SEG_A, SEG_B, SEG_C, SEG_D, SEG_E, SEG_F, SEG_G],
            [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40]
        );
    }
}
