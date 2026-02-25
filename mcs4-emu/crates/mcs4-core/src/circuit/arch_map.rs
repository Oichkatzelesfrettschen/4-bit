//! Transistor-to-architecture block mapping for the 4004 die.
//!
//! Associates each transistor with its functional role in the CPU
//! architecture based on spatial position and connectivity patterns.
//! This enables per-block analysis: transistor count, area, power
//! density, and critical path identification.
//!
//! Block assignments are based on the 4004.com reverse-engineering
//! annotations and the known 4004 floor plan layout.

use std::collections::HashMap;

use crate::circuit::graph::CircuitGraph;

/// Architectural functional blocks of the Intel 4004.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ArchBlock {
    /// 4-bit ALU (adder, complement, decimal adjust).
    Alu,
    /// 16 x 4-bit index register file (4 register pairs).
    RegisterFile,
    /// Instruction decoder ROM and control logic.
    Decoder,
    /// 3-level subroutine stack (program counter stack).
    Stack,
    /// Two-phase clock generation and distribution tree.
    ClockTree,
    /// I/O buffers and bus interface (data, address).
    IoBuffer,
    /// Accumulator and carry flag.
    Accumulator,
    /// Timing and sequencing (machine cycle FSM).
    TimingSequencer,
    /// Not yet classified.
    Unassigned,
}

impl ArchBlock {
    /// Human-readable short name for display.
    pub fn short_name(self) -> &'static str {
        match self {
            Self::Alu => "ALU",
            Self::RegisterFile => "REG",
            Self::Decoder => "DEC",
            Self::Stack => "STK",
            Self::ClockTree => "CLK",
            Self::IoBuffer => "IO",
            Self::Accumulator => "ACC",
            Self::TimingSequencer => "SEQ",
            Self::Unassigned => "???",
        }
    }
}

/// A block annotation for a single transistor.
#[derive(Clone, Debug)]
pub struct BlockAnnotation {
    /// Transistor index in the circuit graph.
    pub transistor_idx: usize,
    /// Assigned architectural block.
    pub block: ArchBlock,
    /// Confidence level (0.0 to 1.0). Higher means more certain.
    pub confidence: f64,
}

/// Statistics for a single architectural block.
#[derive(Clone, Debug, Default)]
pub struct BlockStats {
    /// Number of transistors in this block.
    pub transistor_count: usize,
    /// Bounding box: (min_x, min_y, max_x, max_y) in um. None if no spatial data.
    pub bounding_box: Option<(f64, f64, f64, f64)>,
    /// Estimated area from bounding box (um^2). None if no spatial data.
    pub area_um2: Option<f64>,
    /// Fraction of total transistors in this block.
    pub fraction: f64,
}

/// Complete architecture mapping for a chip.
#[derive(Clone, Debug)]
pub struct ArchMapping {
    /// Per-transistor block annotations.
    pub annotations: Vec<BlockAnnotation>,
    /// Per-block statistics.
    pub stats: HashMap<ArchBlock, BlockStats>,
    /// Total transistor count.
    pub total_transistors: usize,
}

/// Spatial region definition for block assignment.
///
/// Defines a rectangular region on the die (in um coordinates)
/// that corresponds to a known architectural block.
#[derive(Clone, Debug)]
pub struct SpatialRegion {
    /// Block this region maps to.
    pub block: ArchBlock,
    /// Minimum X coordinate (um).
    pub x_min: f64,
    /// Minimum Y coordinate (um).
    pub y_min: f64,
    /// Maximum X coordinate (um).
    pub x_max: f64,
    /// Maximum Y coordinate (um).
    pub y_max: f64,
}

impl SpatialRegion {
    /// Check if a point falls within this region.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x_min && x <= self.x_max && y >= self.y_min && y <= self.y_max
    }
}

