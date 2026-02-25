//! Static timing analysis on the transistor-level netlist.
//!
//! Extracts critical paths through the circuit by computing propagation
//! delay from each primary input to each primary output, using RC
//! Elmore delay for wire segments and gate delay models for transistors.
//!
//! # Gate Delay Model
//!
//! For the Intel 10um pMOS process, gate delay is dominated by the RC
//! charging of the output node. The delay through a single inverter is:
//!
//! t_pd = R_on * C_load
//!
//! where R_on is the transistor on-resistance and C_load is the total
//! capacitance at the output node (wire + gate loading of fanout).
//!
//! # Critical Path
//!
//! The critical path is the longest delay path from any primary input
//! (clock, data input, control signal) to any primary output (data bus,
//! register write). This determines the maximum operating frequency.

use crate::{
    circuit::{
        graph::CircuitGraph,
        parasitic_extract::{self, LayerParams},
    },
    process::ProcessParams,
};

/// Configuration for timing analysis.
#[derive(Clone, Debug)]
pub struct TimingConfig {
    /// Process parameters (for transistor on-resistance).
    pub process: ProcessParams,
    /// Layer parameters (for wire RC extraction).
    pub layer_params: LayerParams,
    /// Target clock period (s). Used for slack computation.
    pub clock_period: f64,
}

impl Default for TimingConfig {
    fn default() -> Self {
        Self {
            process: ProcessParams::default(),
            layer_params: LayerParams::default(),
            clock_period: 1.35e-6, // 740 kHz nominal
        }
    }
}

/// A single timing arc (delay from one node to another through a gate).
#[derive(Clone, Debug)]
pub struct TimingArc {
    /// Source node index (input).
    pub from_node: usize,
    /// Destination node index (output).
    pub to_node: usize,
    /// Transistor providing this arc.
    pub transistor_idx: usize,
    /// Gate propagation delay (s).
    pub gate_delay: f64,
    /// Wire delay at the output node (s).
    pub wire_delay: f64,
    /// Total delay for this arc: gate_delay + wire_delay (s).
    pub total_delay: f64,
}

/// A complete timing path from input to output.
#[derive(Clone, Debug)]
pub struct TimingPath {
    /// Ordered list of timing arcs from input to output.
    pub arcs: Vec<TimingArc>,
    /// Total path delay (s): sum of all arc delays.
    pub total_delay: f64,
    /// Slack: clock_period - total_delay (s). Negative = timing violation.
    pub slack: f64,
    /// Starting node index.
    pub start_node: usize,
    /// Ending node index.
    pub end_node: usize,
}

/// Result of static timing analysis.
#[derive(Clone, Debug)]
pub struct TimingResult {
    /// All timing arcs in the circuit.
    pub arcs: Vec<TimingArc>,
    /// Critical path (longest delay path).
    pub critical_path: Option<TimingPath>,
    /// All extracted paths, sorted by delay (longest first).
    pub paths: Vec<TimingPath>,
    /// Maximum delay across all paths (s).
    pub max_delay: f64,
    /// Minimum slack across all paths (s). Negative = violation.
    pub min_slack: f64,
    /// Whether the circuit meets timing (min_slack >= 0).
    pub meets_timing: bool,
}

/// Estimate transistor on-resistance.
///
/// R_on = 1 / (mu * Cox * (W/L) * (Vgs - Vth))
///
/// Uses the operating point assumption that Vgs = VDD (worst case
/// for pMOS: fully on).
pub fn transistor_ron(w: f64, l: f64, process: &ProcessParams) -> f64 {
    let cox = crate::process::oxide::cox(process.t_ox);
    let mu_eff = process.mu_0 * 1e-4; // cm^2/V*s -> m^2/V*s
    let vov = (process.vdd - process.vth_enhancement).abs();

    if vov < 0.1 || w < 1e-9 || l < 1e-9 {
        return 1e12; // very high resistance for off or tiny transistors
    }

    1.0 / (mu_eff * cox * (w / l) * vov)
}

