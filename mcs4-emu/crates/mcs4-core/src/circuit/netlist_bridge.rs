//! Bridge from extracted NetlistV1 JSON format to CircuitGraph.
//!
//! Parses transistor netlists produced by the photomicrograph extraction
//! pipeline and builds a typed circuit graph suitable for the SPICE-class
//! solver. Handles node deduplication, power rail assignment, and geometry
//! estimation from bounding boxes.

use crate::layout_netlist::NetlistV1;
use super::bbox_to_geometry::{self, BBox};
use super::graph::{CircuitGraph, TransistorKind};
use super::power_rail_id;

/// Configuration for the netlist-to-circuit conversion.
#[derive(Clone, Debug)]
pub struct BridgeConfig {
    /// VDD voltage (V).
    pub vdd: f64,
    /// VSS voltage (V).
    pub vss: f64,
    /// Pixel-to-micrometer scale factor.
    pub scale_um_per_px: f64,
    /// Default node capacitance for internal nodes (F).
    pub default_node_cap: f64,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            vdd: -15.0,
            vss: 0.0,
            scale_um_per_px: bbox_to_geometry::DEFAULT_SCALE_UM_PER_PX,
            default_node_cap: 50e-15, // 50 fF default
        }
    }
}

/// Convert a NetlistV1 to a CircuitGraph.
///
/// Performs these steps:
/// 1. Identify power rail nodes from anchor signals
/// 2. Create graph nodes for all referenced netlist nodes
/// 3. Set power rails as fixed-voltage nodes
/// 4. Create transistor instances with geometry from bounding boxes
/// 5. Assign default capacitance to internal nodes
pub fn netlist_v1_to_circuit(netlist: &NetlistV1, config: &BridgeConfig) -> CircuitGraph {
    let mut graph = CircuitGraph::new();

    // Identify power rails
    let rails = power_rail_id::identify_power_rails(netlist);

    // First pass: create all nodes referenced by transistors
    for trans in &netlist.devices.transistors {
        graph.add_node(trans.gate_node);
        graph.add_node(trans.a_node);
        graph.add_node(trans.b_node);
    }

    // Set power rails
    if let Some(vdd_id) = rails.vdd_node {
        let idx = graph.add_node(vdd_id);
        graph.set_power_rail(idx, config.vdd, "VDD");
        graph.vdd_idx = Some(idx);
    }
    if let Some(vss_id) = rails.vss_node {
        let idx = graph.add_node(vss_id);
        graph.set_power_rail(idx, config.vss, "VSS");
        graph.vss_idx = Some(idx);
    }

    // Apply signal names to nodes
    for signal in &netlist.signals {
        if let Some(node_id) = signal.layout_node {
            if let Some(&idx) = graph.node_map.get(&node_id) {
                if graph.nodes[idx].name.is_none() {
                    graph.nodes[idx].name = Some(signal.name.clone());
                }
            }
        }
    }

    // Set default capacitance for non-fixed nodes
    for node in &mut graph.nodes {
        if !node.is_fixed {
            node.capacitance = config.default_node_cap;
        }
    }

    // Create transistors with geometry estimation
    for trans in &netlist.devices.transistors {
        let gate_idx = graph.node_map[&trans.gate_node];
        let a_idx = graph.node_map[&trans.a_node];
        let b_idx = graph.node_map[&trans.b_node];

        // Determine kind: if gate connects to one of its own terminals, it is
        // a depletion-mode load (gate tied to source).
        let kind = if trans.gate_node == trans.a_node || trans.gate_node == trans.b_node {
            TransistorKind::Depletion
        } else {
            TransistorKind::Enhancement
        };

        // Estimate W/L from bounding box if available
        // NetlistV1 transistors don't have a direct bbox field in the struct,
        // but the JSON does. We use a default 10um/10um if not available.
        let (w, l) = (10e-6, 10e-6); // Default geometry

        graph.add_transistor(gate_idx, a_idx, b_idx, kind, w, l);
    }

    // Initialize non-fixed nodes to midpoint voltage
    let vmid = (config.vdd + config.vss) / 2.0;
    graph.init_voltages(vmid);

    graph
}