/// Default spatial regions for the Intel 4004 die.
///
/// Based on the 4004.com reverse-engineering floor plan:
/// - Die is approximately 3200 x 3800 um
/// - Decoder/ROM at bottom
/// - Register file in the center-left
/// - ALU in the center
/// - Stack at top-right
/// - I/O buffers along edges
///
/// These are approximate regions; actual boundaries depend on the
/// extracted photomicrograph coordinate system.
pub fn default_4004_regions() -> Vec<SpatialRegion> {
    vec![
        SpatialRegion {
            block: ArchBlock::Decoder,
            x_min: 200.0,
            y_min: 0.0,
            x_max: 2800.0,
            y_max: 800.0,
        },
        SpatialRegion {
            block: ArchBlock::RegisterFile,
            x_min: 200.0,
            y_min: 800.0,
            x_max: 1200.0,
            y_max: 2200.0,
        },
        SpatialRegion {
            block: ArchBlock::Alu,
            x_min: 1200.0,
            y_min: 800.0,
            x_max: 2400.0,
            y_max: 2000.0,
        },
        SpatialRegion {
            block: ArchBlock::Accumulator,
            x_min: 2400.0,
            y_min: 800.0,
            x_max: 3000.0,
            y_max: 1600.0,
        },
        SpatialRegion {
            block: ArchBlock::Stack,
            x_min: 1200.0,
            y_min: 2200.0,
            x_max: 2800.0,
            y_max: 3200.0,
        },
        SpatialRegion {
            block: ArchBlock::TimingSequencer,
            x_min: 200.0,
            y_min: 2200.0,
            x_max: 1200.0,
            y_max: 3200.0,
        },
        SpatialRegion {
            block: ArchBlock::IoBuffer,
            x_min: 0.0,
            y_min: 0.0,
            x_max: 200.0,
            y_max: 3800.0,
        },
        SpatialRegion {
            block: ArchBlock::IoBuffer,
            x_min: 2800.0,
            y_min: 0.0,
            x_max: 3200.0,
            y_max: 3800.0,
        },
        SpatialRegion {
            block: ArchBlock::ClockTree,
            x_min: 0.0,
            y_min: 3200.0,
            x_max: 3200.0,
            y_max: 3800.0,
        },
    ]
}

/// Assign architectural blocks to transistors based on spatial position.
///
/// Uses the provided spatial regions to classify each transistor.
/// Transistors without spatial coordinates are marked as Unassigned.
/// If a transistor falls in multiple regions, the first matching region wins.
pub fn assign_blocks(graph: &CircuitGraph, regions: &[SpatialRegion]) -> Vec<BlockAnnotation> {
    graph
        .transistors
        .iter()
        .enumerate()
        .map(|(idx, t)| {
            let (block, confidence) = match (t.x_center, t.y_center) {
                (Some(x), Some(y)) => {
                    let matched = regions.iter().find(|r| r.contains(x, y));
                    match matched {
                        Some(r) => (r.block, 0.8), // spatial match, moderate confidence
                        None => (ArchBlock::Unassigned, 0.0),
                    }
                }
                _ => (ArchBlock::Unassigned, 0.0),
            };

            BlockAnnotation {
                transistor_idx: idx,
                block,
                confidence,
            }
        })
        .collect()
}

/// Compute per-block statistics from annotations.
pub fn compute_stats(graph: &CircuitGraph, annotations: &[BlockAnnotation]) -> HashMap<ArchBlock, BlockStats> {
    let total = annotations.len();
    let mut stats: HashMap<ArchBlock, BlockStats> = HashMap::new();

    for ann in annotations {
        let entry = stats.entry(ann.block).or_default();
        entry.transistor_count += 1;

        // Update bounding box if transistor has spatial data
        if let (Some(x), Some(y)) = (
            graph.transistors[ann.transistor_idx].x_center,
            graph.transistors[ann.transistor_idx].y_center,
        ) {
            match &mut entry.bounding_box {
                Some((x_min, y_min, x_max, y_max)) => {
                    *x_min = x_min.min(x);
                    *y_min = y_min.min(y);
                    *x_max = x_max.max(x);
                    *y_max = y_max.max(y);
                }
                None => {
                    entry.bounding_box = Some((x, y, x, y));
                }
            }
        }
    }

    // Compute derived fields
    for entry in stats.values_mut() {
        entry.fraction = if total > 0 {
            entry.transistor_count as f64 / total as f64
        } else {
            0.0
        };
        if let Some((x_min, y_min, x_max, y_max)) = entry.bounding_box {
            let w = (x_max - x_min).max(1.0);
            let h = (y_max - y_min).max(1.0);
            entry.area_um2 = Some(w * h);
        }
    }

    stats
}

