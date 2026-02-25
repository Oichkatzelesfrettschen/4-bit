//! Power rail identification from netlist anchor signals.
//!
//! Extracted netlists tag certain signals as "anchors" (VDD, VSS, CLK, etc.).
//! This module identifies which nodes correspond to power rails and assigns
//! them fixed voltages for simulation.

use crate::layout_netlist::NetlistV1;

/// Result of power rail identification.
#[derive(Clone, Debug)]
pub struct PowerRails {
    /// Netlist node ID for VDD (negative supply, e.g., -15V).
    pub vdd_node: Option<i32>,
    /// Netlist node ID for VSS (ground, 0V).
    pub vss_node: Option<i32>,
    /// Clock signal node IDs (if identified).
    pub clock_nodes: Vec<i32>,
}

/// Identify power rail nodes from anchor signals in a NetlistV1.
///
/// Searches for signals named "VDD", "VSS", "GND", "VCC" etc.
/// and returns their layout node IDs.
pub fn identify_power_rails(netlist: &NetlistV1) -> PowerRails {
    let mut result = PowerRails {
        vdd_node: None,
        vss_node: None,
        clock_nodes: Vec::new(),
    };

    for signal in &netlist.signals {
        if !signal.evidence.anchor {
            continue;
        }
        let name_upper = signal.name.to_uppercase();
        match name_upper.as_str() {
            "VDD" | "VCC" | "V+" => {
                result.vdd_node = signal.layout_node;
            }
            "VSS" | "GND" | "V-" => {
                result.vss_node = signal.layout_node;
            }
            _ if name_upper.contains("CLK") || name_upper.contains("CLOCK") => {
                if let Some(node) = signal.layout_node {
                    result.clock_nodes.push(node);
                }
            }
            _ => {}
        }
    }

    result
}

/// Check if a node is referenced by any transistor in the netlist.
pub fn node_has_connections(netlist: &NetlistV1, node_id: i32) -> bool {
    netlist.devices.transistors.iter().any(|t| {
        t.gate_node == node_id || t.a_node == node_id || t.b_node == node_id
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_netlist::{Devices, Evidence, Signal, Transistor};

    fn make_test_netlist() -> NetlistV1 {
        NetlistV1 {
            chip: "test".to_string(),
            devices: Devices {
                transistors: vec![
                    Transistor {
                        id: Some(0),
                        kind: Some("pmos_candidate".to_string()),
                        gate_node: 10,
                        a_node: 1,
                        b_node: 2,
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
                    name: "CLK1".to_string(),
                    layout_node: Some(5),
                    evidence: Evidence { anchor: true },
                },
                Signal {
                    name: "DATA".to_string(),
                    layout_node: Some(10),
                    evidence: Evidence { anchor: false },
                },
            ],
        }
    }

    #[test]
    fn identifies_vdd_vss() {
        let net = make_test_netlist();
        let rails = identify_power_rails(&net);
        assert_eq!(rails.vdd_node, Some(1));
        assert_eq!(rails.vss_node, Some(2));
    }

    #[test]
    fn identifies_clock() {
        let net = make_test_netlist();
        let rails = identify_power_rails(&net);
        assert_eq!(rails.clock_nodes, vec![5]);
    }

    #[test]
    fn ignores_non_anchor_signals() {
        let net = make_test_netlist();
        let rails = identify_power_rails(&net);
        // DATA is not an anchor, so it should not appear anywhere
        assert_ne!(rails.vdd_node, Some(10));
        assert_ne!(rails.vss_node, Some(10));
    }

    #[test]
    fn node_has_connections_true() {
        let net = make_test_netlist();
        assert!(node_has_connections(&net, 1)); // a_node of transistor 0
        assert!(node_has_connections(&net, 10)); // gate_node
    }

    #[test]
    fn node_has_connections_false() {
        let net = make_test_netlist();
        assert!(!node_has_connections(&net, 999));
    }
}
