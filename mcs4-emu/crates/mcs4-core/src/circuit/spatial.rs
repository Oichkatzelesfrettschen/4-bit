//! Spatial coordinate system and transforms for die-level simulation.
//!
//! Maps pixel coordinates from photomicrograph extraction to physical
//! die coordinates in micrometers. Each chip has known die dimensions
//! used to calibrate the pixel-to-physical transform.
//!
//! The coordinate system places (0, 0) at the top-left corner of the
//! die image, with X increasing rightward and Y increasing downward
//! (matching image conventions).

use super::{
    bbox_to_geometry::BBox,
    graph::{CircuitGraph, MetalLayer, WireSegment},
};

/// Known die dimensions for the MCS-4 chip family.
///
/// Values from die photomicrograph measurements and Intel datasheets.
#[derive(Clone, Debug)]
pub struct DieDimensions {
    /// Die width (um).
    pub width_um: f64,
    /// Die height (um).
    pub height_um: f64,
    /// Chip name for identification.
    pub chip: &'static str,
}

/// Intel 4001 ROM die dimensions.
pub const DIE_4001: DieDimensions = DieDimensions {
    width_um: 3200.0,
    height_um: 3800.0,
    chip: "4001",
};

/// Intel 4002 RAM die dimensions.
pub const DIE_4002: DieDimensions = DieDimensions {
    width_um: 2600.0,
    height_um: 2200.0,
    chip: "4002",
};

/// Intel 4003 shift register die dimensions.
pub const DIE_4003: DieDimensions = DieDimensions {
    width_um: 2300.0,
    height_um: 1700.0,
    chip: "4003",
};

/// Intel 4004 CPU die dimensions.
pub const DIE_4004: DieDimensions = DieDimensions {
    width_um: 3200.0,
    height_um: 2300.0,
    chip: "4004",
};

/// Coordinate system for mapping between pixel space and die space.
///
/// Handles the transform from extraction image pixels to physical
/// micrometers on the die surface.
#[derive(Clone, Debug)]
pub struct DieCoordinateSystem {
    /// Scale factor: micrometers per pixel.
    pub scale_um_per_px: f64,
    /// Die dimensions in um.
    pub die: DieDimensions,
    /// Image width in pixels (for bounds checking).
    pub image_width_px: u32,
    /// Image height in pixels (for bounds checking).
    pub image_height_px: u32,
}

impl DieCoordinateSystem {
    /// Create a coordinate system from known die width and image width.
    ///
    /// Derives the scale factor from the horizontal dimension.
    pub fn from_die_and_image(die: DieDimensions, image_width_px: u32, image_height_px: u32) -> Self {
        let scale = die.width_um / f64::from(image_width_px);
        Self {
            scale_um_per_px: scale,
            die,
            image_width_px,
            image_height_px,
        }
    }

    /// Create with explicit scale factor.
    pub fn with_scale(die: DieDimensions, scale_um_per_px: f64, image_width_px: u32, image_height_px: u32) -> Self {
        Self {
            scale_um_per_px,
            die,
            image_width_px,
            image_height_px,
        }
    }

    /// Convert pixel coordinates to die coordinates (um).
    pub fn px_to_um(&self, px_x: f64, px_y: f64) -> (f64, f64) {
        (px_x * self.scale_um_per_px, px_y * self.scale_um_per_px)
    }

    /// Convert die coordinates (um) to pixel coordinates.
    pub fn um_to_px(&self, um_x: f64, um_y: f64) -> (f64, f64) {
        (um_x / self.scale_um_per_px, um_y / self.scale_um_per_px)
    }

    /// Compute the center point of a bounding box in die coordinates (um).
    pub fn bbox_center_um(&self, bbox: &BBox) -> (f64, f64) {
        let cx_px = f64::from(bbox.x) + f64::from(bbox.w) / 2.0;
        let cy_px = f64::from(bbox.y) + f64::from(bbox.h) / 2.0;
        self.px_to_um(cx_px, cy_px)
    }

    /// Check if a point (in um) is within the die bounds.
    ///
    /// Allows a small margin (5%) for rounding and edge effects.
    pub fn is_within_die(&self, x_um: f64, y_um: f64) -> bool {
        let margin_x = self.die.width_um * 0.05;
        let margin_y = self.die.height_um * 0.05;
        x_um >= -margin_x
            && x_um <= self.die.width_um + margin_x
            && y_um >= -margin_y
            && y_um <= self.die.height_um + margin_y
    }