/// Convert with bounding box data extracted separately.
///
/// # Arguments
/// * `netlist` - parsed NetlistV1
/// * `bboxes` - bounding boxes for each transistor (same order as netlist)
/// * `config` - bridge configuration
pub fn netlist_v1_to_circuit_with_bboxes(
    netlist: &NetlistV1,
    bboxes: &[BBox],
    config: &BridgeConfig,
) -> CircuitGraph {
    let mut graph = netlist_v1_to_circuit(netlist, config);

    // Update transistor geometry from bounding boxes
    for (i, bbox) in bboxes.iter().enumerate() {
        if i < graph.transistors.len() {
            let (w, l) = bbox_to_geometry::bbox_to_wl(bbox, config.scale_um_per_px);
            graph.transistors[i].w = w;
            graph.transistors[i].l = l;
        }
    }

    graph
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_netlist::{Devices, Evidence, Signal, Transistor};
    use std::path::Path;

    fn make_inverter_netlist() -> NetlistV1 {
        // Minimal inverter: depletion load + enhancement driver
        NetlistV1 {
            chip: "test_inv".to_string(),
            devices: Devices {
                transistors: vec![
                    // Depletion load: gate tied to source (node 3)
                    Transistor {
                        id: Some(0),
                        kind: Some("pmos_candidate".to_string()),
                        gate_node: 3, // gate = a_node (depletion)
                        a_node: 3,    // output node
                        b_node: 1,    // VDD
                    },
                    // Enhancement driver
                    Transistor {
                        id: Some(1),
                        kind: Some("pmos_candidate".to_string()),
                        gate_node: 4, // input
                        a_node: 3,    // output
                        b_node: 2,    // VSS
                    },
                ],
            },
            signals: vec![
                Signal {
                    name: "VDD".to_string(),
                    layout_node: Some(1),
                    evidence: Evidence { anchor: true },
                },
                Signal {
                    name: "VSS".to_string(),
                    layout_node: Some(2),
                    evidence: Evidence { anchor: true },
                },
                Signal {
                    name: "OUT".to_string(),
                    layout_node: Some(3),
                    evidence: Evidence { anchor: false },
                },
                Signal {
                    name: "IN".to_string(),
                    layout_node: Some(4),
                    evidence: Evidence { anchor: false },
                },
            ],
        }
    }

    #[test]
    fn converts_inverter_netlist() {
        let net = make_inverter_netlist();
        let config = BridgeConfig::default();
        let graph = netlist_v1_to_circuit(&net, &config);

        assert_eq!(graph.transistor_count(), 2);
        // Nodes: VDD(1), VSS(2), OUT(3), IN(4)
        assert_eq!(graph.nodes.len(), 4);
        assert_eq!(graph.free_node_count(), 2); // OUT and IN are free
    }

    #[test]
    fn identifies_depletion_load() {
        let net = make_inverter_netlist();
        let config = BridgeConfig::default();
        let graph = netlist_v1_to_circuit(&net, &config);

        // First transistor has gate = a_node, should be depletion
        assert_eq!(graph.transistors[0].kind, TransistorKind::Depletion);
        assert_eq!(graph.transistors[1].kind, TransistorKind::Enhancement);
    }

    #[test]
    fn power_rails_assigned() {
        let net = make_inverter_netlist();
        let config = BridgeConfig::default();
        let graph = netlist_v1_to_circuit(&net, &config);

        assert!(graph.vdd_idx.is_some());
        assert!(graph.vss_idx.is_some());

        let vdd_node = &graph.nodes[graph.vdd_idx.expect("vdd")];
        assert!(vdd_node.is_fixed);
        assert!((vdd_node.voltage - (-15.0)).abs() < 1e-10);

        let vss_node = &graph.nodes[graph.vss_idx.expect("vss")];
        assert!(vss_node.is_fixed);
        assert!((vss_node.voltage - 0.0).abs() < 1e-10);
    }

    #[test]
    fn non_fixed_nodes_at_midpoint() {
        let net = make_inverter_netlist();
        let config = BridgeConfig::default();
        let graph = netlist_v1_to_circuit(&net, &config);

        for node in &graph.nodes {
            if !node.is_fixed {
                assert!((node.voltage - (-7.5)).abs() < 1e-10,
                    "Free node should be at midpoint (-7.5V), got {}", node.voltage);
            }
        }
    }

    #[test]
    fn parses_4003_netlist_v1_to_graph() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("repo root");
        let path = repo_root.join("docs/evidence/netlists_v1/4003_netlist_v1.json");

        let net = crate::layout_netlist::load_netlist_v1(&path)
            .expect("load 4003 netlist");
        assert_eq!(net.chip, "4003");

        let config = BridgeConfig::default();
        let graph = netlist_v1_to_circuit(&net, &config);

        // 4003 has 37 transistors
        assert_eq!(graph.transistor_count(), 37,
            "4003 should have 37 transistors, got {}", graph.transistor_count());

        // Should have some nodes
        assert!(graph.nodes.len() > 10,
            "Should have many nodes, got {}", graph.nodes.len());

        // All transistor node references should be valid
        for trans in &graph.transistors {
            assert!(trans.gate < graph.nodes.len());
            assert!(trans.a_node < graph.nodes.len());
            assert!(trans.b_node < graph.nodes.len());
        }
    }

    #[test]
    fn bbox_geometry_applied() {
        let net = make_inverter_netlist();
        let config = BridgeConfig::default();
        let bboxes = vec![
            BBox::new(0, 0, 30, 80),  // depletion load
            BBox::new(0, 0, 50, 100), // enhancement driver
        ];

        let graph = netlist_v1_to_circuit_with_bboxes(&net, &bboxes, &config);

        // First transistor: bbox 30x80 -> w=30um, l=80um (shorter=width)
        assert!((graph.transistors[0].w - 30e-6).abs() < 1e-9);
        assert!((graph.transistors[0].l - 80e-6).abs() < 1e-9);
    }
}