/// Build all timing arcs in the circuit.
///
/// Each transistor creates arcs from its gate to its output terminal.
/// The output terminal is determined by the lower-voltage terminal
/// (for pMOS: the more negative terminal is the drain/output).
pub fn build_timing_arcs(graph: &CircuitGraph, config: &TimingConfig) -> Vec<TimingArc> {
    let mut arcs = Vec::new();

    for t in &graph.transistors {
        // Skip if gate is a power rail (depletion loads)
        if graph.nodes[t.gate].is_fixed {
            continue;
        }

        let r_on = transistor_ron(t.w, t.l, &config.process);

        // Both a_node and b_node could be output
        // Create arcs for both directions
        for &out_node in &[t.a_node, t.b_node] {
            if graph.nodes[out_node].is_fixed {
                continue;
            }

            // Total capacitance at output: wire + fanout gates
            let c_wire = graph.nodes[out_node].capacitance;
            let c_fanout = fanout_capacitance(graph, out_node, &config.process);
            let c_load = c_wire + c_fanout;

            // Gate delay = R_on * C_load
            let gate_delay = r_on * c_load;

            // Wire delay from extracted parasitics
            let wire_delay = if !graph.nodes[out_node].wire_segments.is_empty() {
                let net = parasitic_extract::extract_net(graph, out_node, &config.layer_params);
                net.elmore_delay
            } else {
                0.0
            };

            arcs.push(TimingArc {
                from_node: t.gate,
                to_node: out_node,
                transistor_idx: t.id,
                gate_delay,
                wire_delay,
                total_delay: gate_delay + wire_delay,
            });
        }
    }

    arcs
}

/// Compute total gate capacitance loading at a node from fanout transistors.
fn fanout_capacitance(graph: &CircuitGraph, node_idx: usize, process: &ProcessParams) -> f64 {
    let cox = crate::process::oxide::cox(process.t_ox);

    graph
        .transistors
        .iter()
        .filter(|t| t.gate == node_idx)
        .map(|t| cox * t.w * t.l)
        .sum()
}

/// Find primary input nodes (not driven by any transistor output).
pub fn find_primary_inputs(graph: &CircuitGraph) -> Vec<usize> {
    let driven: std::collections::HashSet<usize> =
        graph.transistors.iter().flat_map(|t| [t.a_node, t.b_node]).collect();

    graph
        .nodes
        .iter()
        .filter(|n| {
            !n.is_fixed
                && !driven.contains(&n.id)
                // Also include clock/signal inputs
                || n.name.as_ref().is_some_and(|name| {
                    let lower = name.to_lowercase();
                    lower.contains("phi") || lower.contains("clk") || lower.contains("in")
                })
        })
        .map(|n| n.id)
        .collect()
}

/// Find primary output nodes (nodes that drive off-chip or are observation points).
pub fn find_primary_outputs(graph: &CircuitGraph) -> Vec<usize> {
    graph
        .nodes
        .iter()
        .filter(|n| {
            !n.is_fixed
                && n.name.as_ref().is_some_and(|name| {
                    let lower = name.to_lowercase();
                    lower.contains("out") || lower.contains("data") || lower.contains("bus")
                })
        })
        .map(|n| n.id)
        .collect()
}

