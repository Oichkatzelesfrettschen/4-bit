//! Layout-aware parasitic extraction from wire geometry.
//!
//! Computes wire resistance and capacitance from physical layout data
//! (wire segments with coordinates, widths, and layer assignments),
//! replacing default lumped estimates with layout-derived values.
//!
//! Uses the interconnect models from `process::interconnect` to compute
//! per-segment R and C, then assembles RC trees for Elmore delay estimation.

use crate::circuit::graph::{CircuitGraph, MetalLayer, WireSegment};
use crate::process::interconnect;

/// Physical layer parameters for parasitic computation.
///
/// Thickness values are typical for the Intel 10um pMOS process.
#[derive(Clone, Debug)]
pub struct LayerParams {
    /// Polysilicon thickness (m).
    pub poly_thickness: f64,
    /// Poly-to-substrate dielectric thickness (m). Gate oxide under poly.
    pub poly_dielectric: f64,
    /// Diffusion junction depth used as effective thickness (m).
    pub diff_thickness: f64,
    /// Diffusion-to-substrate effective dielectric thickness (m).
    pub diff_dielectric: f64,
    /// Metal (Al) thickness (m).
    pub metal_thickness: f64,
    /// Metal-to-substrate dielectric thickness (m). Field oxide.
    pub metal_dielectric: f64,
    /// Minimum spacing for coupling capacitance (um).
    pub min_spacing: f64,
}

impl Default for LayerParams {
    fn default() -> Self {
        Self {
            poly_thickness: 0.4e-6,    // ~400nm polysilicon
            poly_dielectric: 80e-9,     // gate oxide (also field oxide under poly routing)
            diff_thickness: 2e-6,       // junction depth
            diff_dielectric: 0.5e-6,    // field oxide over diffusion
            metal_thickness: 1.0e-6,    // ~1um aluminum
            metal_dielectric: 1.0e-6,   // field oxide + interlayer dielectric
            min_spacing: 10.0,          // 10um minimum spacing
        }
    }
}

/// Extracted parasitic R and C for a single wire segment.
#[derive(Clone, Copy, Debug)]
pub struct SegmentParasitics {
    /// Wire resistance (ohms).
    pub resistance: f64,
    /// Wire-to-substrate capacitance (F).
    pub capacitance: f64,
    /// Segment length (m).
    pub length_m: f64,
}

/// Extracted parasitics for an entire net (node).
#[derive(Clone, Debug)]
pub struct NetParasitics {
    /// Node index in the circuit graph.
    pub node_idx: usize,
    /// Per-segment R and C values.
    pub segments: Vec<SegmentParasitics>,
    /// Total resistance of the net (ohms), sum of all segments.
    pub total_resistance: f64,
    /// Total capacitance of the net (F), sum of all segment caps.
    pub total_capacitance: f64,
    /// Elmore delay from driver to farthest load (seconds).
    pub elmore_delay: f64,
}

/// Sheet resistance for a given metal layer.
fn sheet_resistance(layer: &MetalLayer) -> f64 {
    match layer {
        MetalLayer::Poly => interconnect::R_SHEET_POLY,
        MetalLayer::Diffusion => interconnect::R_SHEET_DIFFUSION,
        MetalLayer::Metal1 => interconnect::R_SHEET_METAL,
    }
}

/// Extract parasitics for a single wire segment.
///
/// Coordinates in the WireSegment are in um; results in SI (ohms, farads).
pub fn extract_segment(seg: &WireSegment, params: &LayerParams) -> SegmentParasitics {
    // Convert um to meters
    let length_m = seg.length() * 1e-6;
    let width_m = seg.width * 1e-6;

    // Resistance: R_sheet * L / W
    let r_sheet = sheet_resistance(&seg.layer);
    let resistance = if width_m > 0.0 {
        interconnect::wire_resistance(r_sheet, length_m, width_m)
    } else {
        0.0
    };

    // Capacitance: parallel-plate + fringing
    let (thickness, dielectric) = match seg.layer {
        MetalLayer::Poly => (params.poly_thickness, params.poly_dielectric),
        MetalLayer::Diffusion => (params.diff_thickness, params.diff_dielectric),
        MetalLayer::Metal1 => (params.metal_thickness, params.metal_dielectric),
    };
    let capacitance = if length_m > 0.0 && dielectric > 0.0 {
        interconnect::wire_capacitance(width_m, length_m, thickness, dielectric)
    } else {
        0.0
    };

    SegmentParasitics {
        resistance,
        capacitance,
        length_m,
    }
}

