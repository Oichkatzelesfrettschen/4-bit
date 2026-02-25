//! Clock distribution analysis for the MCS-4 chip family.
//!
//! Traces phi1/phi2 clock signals from pad locations through the die,
//! computing per-endpoint Elmore delay and clock skew. The 4004 uses a
//! two-phase non-overlapping clock driven externally (by the 4201 clock
//! generator or discrete components).
//!
//! # Clock Timing
//!
//! The 4004 operates at 740 kHz nominal (1.35 us period), with a maximum
//! of 2.0 us period. Each machine cycle is 8 clock phases: A1-A3, M1-M2,
//! X1-X3. Setup and hold margins are critical for reliable operation.
//!
//! # Analysis Flow
//!
//! 1. Identify clock source nodes (phi1, phi2 pads)
//! 2. Trace clock nets through wire segments
//! 3. Compute Elmore delay at each endpoint
//! 4. Report maximum skew (max delay - min delay)
//! 5. Compare against timing specifications

use crate::{
    circuit::{
        graph::CircuitGraph,
        parasitic_extract::{self, LayerParams, NetParasitics},
    },
    process::interconnect,
};

/// Clock timing specification for the Intel 4004.
///
/// Values from the Intel MCS-4 datasheet.
#[derive(Clone, Debug)]
pub struct ClockSpec {
    /// Minimum clock period (s).
    pub t_period_min: f64,
    /// Maximum clock period (s).
    pub t_period_max: f64,
    /// Minimum non-overlap time between phi1 and phi2 (s).
    pub t_non_overlap_min: f64,
    /// Maximum allowed clock skew across the die (s).
    pub t_skew_max: f64,
    /// Setup time for sequential elements (s).
    pub t_setup: f64,
    /// Hold time for sequential elements (s).
    pub t_hold: f64,
}

impl Default for ClockSpec {
    fn default() -> Self {
        Self {
            t_period_min: 1.35e-6,     // 1.35 us (740 kHz)
            t_period_max: 2.0e-6,      // 2.0 us (500 kHz)
            t_non_overlap_min: 100e-9, // 100 ns minimum non-overlap
            t_skew_max: 50e-9,         // 50 ns maximum acceptable skew
            t_setup: 50e-9,            // 50 ns setup time
            t_hold: 20e-9,             // 20 ns hold time
        }
    }
}

/// A clock endpoint (a node that receives a clock signal).
#[derive(Clone, Debug)]
pub struct ClockEndpoint {
    /// Node index in the circuit graph.
    pub node_idx: usize,
    /// Elmore delay from clock source to this endpoint (s).
    pub delay: f64,
    /// Resistance from source to endpoint (ohms).
    pub resistance: f64,
    /// Capacitance at this endpoint (F).
    pub capacitance: f64,
    /// X coordinate on die (um). None if not spatially mapped.
    pub x_um: Option<f64>,
    /// Y coordinate on die (um). None if not spatially mapped.
    pub y_um: Option<f64>,
}

/// Result of clock distribution analysis for one clock phase.
#[derive(Clone, Debug)]
pub struct ClockPhaseResult {
    /// Clock phase name (e.g., "phi1", "phi2").
    pub name: String,
    /// Source node index.
    pub source_node: usize,
    /// All endpoints receiving this clock.
    pub endpoints: Vec<ClockEndpoint>,
    /// Minimum delay among all endpoints (s).
    pub min_delay: f64,
    /// Maximum delay among all endpoints (s).
    pub max_delay: f64,
    /// Clock skew: max_delay - min_delay (s).
    pub skew: f64,
    /// Total capacitive load on the clock net (F).
    pub total_load: f64,
}

/// Complete clock distribution analysis result.
#[derive(Clone, Debug)]
pub struct ClockTreeResult {
    /// Per-phase results.
    pub phases: Vec<ClockPhaseResult>,
    /// Timing specification used.
    pub spec: ClockSpec,
    /// Whether all phases meet the skew specification.
    pub meets_spec: bool,
    /// Maximum skew across all phases (s).
    pub worst_skew: f64,
}

