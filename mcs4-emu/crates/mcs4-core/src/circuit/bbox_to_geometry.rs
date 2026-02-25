//! Bounding box to transistor geometry conversion.
//!
//! Extracted netlists contain pixel bounding boxes for each transistor
//! from photomicrograph analysis. This module converts those pixel
//! dimensions to physical gate width and length in meters using a
//! calibratable scale factor.

/// Pixel bounding box from netlist extraction.
#[derive(Clone, Copy, Debug)]
pub struct BBox {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl BBox {
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    /// Area in pixels squared.
    pub fn area(&self) -> u64 {
        u64::from(self.w) * u64::from(self.h)
    }
}

/// Scale factor from pixels to micrometers.
///
/// Calibrated by matching known die dimensions to image dimensions.
/// For the 4003 die (~2.3mm x ~1.7mm) on a typical extraction image.
///
/// Default: 1 pixel ~ 1.0 um (approximate, needs calibration per chip).
pub const DEFAULT_SCALE_UM_PER_PX: f64 = 1.0;

/// Convert pixel bounding box to physical gate dimensions (meters).
///
/// The gate width is taken as the longer bbox dimension and the gate
/// length as the shorter dimension, since transistor channels are
/// typically drawn with width > length.
///
/// # Arguments
/// * `bbox` - pixel bounding box
/// * `scale_um_per_px` - micrometers per pixel calibration factor
///
/// # Returns
/// (width_m, length_m) in meters
pub fn bbox_to_wl(bbox: &BBox, scale_um_per_px: f64) -> (f64, f64) {
    let w_um = f64::from(bbox.w) * scale_um_per_px;
    let h_um = f64::from(bbox.h) * scale_um_per_px;

    // Gate width is the longer dimension, length is the shorter
    let (width, length) = if w_um >= h_um { (h_um, w_um) } else { (w_um, h_um) };

    // Clamp minimum to process minimum feature size
    let width_m = width.max(1.0) * 1e-6;
    let length_m = length.max(1.0) * 1e-6;

    (width_m, length_m)
}

/// Estimate scale factor from known die dimensions.
///
/// # Arguments
/// * `die_width_mm` - known die width in millimeters
/// * `image_width_px` - image width in pixels
///
/// # Returns
/// Scale factor in um/pixel
pub fn calibrate_scale(die_width_mm: f64, image_width_px: u32) -> f64 {
    let die_width_um = die_width_mm * 1000.0;
    die_width_um / f64::from(image_width_px)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bbox_area() {
        let bb = BBox::new(0, 0, 41, 105);
        assert_eq!(bb.area(), 4305);
    }

    #[test]
    fn bbox_to_wl_assigns_longer_as_length() {
        // bbox w=41, h=105 -> h is longer -> length = h * scale
        let bb = BBox::new(0, 0, 41, 105);
        let (w, l) = bbox_to_wl(&bb, 1.0);
        // width = min(41,105) = 41um, length = max(41,105) = 105um
        assert!((w - 41e-6).abs() < 1e-9);
        assert!((l - 105e-6).abs() < 1e-9);
    }

    #[test]
    fn bbox_to_wl_with_scale() {
        let bb = BBox::new(0, 0, 80, 40);
        let (w, l) = bbox_to_wl(&bb, 0.5);
        // w_um = 80*0.5=40, h_um=40*0.5=20
        // width (shorter) = 20um, length (longer) = 40um
        assert!((w - 20e-6).abs() < 1e-9);
        assert!((l - 40e-6).abs() < 1e-9);
    }

    #[test]
    fn minimum_clamped_to_1um() {
        let bb = BBox::new(0, 0, 0, 0);
        let (w, l) = bbox_to_wl(&bb, 1.0);
        // Both should be clamped to 1um
        assert!((w - 1e-6).abs() < 1e-9);
        assert!((l - 1e-6).abs() < 1e-9);
    }

    #[test]
    fn calibrate_scale_4003() {
        // 4003 die: ~2.3mm wide, typical image ~2300px wide
        let scale = calibrate_scale(2.3, 2300);
        assert!(
            (scale - 1.0).abs() < 0.01,
            "Scale should be ~1.0 um/px, got {:.3}",
            scale
        );
    }

    #[test]
    fn calibrate_scale_4004() {
        // 4004 die: ~3mm wide, typical image ~3000px wide
        let scale = calibrate_scale(3.0, 3000);
        assert!((scale - 1.0).abs() < 0.01);
    }
}