/// Extract parasitics for all wire segments on a given node.
///
/// Returns the total R, C, and Elmore delay for the net.
pub fn extract_net(
    graph: &CircuitGraph,
    node_idx: usize,
    params: &LayerParams,
) -> NetParasitics {
    let node = &graph.nodes[node_idx];

    let segments: Vec<SegmentParasitics> = node
        .wire_segments
        .iter()
        .map(|seg| extract_segment(seg, params))
        .collect();

    let total_resistance: f64 = segments.iter().map(|s| s.resistance).sum();
    let total_capacitance: f64 = segments.iter().map(|s| s.capacitance).sum();

    // Elmore delay: treat segments as a chain in order
    let rc_chain: Vec<(f64, f64)> = segments.iter().map(|s| (s.resistance, s.capacitance)).collect();
    let elmore = interconnect::elmore_delay_chain(&rc_chain);

    NetParasitics {
        node_idx,
        segments,
        total_resistance,
        total_capacitance,
        elmore_delay: elmore,
    }
}

/// Extract parasitics for all nodes in a circuit graph.
///
/// Only processes nodes that have wire segment data. Nodes without
/// wire segments retain their existing (default) capacitance values.
pub fn extract_all(graph: &CircuitGraph, params: &LayerParams) -> Vec<NetParasitics> {
    graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| !n.wire_segments.is_empty())
        .map(|(idx, _)| extract_net(graph, idx, params))
        .collect()
}

/// Update a circuit graph's node capacitances from extracted parasitics.
///
/// Replaces the default lumped capacitance values with layout-computed ones.
/// Only updates nodes that have wire segment data.
pub fn apply_to_graph(graph: &mut CircuitGraph, params: &LayerParams) {
    for idx in 0..graph.nodes.len() {
        if graph.nodes[idx].wire_segments.is_empty() {
            continue;
        }
        let net = extract_net(graph, idx, params);
        graph.nodes[idx].capacitance = net.total_capacitance;
    }
}