/// Identify clock source nodes by name pattern.
///
/// Searches for nodes with names containing "phi1", "phi2", "clk",
/// or "clock" (case-insensitive).
pub fn find_clock_nodes(graph: &CircuitGraph) -> Vec<(usize, String)> {
    let clock_patterns = ["phi1", "phi2", "clk", "clock", "phi_1", "phi_2"];

    graph
        .nodes
        .iter()
        .filter_map(|node| {
            if let Some(ref name) = node.name {
                let lower = name.to_lowercase();
                if clock_patterns.iter().any(|p| lower.contains(p)) {
                    Some((node.id, name.clone()))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

/// Find all gate-connected nodes that a clock source drives.
///
/// Starting from the source node, find all transistors where this node
/// is the gate. Each such transistor gate connection is a clock endpoint.
pub fn find_clock_endpoints(graph: &CircuitGraph, source_node: usize) -> Vec<usize> {
    graph
        .transistors
        .iter()
        .filter(|t| t.gate == source_node)
        .map(|t| t.gate)
        .collect::<Vec<_>>()
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

/// Compute clock distribution for a single clock phase.
///
/// Uses Elmore delay from the clock source node to each endpoint.
/// If the source node has wire segments, uses layout-extracted RC.
/// Otherwise, estimates from transistor gate loading.
pub fn analyze_clock_phase(
    graph: &CircuitGraph,
    source_node: usize,
    phase_name: &str,
    layer_params: &LayerParams,
) -> ClockPhaseResult {
    // Extract parasitics for the source net
    let net_parasitics = parasitic_extract::extract_net(graph, source_node, layer_params);

    // Find transistors driven by this clock
    let driven_transistors: Vec<usize> = graph
        .transistors
        .iter()
        .filter(|t| t.gate == source_node)
        .map(|t| t.id)
        .collect();

    // Build endpoints from wire segment chain
    let endpoints = build_endpoints_from_net(graph, source_node, &net_parasitics, &driven_transistors, layer_params);

    let (min_delay, max_delay) = if endpoints.is_empty() {
        (0.0, 0.0)
    } else {
        let min = endpoints.iter().map(|e| e.delay).fold(f64::INFINITY, f64::min);
        let max = endpoints.iter().map(|e| e.delay).fold(f64::NEG_INFINITY, f64::max);
        (min, max)
    };

    let skew = max_delay - min_delay;
    let total_load: f64 = endpoints.iter().map(|e| e.capacitance).sum();

    ClockPhaseResult {
        name: phase_name.to_string(),
        source_node,
        endpoints,
        min_delay,
        max_delay,
        skew,
        total_load,
    }
}

/// Build clock endpoints from net parasitics and transistor loading.
fn build_endpoints_from_net(
    graph: &CircuitGraph,
    source_node: usize,
    net_parasitics: &NetParasitics,
    driven_transistors: &[usize],
    layer_params: &LayerParams,
) -> Vec<ClockEndpoint> {
    if net_parasitics.segments.is_empty() {
        // No wire data: create endpoints from transistor gate loading
        return driven_transistors
            .iter()
            .map(|&t_idx| {
                let t = &graph.transistors[t_idx];
                // Gate capacitance estimate: C_gate = Cox * W * L
                let cox_per_area = crate::process::oxide::cox(crate::process::ProcessParams::default().t_ox);
                let c_gate = cox_per_area * t.w * t.l;
                ClockEndpoint {
                    node_idx: source_node,
                    delay: 0.0, // no wire delay without layout
                    resistance: 0.0,
                    capacitance: c_gate,
                    x_um: t.x_center,
                    y_um: t.y_center,
                }
            })
            .collect();
    }

    // With wire data: compute cumulative Elmore delay along the chain
    let segments = &net_parasitics.segments;
    let n_segments = segments.len();

    // Build cumulative R and C for Elmore delay at each point
    let mut cumulative_r = Vec::with_capacity(n_segments + 1);
    cumulative_r.push(0.0);
    let mut r_sum = 0.0;
    for seg in segments {
        r_sum += seg.resistance;
        cumulative_r.push(r_sum);
    }

    // For each driven transistor, find the closest wire segment endpoint
    // and compute the Elmore delay to that point.
    let _ = layer_params; // used via extract_net above
    let mut endpoints = Vec::new();
    for &t_idx in driven_transistors {
        let t = &graph.transistors[t_idx];
        let cox_per_area = crate::process::oxide::cox(crate::process::ProcessParams::default().t_ox);
        let c_gate = cox_per_area * t.w * t.l;

        // Estimate Elmore delay proportional to position along the chain
        // Use the total net Elmore delay scaled by fraction of chain
        let (delay, r_to_point) = if let (Some(tx), Some(ty)) = (t.x_center, t.y_center) {
            estimate_delay_to_point(graph, source_node, tx, ty, &cumulative_r, net_parasitics)
        } else {
            // No spatial data: use average
            let avg_delay = net_parasitics.elmore_delay / 2.0;
            let avg_r = net_parasitics.total_resistance / 2.0;
            (avg_delay, avg_r)
        };

        endpoints.push(ClockEndpoint {
            node_idx: source_node,
            delay,
            resistance: r_to_point,
            capacitance: c_gate,
            x_um: t.x_center,
            y_um: t.y_center,
        });
    }

    endpoints
}

/// Estimate Elmore delay from the source node to a specific die coordinate.
///
/// Uses wire segment topology to find the closest segment and interpolate.
fn estimate_delay_to_point(
    graph: &CircuitGraph,
    source_node: usize,
    target_x: f64,
    target_y: f64,
    cumulative_r: &[f64],
    net_parasitics: &NetParasitics,
) -> (f64, f64) {
    let node = &graph.nodes[source_node];
    if node.wire_segments.is_empty() {
        return (0.0, 0.0);
    }

    // Find the wire segment whose midpoint is closest to the target
    let mut best_seg_idx = 0;
    let mut best_dist = f64::INFINITY;

    for (i, seg) in node.wire_segments.iter().enumerate() {
        let mid_x = (seg.x0 + seg.x1) / 2.0;
        let mid_y = (seg.y0 + seg.y1) / 2.0;
        let dist = ((mid_x - target_x).powi(2) + (mid_y - target_y).powi(2)).sqrt();
        if dist < best_dist {
            best_dist = dist;
            best_seg_idx = i;
        }
    }

    // R to this segment endpoint
    let r_to_point = if best_seg_idx + 1 < cumulative_r.len() {
        cumulative_r[best_seg_idx + 1]
    } else {
        *cumulative_r.last().unwrap_or(&0.0)
    };

    // Downstream capacitance from this point
    let downstream_c: f64 = net_parasitics
        .segments
        .iter()
        .skip(best_seg_idx)
        .map(|s| s.capacitance)
        .sum();

    let delay = interconnect::elmore_delay(r_to_point, downstream_c, false);
    (delay, r_to_point)
}

/// Run complete clock distribution analysis.
///
/// Analyzes all identified clock phases, computes skew, and checks
/// against the timing specification.
pub fn analyze_clock_tree(
    graph: &CircuitGraph,
    clock_nodes: &[(usize, String)],
    spec: &ClockSpec,
    layer_params: &LayerParams,
) -> ClockTreeResult {
    let phases: Vec<ClockPhaseResult> = clock_nodes
        .iter()
        .map(|(node_idx, name)| analyze_clock_phase(graph, *node_idx, name, layer_params))
        .collect();

    let worst_skew = phases.iter().map(|p| p.skew).fold(0.0_f64, f64::max);

    let meets_spec = worst_skew <= spec.t_skew_max;

    ClockTreeResult {
        phases,
        spec: spec.clone(),
        meets_spec,
        worst_skew,
    }
}

/// Compute maximum available timing margin.
///
/// Margin = t_period/2 - t_non_overlap - t_skew - t_setup
///
/// This is the time available for combinational logic between
/// clock edges.
pub fn timing_margin(spec: &ClockSpec, worst_skew: f64) -> f64 {
    spec.t_period_min / 2.0 - spec.t_non_overlap_min - worst_skew - spec.t_setup
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::graph::{CircuitGraph, MetalLayer, TransistorKind, WireSegment};

    fn make_clock_graph() -> CircuitGraph {
        let mut graph = CircuitGraph::new();
        let vss = graph.add_node(0);
        let vdd = graph.add_node(1);
        let phi1 = graph.add_node(2);
        let phi2 = graph.add_node(3);
        let n4 = graph.add_node(4);
        let n5 = graph.add_node(5);

        graph.set_power_rail(vss, 0.0, "VSS");
        graph.set_power_rail(vdd, -15.0, "VDD");
        graph.nodes[phi1].name = Some("phi1".to_string());
        graph.nodes[phi2].name = Some("phi2".to_string());

        // Add wire segments to phi1
        graph.nodes[phi1].wire_segments = vec![
            WireSegment {
                x0: 0.0,
                y0: 100.0,
                x1: 500.0,
                y1: 100.0,
                width: 10.0,
                layer: MetalLayer::Metal1,
            },
            WireSegment {
                x0: 500.0,
                y0: 100.0,
                x1: 1000.0,
                y1: 100.0,
                width: 10.0,
                layer: MetalLayer::Metal1,
            },
        ];

        // Transistors driven by phi1
        let t0 = graph.add_transistor(phi1, vss, n4, TransistorKind::Enhancement, 10e-6, 10e-6);
        graph.transistors[t0].x_center = Some(200.0);
        graph.transistors[t0].y_center = Some(200.0);

        let t1 = graph.add_transistor(phi1, n4, n5, TransistorKind::Enhancement, 10e-6, 10e-6);
        graph.transistors[t1].x_center = Some(800.0);
        graph.transistors[t1].y_center = Some(200.0);

        // Transistor driven by phi2
        let t2 = graph.add_transistor(phi2, vdd, n5, TransistorKind::Enhancement, 10e-6, 10e-6);
        graph.transistors[t2].x_center = Some(500.0);
        graph.transistors[t2].y_center = Some(500.0);

        graph
    }

    #[test]
    fn find_clock_nodes_identifies_phi() {
        let graph = make_clock_graph();
        let clocks = find_clock_nodes(&graph);
        assert_eq!(clocks.len(), 2);
        let names: Vec<&str> = clocks.iter().map(|(_, n)| n.as_str()).collect();
        assert!(names.contains(&"phi1"));
        assert!(names.contains(&"phi2"));
    }

    #[test]
    fn find_clock_nodes_empty_on_no_clocks() {
        let graph = CircuitGraph::new();
        let clocks = find_clock_nodes(&graph);
        assert!(clocks.is_empty());
    }

    #[test]
    fn analyze_clock_phase_produces_endpoints() {
        let graph = make_clock_graph();
        let layer_params = LayerParams::default();

        // phi1 is node index 2
        let result = analyze_clock_phase(&graph, 2, "phi1", &layer_params);

        assert_eq!(result.name, "phi1");
        assert_eq!(result.source_node, 2);
        assert_eq!(result.endpoints.len(), 2, "phi1 drives 2 transistors");
        assert!(result.total_load > 0.0, "Should have gate capacitance load");
    }

    #[test]
    fn clock_phase_skew_non_negative() {
        let graph = make_clock_graph();
        let layer_params = LayerParams::default();
        let result = analyze_clock_phase(&graph, 2, "phi1", &layer_params);

        assert!(result.skew >= 0.0, "Skew should be non-negative");
        assert!(result.min_delay <= result.max_delay);
    }

    #[test]
    fn analyze_clock_tree_complete() {
        let graph = make_clock_graph();
        let clock_nodes = find_clock_nodes(&graph);
        let spec = ClockSpec::default();
        let layer_params = LayerParams::default();

        let result = analyze_clock_tree(&graph, &clock_nodes, &spec, &layer_params);

        assert_eq!(result.phases.len(), 2);
        assert!(result.worst_skew >= 0.0);
    }

    #[test]
    fn default_spec_reasonable() {
        let spec = ClockSpec::default();
        assert!(spec.t_period_min > 0.0);
        assert!(spec.t_period_max > spec.t_period_min);
        assert!(spec.t_non_overlap_min > 0.0);
        assert!(spec.t_setup > 0.0);
        assert!(spec.t_hold > 0.0);
        assert!(spec.t_skew_max > 0.0);
    }

    #[test]
    fn timing_margin_positive_at_nominal() {
        let spec = ClockSpec::default();
        let margin = timing_margin(&spec, 10e-9); // 10ns skew
                                                  // margin = 675ns - 100ns - 10ns - 50ns = 515ns
        assert!(
            margin > 0.0,
            "Timing margin should be positive at nominal: {:.1e} s",
            margin
        );
    }

    #[test]
    fn timing_margin_decreases_with_skew() {
        let spec = ClockSpec::default();
        let m1 = timing_margin(&spec, 10e-9);
        let m2 = timing_margin(&spec, 40e-9);
        assert!(m2 < m1, "More skew should reduce margin");
    }

    #[test]
    fn no_wire_segments_gives_zero_delay() {
        let mut graph = CircuitGraph::new();
        let phi = graph.add_node(0);
        let n1 = graph.add_node(1);
        graph.nodes[phi].name = Some("phi1".to_string());
        // No wire segments on phi

        let t = graph.add_transistor(phi, n1, n1, TransistorKind::Enhancement, 10e-6, 10e-6);
        graph.transistors[t].x_center = Some(100.0);
        graph.transistors[t].y_center = Some(100.0);

        let layer_params = LayerParams::default();
        let result = analyze_clock_phase(&graph, phi, "phi1", &layer_params);

        assert_eq!(result.endpoints.len(), 1);
        assert!(
            result.endpoints[0].delay.abs() < 1e-30,
            "No wire segments should give zero delay"
        );
    }

    #[test]
    fn clock_load_scales_with_transistor_count() {
        let mut graph = CircuitGraph::new();
        let phi = graph.add_node(0);
        let n1 = graph.add_node(1);
        graph.nodes[phi].name = Some("phi1".to_string());

        // Add several transistors
        for i in 0..5 {
            let t = graph.add_transistor(phi, n1, n1, TransistorKind::Enhancement, 10e-6, 10e-6);
            graph.transistors[t].x_center = Some(i as f64 * 100.0);
            graph.transistors[t].y_center = Some(100.0);
        }

        let layer_params = LayerParams::default();
        let result = analyze_clock_phase(&graph, phi, "phi1", &layer_params);

        assert_eq!(result.endpoints.len(), 5);
        // Total load should be ~5x single gate cap
        let single_cap = result.endpoints[0].capacitance;
        assert!(
            (result.total_load - 5.0 * single_cap).abs() / result.total_load < 0.01,
            "Total load should be 5x single gate cap"
        );
    }

    #[test]
    fn meets_spec_true_for_small_skew() {
        let graph = make_clock_graph();
        let clock_nodes = find_clock_nodes(&graph);
        let spec = ClockSpec::default();
        let layer_params = LayerParams::default();

        let result = analyze_clock_tree(&graph, &clock_nodes, &spec, &layer_params);

        // Small test circuit should have minimal skew
        assert!(
            result.meets_spec,
            "Small circuit should meet spec, skew = {:.3e} s",
            result.worst_skew
        );
    }
}