/// Extract timing paths using depth-first traversal.
///
/// Starting from each primary input, follow timing arcs to find all
/// paths to primary outputs. Limits depth to avoid infinite loops
/// in feedback circuits.
pub fn extract_paths(
    arcs: &[TimingArc],
    start_nodes: &[usize],
    end_nodes: &[usize],
    clock_period: f64,
    max_depth: usize,
) -> Vec<TimingPath> {
    // Build adjacency map: from_node -> list of arcs
    let mut adj: std::collections::HashMap<usize, Vec<&TimingArc>> = std::collections::HashMap::new();
    for arc in arcs {
        adj.entry(arc.from_node).or_default().push(arc);
    }

    let end_set: std::collections::HashSet<usize> = end_nodes.iter().copied().collect();
    let mut paths = Vec::new();

    for &start in start_nodes {
        let mut stack: Vec<(usize, Vec<TimingArc>, std::collections::HashSet<usize>)> = Vec::new();
        let mut visited = std::collections::HashSet::new();
        visited.insert(start);
        stack.push((start, Vec::new(), visited));

        while let Some((node, path_arcs, visited_set)) = stack.pop() {
            if path_arcs.len() >= max_depth {
                continue;
            }

            if let Some(next_arcs) = adj.get(&node) {
                for arc in next_arcs {
                    if visited_set.contains(&arc.to_node) {
                        continue; // avoid cycles
                    }

                    let mut new_path = path_arcs.clone();
                    new_path.push((*arc).clone());

                    // Check if we reached an end node
                    if end_set.contains(&arc.to_node) {
                        let total_delay: f64 = new_path.iter().map(|a| a.total_delay).sum();
                        paths.push(TimingPath {
                            arcs: new_path.clone(),
                            total_delay,
                            slack: clock_period - total_delay,
                            start_node: start,
                            end_node: arc.to_node,
                        });
                    }

                    // Continue exploring
                    let mut new_visited = visited_set.clone();
                    new_visited.insert(arc.to_node);
                    stack.push((arc.to_node, new_path, new_visited));
                }
            }
        }
    }

    // Sort by delay, longest first
    paths.sort_by(|a, b| {
        b.total_delay
            .partial_cmp(&a.total_delay)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    paths
}

/// Run complete static timing analysis.
pub fn analyze_timing(graph: &CircuitGraph, config: &TimingConfig) -> TimingResult {
    let arcs = build_timing_arcs(graph, config);

    let start_nodes = find_primary_inputs(graph);
    let end_nodes = find_primary_outputs(graph);

    let paths = extract_paths(
        &arcs,
        &start_nodes,
        &end_nodes,
        config.clock_period,
        20, // max depth
    );

    let critical_path = paths.first().cloned();
    let max_delay = paths.first().map(|p| p.total_delay).unwrap_or(0.0);
    let min_slack = paths.iter().map(|p| p.slack).fold(f64::INFINITY, f64::min);

    let meets_timing = min_slack >= 0.0 || paths.is_empty();

    TimingResult {
        arcs,
        critical_path,
        paths,
        max_delay,
        min_slack,
        meets_timing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::graph::{CircuitGraph, MetalLayer, TransistorKind, WireSegment};

    fn make_inverter_chain() -> CircuitGraph {
        let mut graph = CircuitGraph::new();
        let vss = graph.add_node(0);
        let vdd = graph.add_node(1);
        let input = graph.add_node(10);
        let mid = graph.add_node(11);
        let output = graph.add_node(12);

        graph.set_power_rail(vss, 0.0, "VSS");
        graph.set_power_rail(vdd, -15.0, "VDD");
        graph.nodes[input].name = Some("input".to_string());
        graph.nodes[output].name = Some("output".to_string());

        // First inverter: input -> mid
        // Enhancement (switching) transistor
        graph.add_transistor(input, vdd, mid, TransistorKind::Enhancement, 10e-6, 10e-6);
        // Depletion load (gate tied to source = VDD, so gate is fixed)
        graph.add_transistor(vdd, vdd, mid, TransistorKind::Depletion, 10e-6, 20e-6);

        // Second inverter: mid -> output
        graph.add_transistor(mid, vdd, output, TransistorKind::Enhancement, 10e-6, 10e-6);
        graph.add_transistor(vdd, vdd, output, TransistorKind::Depletion, 10e-6, 20e-6);

        // Add some wire capacitance
        graph.nodes[mid].capacitance = 50e-15; // 50 fF
        graph.nodes[output].capacitance = 100e-15; // 100 fF

        graph
    }

    #[test]
    fn transistor_ron_reasonable() {
        let process = ProcessParams::default();
        let r_on = transistor_ron(10e-6, 10e-6, &process);

        // For 10um/10um pMOS at VDD=-15V, Vth=-3V:
        // R_on should be in the kohm range
        assert!(r_on > 100.0 && r_on < 1e6, "R_on = {:.1} ohm, expected 100 to 1M", r_on);
    }

    #[test]
    fn wider_transistor_lower_ron() {
        let process = ProcessParams::default();
        let r_narrow = transistor_ron(10e-6, 10e-6, &process);
        let r_wide = transistor_ron(20e-6, 10e-6, &process);
        assert!(r_wide < r_narrow, "Wider transistor should have lower R_on");
    }

    #[test]
    fn build_arcs_creates_arcs() {
        let graph = make_inverter_chain();
        let config = TimingConfig::default();
        let arcs = build_timing_arcs(&graph, &config);

        // Should have arcs from input->mid and mid->output
        // (depletion loads have fixed gates, so no arcs from them)
        assert!(!arcs.is_empty(), "Should have timing arcs");

        // All arcs should have positive delay
        for arc in &arcs {
            assert!(arc.total_delay > 0.0, "Arc delay should be positive");
            assert!(arc.gate_delay > 0.0, "Gate delay should be positive");
        }
    }

    #[test]
    fn arc_delay_reasonable_order_of_magnitude() {
        let graph = make_inverter_chain();
        let config = TimingConfig::default();
        let arcs = build_timing_arcs(&graph, &config);

        for arc in &arcs {
            // Delay should be in ps to us range for 10um process
            assert!(
                arc.total_delay > 1e-15 && arc.total_delay < 1e-3,
                "Arc delay = {:.3e} s, expected ps to us range",
                arc.total_delay
            );
        }
    }

    #[test]
    fn find_inputs_and_outputs() {
        let graph = make_inverter_chain();
        let inputs = find_primary_inputs(&graph);
        let outputs = find_primary_outputs(&graph);

        assert!(!inputs.is_empty(), "Should find primary inputs");
        assert!(!outputs.is_empty(), "Should find primary outputs");
    }

    #[test]
    fn extract_paths_finds_input_to_output() {
        let graph = make_inverter_chain();
        let config = TimingConfig::default();
        let arcs = build_timing_arcs(&graph, &config);
        let inputs = find_primary_inputs(&graph);
        let outputs = find_primary_outputs(&graph);

        let paths = extract_paths(&arcs, &inputs, &outputs, config.clock_period, 20);

        // Should find at least one path from input to output
        assert!(!paths.is_empty(), "Should find paths from input to output");
    }

    #[test]
    fn critical_path_has_positive_delay() {
        let graph = make_inverter_chain();
        let config = TimingConfig::default();
        let result = analyze_timing(&graph, &config);

        if let Some(ref cp) = result.critical_path {
            assert!(cp.total_delay > 0.0, "Critical path should have positive delay");
            assert!(!cp.arcs.is_empty(), "Critical path should have arcs");
        }
    }

    #[test]
    fn timing_within_clock_period() {
        let graph = make_inverter_chain();
        let config = TimingConfig::default();
        let result = analyze_timing(&graph, &config);

        // A 2-inverter chain should easily fit within 1.35us clock period
        if !result.paths.is_empty() {
            assert!(
                result.meets_timing,
                "2-inverter chain should meet timing, delay = {:.3e} s",
                result.max_delay
            );
        }
    }

    #[test]
    fn fanout_capacitance_scales_with_count() {
        let mut graph = CircuitGraph::new();
        let gate = graph.add_node(0);
        let n1 = graph.add_node(1);

        // Add 3 transistors driven by gate
        for _ in 0..3 {
            graph.add_transistor(gate, n1, n1, TransistorKind::Enhancement, 10e-6, 10e-6);
        }

        let process = ProcessParams::default();
        let c_fan = fanout_capacitance(&graph, gate, &process);

        // Should be 3x single gate cap
        let cox = crate::process::oxide::cox(process.t_ox);
        let c_single = cox * 10e-6 * 10e-6;
        assert!(
            (c_fan - 3.0 * c_single).abs() / c_fan < 0.01,
            "Fanout cap should be 3x single: got {:.3e}, expected {:.3e}",
            c_fan,
            3.0 * c_single
        );
    }

    #[test]
    fn empty_circuit_no_crash() {
        let graph = CircuitGraph::new();
        let config = TimingConfig::default();
        let result = analyze_timing(&graph, &config);

        assert!(result.arcs.is_empty());
        assert!(result.paths.is_empty());
        assert!(result.critical_path.is_none());
        assert!(result.meets_timing);
    }

    #[test]
    fn wire_delay_adds_to_gate_delay() {
        let mut graph = CircuitGraph::new();
        let vss = graph.add_node(0);
        let vdd = graph.add_node(1);
        let input = graph.add_node(10);
        let output = graph.add_node(11);

        graph.set_power_rail(vss, 0.0, "VSS");
        graph.set_power_rail(vdd, -15.0, "VDD");
        graph.nodes[input].name = Some("input".to_string());
        graph.nodes[output].name = Some("output".to_string());

        // Single inverter with long output wire
        graph.add_transistor(input, vdd, output, TransistorKind::Enhancement, 10e-6, 10e-6);

        graph.nodes[output].wire_segments = vec![WireSegment {
            x0: 0.0,
            y0: 0.0,
            x1: 500.0,
            y1: 0.0,
            width: 10.0,
            layer: MetalLayer::Poly,
        }];

        let config = TimingConfig::default();
        let arcs = build_timing_arcs(&graph, &config);

        let output_arc = arcs.iter().find(|a| a.to_node == output);
        if let Some(arc) = output_arc {
            assert!(arc.wire_delay > 0.0, "Wire delay should be positive for poly wire");
            assert!(
                arc.total_delay > arc.gate_delay,
                "Total should exceed gate delay when wire is present"
            );
        }
    }

    #[test]
    fn path_slack_computation() {
        let graph = make_inverter_chain();
        let config = TimingConfig::default();
        let result = analyze_timing(&graph, &config);

        for path in &result.paths {
            let expected_slack = config.clock_period - path.total_delay;
            assert!(
                (path.slack - expected_slack).abs() < 1e-18,
                "Slack should be clock_period - delay"
            );
        }
    }
}