/// Estimate coupling capacitance between two adjacent wire segments.
///
/// Uses a parallel-plate approximation for the lateral coupling:
/// C_coupling = epsilon_ox * overlap_length * min(h1, h2) / spacing
///
/// where overlap_length is the length of parallel run and spacing is
/// the edge-to-edge distance.
///
/// Returns 0 if segments are on different layers or do not run in parallel.
pub fn coupling_capacitance(
    seg_a: &WireSegment,
    seg_b: &WireSegment,
    params: &LayerParams,
) -> f64 {
    // Only compute coupling for same-layer segments
    if std::mem::discriminant(&seg_a.layer) != std::mem::discriminant(&seg_b.layer) {
        return 0.0;
    }

    // Compute bounding overlap in the longer axis
    // For simplicity, check if segments run approximately parallel (both horizontal or both vertical)
    let dx_a = (seg_a.x1 - seg_a.x0).abs();
    let dy_a = (seg_a.y1 - seg_a.y0).abs();
    let dx_b = (seg_b.x1 - seg_b.x0).abs();
    let dy_b = (seg_b.y1 - seg_b.y0).abs();

    let a_horizontal = dx_a > dy_a;
    let b_horizontal = dx_b > dy_b;

    if a_horizontal != b_horizontal {
        return 0.0; // perpendicular, minimal coupling
    }

    // Compute parallel run overlap and spacing
    let (overlap_length, spacing) = if a_horizontal {
        // Both horizontal: overlap in X, spacing in Y
        let x_min = seg_a.x0.min(seg_a.x1).max(seg_b.x0.min(seg_b.x1));
        let x_max = seg_a.x0.max(seg_a.x1).min(seg_b.x0.max(seg_b.x1));
        let overlap = (x_max - x_min).max(0.0);

        let y_center_a = (seg_a.y0 + seg_a.y1) / 2.0;
        let y_center_b = (seg_b.y0 + seg_b.y1) / 2.0;
        let center_dist = (y_center_a - y_center_b).abs();
        let edge_spacing = center_dist - (seg_a.width + seg_b.width) / 2.0;

        (overlap, edge_spacing.max(params.min_spacing))
    } else {
        // Both vertical: overlap in Y, spacing in X
        let y_min = seg_a.y0.min(seg_a.y1).max(seg_b.y0.min(seg_b.y1));
        let y_max = seg_a.y0.max(seg_a.y1).min(seg_b.y0.max(seg_b.y1));
        let overlap = (y_max - y_min).max(0.0);

        let x_center_a = (seg_a.x0 + seg_a.x1) / 2.0;
        let x_center_b = (seg_b.x0 + seg_b.x1) / 2.0;
        let center_dist = (x_center_a - x_center_b).abs();
        let edge_spacing = center_dist - (seg_a.width + seg_b.width) / 2.0;

        (overlap, edge_spacing.max(params.min_spacing))
    };

    if overlap_length <= 0.0 || spacing <= 0.0 {
        return 0.0;
    }

    // Convert um to meters
    let overlap_m = overlap_length * 1e-6;
    let spacing_m = spacing * 1e-6;

    // Conductor height for lateral coupling
    let height = match seg_a.layer {
        MetalLayer::Poly => params.poly_thickness,
        MetalLayer::Diffusion => params.diff_thickness,
        MetalLayer::Metal1 => params.metal_thickness,
    };

    // Parallel-plate lateral coupling: C = eps * h * L / s
    let epsilon_ox = crate::process::silicon::EPSILON_OX;
    epsilon_ox * height * overlap_m / spacing_m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::graph::CircuitGraph;

    fn metal_segment(x0: f64, y0: f64, x1: f64, y1: f64, width: f64) -> WireSegment {
        WireSegment {
            x0, y0, x1, y1, width,
            layer: MetalLayer::Metal1,
        }
    }

    fn poly_segment(x0: f64, y0: f64, x1: f64, y1: f64, width: f64) -> WireSegment {
        WireSegment {
            x0, y0, x1, y1, width,
            layer: MetalLayer::Poly,
        }
    }

    #[test]
    fn metal_segment_low_resistance() {
        let seg = metal_segment(0.0, 0.0, 100.0, 0.0, 10.0);
        let params = LayerParams::default();
        let p = extract_segment(&seg, &params);

        // 100um metal, 10um wide: R = 0.05 * 100e-6 / 10e-6 = 0.5 ohm
        assert!(p.resistance > 0.0 && p.resistance < 5.0,
            "Metal R = {:.3} ohm", p.resistance);
    }

    #[test]
    fn poly_segment_higher_resistance() {
        let seg = poly_segment(0.0, 0.0, 100.0, 0.0, 10.0);
        let params = LayerParams::default();
        let p = extract_segment(&seg, &params);

        // 100um poly, 10um wide: R = 40 * 100e-6 / 10e-6 = 400 ohm
        assert!(p.resistance > 100.0,
            "Poly R = {:.1} ohm, should be >> metal", p.resistance);
    }

    #[test]
    fn segment_capacitance_positive() {
        let seg = metal_segment(0.0, 0.0, 100.0, 0.0, 10.0);
        let params = LayerParams::default();
        let p = extract_segment(&seg, &params);

        assert!(p.capacitance > 0.0, "Capacitance should be positive");
        assert!(p.capacitance < 1e-12, "100um wire cap should be < 1pF");
    }

    #[test]
    fn longer_wire_more_parasitics() {
        let short = metal_segment(0.0, 0.0, 50.0, 0.0, 10.0);
        let long = metal_segment(0.0, 0.0, 200.0, 0.0, 10.0);
        let params = LayerParams::default();

        let p_short = extract_segment(&short, &params);
        let p_long = extract_segment(&long, &params);

        assert!(p_long.resistance > p_short.resistance);
        assert!(p_long.capacitance > p_short.capacitance);
    }

    #[test]
    fn net_extraction_aggregates_segments() {
        let mut graph = CircuitGraph::new();
        let node = graph.add_node(0);

        graph.nodes[node].wire_segments = vec![
            metal_segment(0.0, 0.0, 100.0, 0.0, 10.0),
            metal_segment(100.0, 0.0, 200.0, 0.0, 10.0),
        ];

        let params = LayerParams::default();
        let net = extract_net(&graph, node, &params);

        assert_eq!(net.segments.len(), 2);
        assert!(net.total_resistance > 0.0);
        assert!(net.total_capacitance > 0.0);
        assert!(net.elmore_delay > 0.0);
    }

    #[test]
    fn elmore_delay_increases_with_length() {
        let mut graph = CircuitGraph::new();
        let short_node = graph.add_node(0);
        let long_node = graph.add_node(1);

        graph.nodes[short_node].wire_segments = vec![
            metal_segment(0.0, 0.0, 50.0, 0.0, 10.0),
        ];
        graph.nodes[long_node].wire_segments = vec![
            metal_segment(0.0, 0.0, 50.0, 0.0, 10.0),
            metal_segment(50.0, 0.0, 150.0, 0.0, 10.0),
            metal_segment(150.0, 0.0, 300.0, 0.0, 10.0),
        ];

        let params = LayerParams::default();
        let net_short = extract_net(&graph, short_node, &params);
        let net_long = extract_net(&graph, long_node, &params);

        assert!(net_long.elmore_delay > net_short.elmore_delay,
            "Longer net should have more Elmore delay");
    }

    #[test]
    fn apply_updates_graph_capacitance() {
        let mut graph = CircuitGraph::new();
        let node = graph.add_node(0);
        graph.nodes[node].capacitance = 50e-15; // default 50 fF

        graph.nodes[node].wire_segments = vec![
            metal_segment(0.0, 0.0, 500.0, 0.0, 10.0),
        ];

        let params = LayerParams::default();
        apply_to_graph(&mut graph, &params);

        // Should have replaced the default capacitance
        assert!(graph.nodes[node].capacitance > 0.0);
        assert!((graph.nodes[node].capacitance - 50e-15).abs() > 1e-18,
            "Capacitance should have changed from default");
    }

    #[test]
    fn extract_all_skips_nodes_without_wires() {
        let mut graph = CircuitGraph::new();
        let with_wires = graph.add_node(0);
        let _without_wires = graph.add_node(1);

        graph.nodes[with_wires].wire_segments = vec![
            metal_segment(0.0, 0.0, 100.0, 0.0, 10.0),
        ];

        let params = LayerParams::default();
        let results = extract_all(&graph, &params);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].node_idx, with_wires);
    }

    #[test]
    fn coupling_same_layer_parallel() {
        let seg_a = metal_segment(0.0, 0.0, 100.0, 0.0, 10.0);
        let seg_b = metal_segment(0.0, 25.0, 100.0, 25.0, 10.0);
        let params = LayerParams::default();

        let cc = coupling_capacitance(&seg_a, &seg_b, &params);
        assert!(cc > 0.0, "Parallel same-layer wires should have coupling cap");
    }

    #[test]
    fn coupling_different_layers_zero() {
        let seg_a = metal_segment(0.0, 0.0, 100.0, 0.0, 10.0);
        let seg_b = poly_segment(0.0, 25.0, 100.0, 25.0, 10.0);
        let params = LayerParams::default();

        let cc = coupling_capacitance(&seg_a, &seg_b, &params);
        assert!((cc - 0.0).abs() < 1e-30, "Different-layer coupling should be zero");
    }

    #[test]
    fn coupling_perpendicular_zero() {
        let seg_a = metal_segment(0.0, 0.0, 100.0, 0.0, 10.0); // horizontal
        let seg_b = metal_segment(50.0, -50.0, 50.0, 50.0, 10.0); // vertical
        let params = LayerParams::default();

        let cc = coupling_capacitance(&seg_a, &seg_b, &params);
        assert!((cc - 0.0).abs() < 1e-30, "Perpendicular wires should have zero coupling");
    }

    #[test]
    fn coupling_closer_wires_more_capacitance() {
        let seg_a = metal_segment(0.0, 0.0, 100.0, 0.0, 10.0);
        let seg_close = metal_segment(0.0, 25.0, 100.0, 25.0, 10.0); // 15um gap
        let seg_far = metal_segment(0.0, 100.0, 100.0, 100.0, 10.0); // 90um gap
        let params = LayerParams::default();

        let cc_close = coupling_capacitance(&seg_a, &seg_close, &params);
        let cc_far = coupling_capacitance(&seg_a, &seg_far, &params);

        assert!(cc_close > cc_far,
            "Closer wires should have more coupling: close={:.3e}, far={:.3e}",
            cc_close, cc_far);
    }

    #[test]
    fn zero_width_segment_no_crash() {
        let seg = WireSegment {
            x0: 0.0, y0: 0.0, x1: 100.0, y1: 0.0,
            width: 0.0,
            layer: MetalLayer::Metal1,
        };
        let params = LayerParams::default();
        let p = extract_segment(&seg, &params);

        assert!((p.resistance - 0.0).abs() < 1e-30);
    }

    #[test]
    fn zero_length_segment_no_crash() {
        let seg = metal_segment(50.0, 50.0, 50.0, 50.0, 10.0);
        let params = LayerParams::default();
        let p = extract_segment(&seg, &params);

        assert!((p.resistance - 0.0).abs() < 1e-30);
        assert!((p.capacitance - 0.0).abs() < 1e-30);
    }
}
