//! Circuit graph representation for analog/transistor-level simulation.
//!
//! Provides a typed graph structure that the solver consumes. Each node
//! carries a voltage and capacitance; each transistor connects three nodes
//! (gate, a-terminal, b-terminal) with geometry and model parameters.

use std::collections::HashMap;

/// Interconnect layer on the die.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetalLayer {
    /// Polysilicon gate layer.
    Poly,
    /// p+ diffusion layer.
    Diffusion,
    /// Aluminum metallization (single layer for 10um process).
    Metal1,
}

/// A physical wire segment on the die.
#[derive(Clone, Debug)]
pub struct WireSegment {
    /// Start X coordinate (um).
    pub x0: f64,
    /// Start Y coordinate (um).
    pub y0: f64,
    /// End X coordinate (um).
    pub x1: f64,
    /// End Y coordinate (um).
    pub y1: f64,
    /// Wire width (um).
    pub width: f64,
    /// Interconnect layer.
    pub layer: MetalLayer,
}

impl WireSegment {
    /// Length of this segment (um).
    pub fn length(&self) -> f64 {
        let dx = self.x1 - self.x0;
        let dy = self.y1 - self.y0;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Transistor operating mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransistorKind {
    /// Enhancement-mode pMOS (normal switching transistor).
    Enhancement,
    /// Depletion-mode pMOS (active load, gate tied to source).
    Depletion,
}

/// A transistor instance in the circuit graph.
#[derive(Clone, Debug)]
pub struct AnalogTransistor {
    /// Unique transistor index.
    pub id: usize,
    /// Gate node index.
    pub gate: usize,
    /// Terminal A node index (source or drain, determined by voltage).
    pub a_node: usize,
    /// Terminal B node index (source or drain, determined by voltage).
    pub b_node: usize,
    /// Bulk (substrate/body) node index. None means bulk is tied to source
    /// (zero body effect), which is the default for backward compatibility.
    pub bulk: Option<usize>,
    /// Enhancement or depletion mode.
    pub kind: TransistorKind,
    /// Gate width (meters).
    pub w: f64,
    /// Gate length (meters).
    pub l: f64,
    /// X coordinate of transistor center on die (um). None if not spatially mapped.
    pub x_center: Option<f64>,
    /// Y coordinate of transistor center on die (um). None if not spatially mapped.
    pub y_center: Option<f64>,
}

/// A node (net) in the circuit graph.
#[derive(Clone, Debug)]
pub struct AnalogNode {
    /// Unique node index.
    pub id: usize,
    /// Original netlist node number (for traceability).
    pub netlist_id: i32,
    /// Human-readable name (if identified as a signal).
    pub name: Option<String>,
    /// Initial/current voltage (V).
    pub voltage: f64,
    /// Total lumped capacitance to ground (F).
    pub capacitance: f64,
    /// True if this is a fixed power rail (VDD or VSS).
    pub is_fixed: bool,
    /// Wire segments for this net. Empty if not spatially mapped.
    pub wire_segments: Vec<WireSegment>,
}

/// Complete circuit graph for simulation.
#[derive(Clone, Debug)]
pub struct CircuitGraph {
    /// All nodes, indexed by node ID.
    pub nodes: Vec<AnalogNode>,
    /// All transistors.
    pub transistors: Vec<AnalogTransistor>,
    /// Map from original netlist node number to graph node index.
    pub node_map: HashMap<i32, usize>,
    /// VDD node index.
    pub vdd_idx: Option<usize>,
    /// VSS node index.
    pub vss_idx: Option<usize>,
}

impl CircuitGraph {
    /// Create an empty circuit graph.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            transistors: Vec::new(),
            node_map: HashMap::new(),
            vdd_idx: None,
            vss_idx: None,
        }
    }

    /// Add a node, returning its index. If the netlist_id already exists,
    /// returns the existing index.
    pub fn add_node(&mut self, netlist_id: i32) -> usize {
        if let Some(&idx) = self.node_map.get(&netlist_id) {
            return idx;
        }
        let idx = self.nodes.len();
        self.nodes.push(AnalogNode {
            id: idx,
            netlist_id,
            name: None,
            voltage: 0.0,
            capacitance: 0.0,
            is_fixed: false,
            wire_segments: Vec::new(),
        });
        self.node_map.insert(netlist_id, idx);
        idx
    }