/// Build a complete architecture mapping for a circuit.
pub fn map_architecture(graph: &CircuitGraph, regions: &[SpatialRegion]) -> ArchMapping {
    let annotations = assign_blocks(graph, regions);
    let stats = compute_stats(graph, &annotations);
    let total_transistors = graph.transistors.len();

    ArchMapping {
        annotations,
        stats,
        total_transistors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::graph::{CircuitGraph, TransistorKind};

    fn make_test_graph() -> CircuitGraph {
        let mut graph = CircuitGraph::new();

        // Add nodes
        let vss = graph.add_node(0);
        let vdd = graph.add_node(1);
        let n2 = graph.add_node(2);
        let n3 = graph.add_node(3);
        graph.set_power_rail(vss, 0.0, "VSS");
        graph.set_power_rail(vdd, -15.0, "VDD");

        // Add transistors at known locations
        // Transistor in ALU region
        let t0 = graph.add_transistor(n2, vss, n3, TransistorKind::Enhancement, 10e-6, 10e-6);
        graph.transistors[t0].x_center = Some(1800.0);
        graph.transistors[t0].y_center = Some(1400.0);

        // Transistor in decoder region
        let t1 = graph.add_transistor(n3, vdd, n2, TransistorKind::Depletion, 10e-6, 10e-6);
        graph.transistors[t1].x_center = Some(1500.0);
        graph.transistors[t1].y_center = Some(400.0);

        // Transistor in register file region
        let t2 = graph.add_transistor(n2, vss, n3, TransistorKind::Enhancement, 10e-6, 10e-6);
        graph.transistors[t2].x_center = Some(700.0);
        graph.transistors[t2].y_center = Some(1500.0);

        // Transistor without spatial data
        let _t3 = graph.add_transistor(n3, vdd, n2, TransistorKind::Enhancement, 10e-6, 10e-6);

        graph
    }

    #[test]
    fn default_regions_cover_die() {
        let regions = default_4004_regions();
        assert!(!regions.is_empty());

        // Should have all major blocks
        let blocks: Vec<ArchBlock> = regions.iter().map(|r| r.block).collect();
        assert!(blocks.contains(&ArchBlock::Alu));
        assert!(blocks.contains(&ArchBlock::RegisterFile));
        assert!(blocks.contains(&ArchBlock::Decoder));
        assert!(blocks.contains(&ArchBlock::Stack));
    }

    #[test]
    fn spatial_region_contains_works() {
        let region = SpatialRegion {
            block: ArchBlock::Alu,
            x_min: 100.0,
            y_min: 100.0,
            x_max: 200.0,
            y_max: 200.0,
        };

        assert!(region.contains(150.0, 150.0));
        assert!(region.contains(100.0, 100.0)); // boundary
        assert!(!region.contains(50.0, 150.0));
        assert!(!region.contains(150.0, 250.0));
    }

    #[test]
    fn assign_blocks_classifies_by_position() {
        let graph = make_test_graph();
        let regions = default_4004_regions();
        let annotations = assign_blocks(&graph, &regions);

        assert_eq!(annotations.len(), 4);

        // Transistor 0: ALU region (1800, 1400)
        assert_eq!(annotations[0].block, ArchBlock::Alu);
        assert!(annotations[0].confidence > 0.5);

        // Transistor 1: Decoder region (1500, 400)
        assert_eq!(annotations[1].block, ArchBlock::Decoder);

        // Transistor 2: Register file region (700, 1500)
        assert_eq!(annotations[2].block, ArchBlock::RegisterFile);

        // Transistor 3: no spatial data
        assert_eq!(annotations[3].block, ArchBlock::Unassigned);
        assert!((annotations[3].confidence - 0.0).abs() < 1e-10);
    }

    #[test]
    fn compute_stats_counts_correctly() {
        let graph = make_test_graph();
        let regions = default_4004_regions();
        let annotations = assign_blocks(&graph, &regions);
        let stats = compute_stats(&graph, &annotations);

        // ALU should have 1 transistor
        let alu_stats = stats.get(&ArchBlock::Alu).expect("ALU should be present");
        assert_eq!(alu_stats.transistor_count, 1);

        // Total fractions should sum to 1.0
        let total_fraction: f64 = stats.values().map(|s| s.fraction).sum();
        assert!(
            (total_fraction - 1.0).abs() < 1e-10,
            "Fractions should sum to 1.0, got {:.3}",
            total_fraction
        );
    }

    #[test]
    fn map_architecture_complete() {
        let graph = make_test_graph();
        let regions = default_4004_regions();
        let mapping = map_architecture(&graph, &regions);

        assert_eq!(mapping.total_transistors, 4);
        assert_eq!(mapping.annotations.len(), 4);
        assert!(!mapping.stats.is_empty());
    }

    #[test]
    fn block_stats_have_bounding_box_for_spatial() {
        let graph = make_test_graph();
        let regions = default_4004_regions();
        let annotations = assign_blocks(&graph, &regions);
        let stats = compute_stats(&graph, &annotations);

        // ALU block should have a bounding box (single point)
        let alu = stats.get(&ArchBlock::Alu).expect("ALU present");
        assert!(alu.bounding_box.is_some());
        assert!(alu.area_um2.is_some());

        // Unassigned block should NOT have bounding box (no spatial data)
        if let Some(unassigned) = stats.get(&ArchBlock::Unassigned) {
            assert!(unassigned.bounding_box.is_none());
        }
    }

    #[test]
    fn short_name_not_empty() {
        let blocks = [
            ArchBlock::Alu,
            ArchBlock::RegisterFile,
            ArchBlock::Decoder,
            ArchBlock::Stack,
            ArchBlock::ClockTree,
            ArchBlock::IoBuffer,
            ArchBlock::Accumulator,
            ArchBlock::TimingSequencer,
            ArchBlock::Unassigned,
        ];

        for block in blocks {
            assert!(!block.short_name().is_empty());
        }
    }

    #[test]
    fn empty_graph_no_crash() {
        let graph = CircuitGraph::new();
        let regions = default_4004_regions();
        let mapping = map_architecture(&graph, &regions);

        assert_eq!(mapping.total_transistors, 0);
        assert!(mapping.annotations.is_empty());
    }

    #[test]
    fn custom_regions_override_defaults() {
        let mut graph = CircuitGraph::new();
        let n0 = graph.add_node(0);
        let n1 = graph.add_node(1);

        let t = graph.add_transistor(n0, n1, n0, TransistorKind::Enhancement, 10e-6, 10e-6);
        graph.transistors[t].x_center = Some(50.0);
        graph.transistors[t].y_center = Some(50.0);

        // Custom region covering (50, 50)
        let regions = vec![SpatialRegion {
            block: ArchBlock::ClockTree,
            x_min: 0.0,
            y_min: 0.0,
            x_max: 100.0,
            y_max: 100.0,
        }];

        let annotations = assign_blocks(&graph, &regions);
        assert_eq!(annotations[0].block, ArchBlock::ClockTree);
    }
}