    /// Die area in square micrometers.
    pub fn die_area_um2(&self) -> f64 {
        self.die.width_um * self.die.height_um
    }

    /// Die area in square millimeters.
    pub fn die_area_mm2(&self) -> f64 {
        self.die_area_um2() / 1e6
    }
}

/// Assign spatial coordinates to transistors in a circuit graph
/// from bounding box data.
///
/// # Arguments
/// * `graph` - circuit graph to annotate
/// * `bboxes` - one BBox per transistor (indexed by transistor ID)
/// * `coord_sys` - coordinate system for pixel-to-um conversion
///
/// # Returns
/// Number of transistors successfully mapped.
pub fn assign_transistor_positions(
    graph: &mut CircuitGraph,
    bboxes: &[BBox],
    coord_sys: &DieCoordinateSystem,
) -> usize {
    let mut mapped = 0;
    for trans in &mut graph.transistors {
        if let Some(bbox) = bboxes.get(trans.id) {
            let (cx, cy) = coord_sys.bbox_center_um(bbox);
            trans.x_center = Some(cx);
            trans.y_center = Some(cy);
            mapped += 1;
        }
    }
    mapped
}

/// Create wire segments from pairs of bounding boxes (source/drain contacts).
///
/// This is a simplified model: each wire is a straight segment from
/// the center of one bbox to the center of another.
pub fn create_wire_segment(
    coord_sys: &DieCoordinateSystem,
    bbox_a: &BBox,
    bbox_b: &BBox,
    width_um: f64,
    layer: MetalLayer,
) -> WireSegment {
    let (x0, y0) = coord_sys.bbox_center_um(bbox_a);
    let (x1, y1) = coord_sys.bbox_center_um(bbox_b);
    WireSegment {
        x0,
        y0,
        x1,
        y1,
        width: width_um,
        layer,
    }
}

/// Validate that all spatially-mapped transistors are within die bounds.
///
/// Returns a list of (transistor_id, x, y) for any out-of-bounds devices.
pub fn validate_positions(graph: &CircuitGraph, coord_sys: &DieCoordinateSystem) -> Vec<(usize, f64, f64)> {
    let mut violations = Vec::new();
    for trans in &graph.transistors {
        if let (Some(x), Some(y)) = (trans.x_center, trans.y_center) {
            if !coord_sys.is_within_die(x, y) {
                violations.push((trans.id, x, y));
            }
        }
    }
    violations
}