    /// Add a transistor, returning its index.
    pub fn add_transistor(
        &mut self,
        gate: usize,
        a_node: usize,
        b_node: usize,
        kind: TransistorKind,
        w: f64,
        l: f64,
    ) -> usize {
        let id = self.transistors.len();
        self.transistors.push(AnalogTransistor {
            id,
            gate,
            a_node,
            b_node,
            bulk: None,
            kind,
            w,
            l,
            x_center: None,
            y_center: None,
        });
        id
    }

    /// Set a node as a fixed power rail.
    pub fn set_power_rail(&mut self, idx: usize, voltage: f64, name: &str) {
        if let Some(node) = self.nodes.get_mut(idx) {
            node.voltage = voltage;
            node.is_fixed = true;
            node.name = Some(name.to_string());
        }
    }

    /// Number of non-fixed (free) nodes in the circuit.
    pub fn free_node_count(&self) -> usize {
        self.nodes.iter().filter(|n| !n.is_fixed).count()
    }

    /// Number of transistors.
    pub fn transistor_count(&self) -> usize {
        self.transistors.len()
    }

    /// Initialize all non-fixed nodes to a given voltage.
    pub fn init_voltages(&mut self, voltage: f64) {
        for node in &mut self.nodes {
            if !node.is_fixed {
                node.voltage = voltage;
            }
        }
    }

    /// Get all transistors connected to a given node (as gate, source, or drain).
    pub fn transistors_at_node(&self, node_idx: usize) -> Vec<usize> {
        self.transistors
            .iter()
            .filter(|t| t.gate == node_idx || t.a_node == node_idx || t.b_node == node_idx)
            .map(|t| t.id)
            .collect()
    }
}

impl Default for CircuitGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph() {
        let g = CircuitGraph::new();
        assert_eq!(g.nodes.len(), 0);
        assert_eq!(g.transistors.len(), 0);
    }

    #[test]
    fn add_nodes_and_transistor() {
        let mut g = CircuitGraph::new();
        let vdd = g.add_node(-1);
        let vss = g.add_node(-2);
        let n1 = g.add_node(10);

        g.set_power_rail(vdd, -15.0, "VDD");
        g.set_power_rail(vss, 0.0, "VSS");

        let _t = g.add_transistor(n1, vdd, vss, TransistorKind::Enhancement, 10e-6, 10e-6);

        assert_eq!(g.nodes.len(), 3);
        assert_eq!(g.transistor_count(), 1);
        assert_eq!(g.free_node_count(), 1); // only n1 is free
    }

    #[test]
    fn duplicate_node_returns_same_index() {
        let mut g = CircuitGraph::new();
        let a = g.add_node(42);
        let b = g.add_node(42);
        assert_eq!(a, b);
        assert_eq!(g.nodes.len(), 1);
    }

    #[test]
    fn transistors_at_node_finds_connections() {
        let mut g = CircuitGraph::new();
        let n0 = g.add_node(0);
        let n1 = g.add_node(1);
        let n2 = g.add_node(2);
        let n3 = g.add_node(3);

        g.add_transistor(n0, n1, n2, TransistorKind::Enhancement, 10e-6, 10e-6);
        g.add_transistor(n0, n2, n3, TransistorKind::Depletion, 10e-6, 20e-6);

        let at_n0 = g.transistors_at_node(n0);
        assert_eq!(at_n0.len(), 2, "Node 0 is gate of both transistors");

        let at_n2 = g.transistors_at_node(n2);
        assert_eq!(at_n2.len(), 2, "Node 2 appears in both transistors");

        let at_n3 = g.transistors_at_node(n3);
        assert_eq!(at_n3.len(), 1);
    }

    #[test]
    fn init_voltages_only_changes_free_nodes() {
        let mut g = CircuitGraph::new();
        let vdd = g.add_node(-1);
        let n1 = g.add_node(10);
        g.set_power_rail(vdd, -15.0, "VDD");

        g.init_voltages(-7.5);

        assert!(
            (g.nodes[vdd].voltage - (-15.0)).abs() < 1e-10,
            "Fixed node should not change"
        );
        assert!(
            (g.nodes[n1].voltage - (-7.5)).abs() < 1e-10,
            "Free node should be initialized"
        );
    }
}
