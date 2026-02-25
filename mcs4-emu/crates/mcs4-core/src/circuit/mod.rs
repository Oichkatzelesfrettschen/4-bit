//! Circuit graph and netlist bridge for transistor-level simulation.
//!
//! Converts extracted JSON netlists (NetlistV1 format) into a typed
//! circuit graph suitable for the SPICE-class solver. Handles:
//! - Transistor connectivity (gate, source/drain assignment)
//! - Power rail identification from signal anchors
//! - Geometry estimation from pixel bounding boxes
//! - Node merging and renumbering

pub mod arch_map;
pub mod bbox_to_geometry;
pub mod graph;
pub mod netlist_bridge;
pub mod parasitic_extract;
pub mod power_rail_id;
pub mod spatial;

pub use arch_map::{ArchBlock, ArchMapping, BlockAnnotation, BlockStats};
pub use graph::{AnalogNode, AnalogTransistor, CircuitGraph, MetalLayer, TransistorKind, WireSegment};
pub use parasitic_extract::{LayerParams, NetParasitics, SegmentParasitics};
pub use spatial::{DieCoordinateSystem, DieDimensions};