/// Compute the bounding box of all spatially-mapped transistors.
///
/// Returns (x_min, y_min, x_max, y_max) in um, or None if no
/// transistors have positions.
pub fn transistor_bounding_box(graph: &CircuitGraph) -> Option<(f64, f64, f64, f64)> {
    let positions: Vec<(f64, f64)> = graph
        .transistors
        .iter()
        .filter_map(|t| match (t.x_center, t.y_center) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        })
        .collect();

    if positions.is_empty() {
        return None;
    }

    let x_min = positions.iter().map(|(x, _)| *x).fold(f64::INFINITY, f64::min);
    let x_max = positions.iter().map(|(x, _)| *x).fold(f64::NEG_INFINITY, f64::max);
    let y_min = positions.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
    let y_max = positions.iter().map(|(_, y)| *y).fold(f64::NEG_INFINITY, f64::max);

    Some((x_min, y_min, x_max, y_max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::graph::TransistorKind;

    fn make_coord_sys_4004() -> DieCoordinateSystem {
        // 4004: 3200um x 2300um, image 3200x2300 -> scale = 1.0
        DieCoordinateSystem::from_die_and_image(DIE_4004, 3200, 2300)
    }

    #[test]
    fn scale_factor_derived_from_die_width() {
        let cs = make_coord_sys_4004();
        assert!(
            (cs.scale_um_per_px - 1.0).abs() < 1e-10,
            "Scale should be 1.0 for matching dimensions"
        );
    }

    #[test]
    fn px_to_um_identity_at_scale_1() {
        let cs = make_coord_sys_4004();
        let (x, y) = cs.px_to_um(100.0, 200.0);
        assert!((x - 100.0).abs() < 1e-10);
        assert!((y - 200.0).abs() < 1e-10);
    }

    #[test]
    fn px_to_um_with_fractional_scale() {
        let cs = DieCoordinateSystem::from_die_and_image(DIE_4003, 4600, 3400);
        // 2300 / 4600 = 0.5 um/px
        assert!((cs.scale_um_per_px - 0.5).abs() < 1e-10);
        let (x, y) = cs.px_to_um(1000.0, 2000.0);
        assert!((x - 500.0).abs() < 1e-10);
        assert!((y - 1000.0).abs() < 1e-10);
    }

    #[test]
    fn um_to_px_roundtrip() {
        let cs = make_coord_sys_4004();
        let (x_um, y_um) = (1600.0, 1150.0);
        let (px, py) = cs.um_to_px(x_um, y_um);
        let (x2, y2) = cs.px_to_um(px, py);
        assert!((x2 - x_um).abs() < 1e-10);
        assert!((y2 - y_um).abs() < 1e-10);
    }

    #[test]
    fn bbox_center_correct() {
        let cs = make_coord_sys_4004();
        let bbox = BBox::new(100, 200, 40, 20);
        let (cx, cy) = cs.bbox_center_um(&bbox);
        // Center: (100 + 20, 200 + 10) = (120, 210) in px = um at scale 1
        assert!((cx - 120.0).abs() < 1e-10);
        assert!((cy - 210.0).abs() < 1e-10);
    }

    #[test]
    fn within_die_bounds() {
        let cs = make_coord_sys_4004();
        assert!(cs.is_within_die(0.0, 0.0));
        assert!(cs.is_within_die(1600.0, 1150.0));
        assert!(cs.is_within_die(3200.0, 2300.0));
        // Slightly outside but within 5% margin
        assert!(cs.is_within_die(-100.0, 0.0));
        // Well outside
        assert!(!cs.is_within_die(-200.0, 0.0));
        assert!(!cs.is_within_die(0.0, 5000.0));
    }

    #[test]
    fn die_area_4004() {
        let cs = make_coord_sys_4004();
        let area_mm2 = cs.die_area_mm2();
        // 3.2mm x 2.3mm = 7.36 mm^2
        assert!((area_mm2 - 7.36).abs() < 0.01, "4004 die area = {:.2} mm^2", area_mm2);
    }

    #[test]
    fn assign_transistor_positions_from_bboxes() {
        let mut graph = CircuitGraph::new();
        let n0 = graph.add_node(0);
        let n1 = graph.add_node(1);
        let n2 = graph.add_node(2);
        graph.add_transistor(n0, n1, n2, TransistorKind::Enhancement, 10e-6, 10e-6);
        graph.add_transistor(n0, n1, n2, TransistorKind::Depletion, 10e-6, 20e-6);

        let bboxes = vec![BBox::new(100, 200, 40, 20), BBox::new(500, 600, 30, 10)];

        let cs = make_coord_sys_4004();
        let mapped = assign_transistor_positions(&mut graph, &bboxes, &cs);

        assert_eq!(mapped, 2);
        assert!(graph.transistors[0].x_center.is_some());
        assert!(graph.transistors[1].x_center.is_some());

        // First transistor center at (120, 210) um
        let (x0, y0) = (
            graph.transistors[0].x_center.expect("x"),
            graph.transistors[0].y_center.expect("y"),
        );
        assert!((x0 - 120.0).abs() < 1e-10);
        assert!((y0 - 210.0).abs() < 1e-10);
    }

    #[test]
    fn validate_positions_catches_out_of_bounds() {
        let mut graph = CircuitGraph::new();
        let n0 = graph.add_node(0);
        let n1 = graph.add_node(1);
        let n2 = graph.add_node(2);

        let t0 = graph.add_transistor(n0, n1, n2, TransistorKind::Enhancement, 10e-6, 10e-6);
        let t1 = graph.add_transistor(n0, n1, n2, TransistorKind::Enhancement, 10e-6, 10e-6);

        graph.transistors[t0].x_center = Some(1600.0);
        graph.transistors[t0].y_center = Some(1150.0);
        // Place second transistor way outside die
        graph.transistors[t1].x_center = Some(10000.0);
        graph.transistors[t1].y_center = Some(10000.0);

        let cs = make_coord_sys_4004();
        let violations = validate_positions(&graph, &cs);

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].0, t1);
    }

    #[test]
    fn validate_positions_clean_for_valid_circuit() {
        let mut graph = CircuitGraph::new();
        let n0 = graph.add_node(0);
        let n1 = graph.add_node(1);
        let n2 = graph.add_node(2);

        let t0 = graph.add_transistor(n0, n1, n2, TransistorKind::Enhancement, 10e-6, 10e-6);
        graph.transistors[t0].x_center = Some(1600.0);
        graph.transistors[t0].y_center = Some(1150.0);

        let cs = make_coord_sys_4004();
        let violations = validate_positions(&graph, &cs);
        assert!(violations.is_empty());
    }

    #[test]
    fn transistor_bounding_box_computed() {
        let mut graph = CircuitGraph::new();
        let n0 = graph.add_node(0);
        let n1 = graph.add_node(1);
        let n2 = graph.add_node(2);

        let t0 = graph.add_transistor(n0, n1, n2, TransistorKind::Enhancement, 10e-6, 10e-6);
        let t1 = graph.add_transistor(n0, n1, n2, TransistorKind::Enhancement, 10e-6, 10e-6);

        graph.transistors[t0].x_center = Some(100.0);
        graph.transistors[t0].y_center = Some(200.0);
        graph.transistors[t1].x_center = Some(500.0);
        graph.transistors[t1].y_center = Some(600.0);

        let (x_min, y_min, x_max, y_max) = transistor_bounding_box(&graph).expect("should have bbox");
        assert!((x_min - 100.0).abs() < 1e-10);
        assert!((y_min - 200.0).abs() < 1e-10);
        assert!((x_max - 500.0).abs() < 1e-10);
        assert!((y_max - 600.0).abs() < 1e-10);
    }

    #[test]
    fn transistor_bounding_box_empty() {
        let graph = CircuitGraph::new();
        assert!(transistor_bounding_box(&graph).is_none());
    }

    #[test]
    fn wire_segment_from_bboxes() {
        let cs = make_coord_sys_4004();
        let a = BBox::new(100, 100, 20, 20);
        let b = BBox::new(500, 500, 20, 20);

        let seg = create_wire_segment(&cs, &a, &b, 2.0, MetalLayer::Metal1);

        assert!((seg.x0 - 110.0).abs() < 1e-10);
        assert!((seg.y0 - 110.0).abs() < 1e-10);
        assert!((seg.x1 - 510.0).abs() < 1e-10);
        assert!((seg.y1 - 510.0).abs() < 1e-10);
        assert!((seg.width - 2.0).abs() < 1e-10);

        // Length should be sqrt((400)^2 + (400)^2) ~ 565.7 um
        let len = seg.length();
        assert!((len - 565.685).abs() < 1.0);
    }

    #[test]
    fn wire_segment_length_positive() {
        let cs = make_coord_sys_4004();
        let a = BBox::new(0, 0, 10, 10);
        let b = BBox::new(100, 0, 10, 10);

        let seg = create_wire_segment(&cs, &a, &b, 1.0, MetalLayer::Poly);
        assert!(seg.length() > 0.0);
    }

    #[test]
    fn die_dimensions_reasonable() {
        // All MCS-4 dies should be in the 1-5mm range
        for die in &[DIE_4001, DIE_4002, DIE_4003, DIE_4004] {
            assert!(
                die.width_um > 1000.0 && die.width_um < 5000.0,
                "{} width {} um out of range",
                die.chip,
                die.width_um
            );
            assert!(
                die.height_um > 1000.0 && die.height_um < 5000.0,
                "{} height {} um out of range",
                die.chip,
                die.height_um
            );
        }
    }

    #[test]
    fn explicit_scale_constructor() {
        let cs = DieCoordinateSystem::with_scale(DIE_4004, 0.8, 4000, 2875);
        assert!((cs.scale_um_per_px - 0.8).abs() < 1e-10);
        let (x, y) = cs.px_to_um(1000.0, 1000.0);
        assert!((x - 800.0).abs() < 1e-10);
        assert!((y - 800.0).abs() < 1e-10);
    }
}
