//! Solver bridge for the Intel 4004 CPU.
//!
//! WHY: Connects the behavioral 4004 model to the transistor-level solver
//! infrastructure, enabling analog validation of internal subcircuits.
//!
//! WHAT: Implements `ChipSolverBridge` for `I4004`, exposing the
//! "clock_buffer" subcircuit (3-inverter chain) as a proof of concept.
//!
//! HOW: The clock buffer uses 6 transistors (3 enhancement + 3 depletion
//! loads) with Intel 10um pMOS geometry. The graph is suitable for DC
//! operating point analysis and transient pulse response simulation.

use mcs4_core::{
    bridge::{ChipSolverBridge, PinDirection, PinMapping},
    circuit::graph::{CircuitGraph, TransistorKind},
    fidelity::SimulationFidelity,
};

use super::I4004;

/// Subcircuit names from `docs/evidence/subcircuits_v0/4004/metrics.json`.
const JSON_SUBCIRCUIT_NAMES: &[&str] = &[
    "CLK1", "CMROM", "CMRAM0", "D0_PAD", "CLK2", "D2_PAD", "CMRAM3", "CMRAM2", "D1_PAD", "D3_PAD", "CMRAM1",
];

impl I4004 {
    /// Load a subcircuit from evidence JSON files.
    fn load_subcircuit_json(name: &str) -> Option<CircuitGraph> {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir.ancestors().nth(3)?;
        let path = repo_root
            .join("docs/evidence/subcircuits_v0/4004")
            .join(format!("4004_{}_subcircuit_v0.json", name));
        let netlist = mcs4_core::layout_netlist::load_netlist_v1(&path).ok()?;
        let config = mcs4_core::circuit::netlist_bridge::BridgeConfig::default();
        Some(mcs4_core::circuit::netlist_bridge::netlist_v1_to_circuit(
            &netlist, &config,
        ))
    }

    /// Build a 3-inverter chain subcircuit for clock buffering.
    ///
    /// Circuit topology (pMOS with depletion loads):
    /// ```text
    ///                VDD (-15V)
    ///                  |
    ///               [dep1]  (W=10um, L=40um depletion load)
    ///                  |
    ///   in ---[enh1]---+--- out1 ---[enh2]---+--- out2 ---[enh3]---+--- out3
    ///                  |            [dep2]    |            [dep3]   |
    ///                 VSS            |       VSS             |    VSS
    ///                               VDD                    VDD
    /// ```
    ///
    /// Each inverter stage: enhancement transistor (W=20um, L=10um) pulls
    /// output to VSS when gate is driven; depletion load (W=10um, L=40um)
    /// pulls output toward VDD when enhancement is off.
    pub fn build_clock_buffer() -> CircuitGraph {
        let mut g = CircuitGraph::new();

        // Power rails
        let vdd = g.add_node(-1);
        g.set_power_rail(vdd, -15.0, "VDD");
        let vss = g.add_node(-2);
        g.set_power_rail(vss, 0.0, "VSS");

        // Signal nodes
        let input = g.add_node(10);
        if let Some(n) = g.nodes.get_mut(input) {
            n.name = Some("CLK_IN".to_string());
            n.voltage = 0.0; // Start at VSS (logic high for pMOS)
            n.capacitance = 50e-15; // 50fF input cap
        }

        let out1 = g.add_node(11);
        if let Some(n) = g.nodes.get_mut(out1) {
            n.name = Some("INV1_OUT".to_string());
            n.voltage = -7.5; // Mid-rail initial guess
            n.capacitance = 100e-15; // 100fF
        }

        let out2 = g.add_node(12);
        if let Some(n) = g.nodes.get_mut(out2) {
            n.name = Some("INV2_OUT".to_string());
            n.voltage = -7.5;
            n.capacitance = 100e-15;
        }

        let out3 = g.add_node(13);
        if let Some(n) = g.nodes.get_mut(out3) {
            n.name = Some("CLK_OUT".to_string());
            n.voltage = -7.5;
            n.capacitance = 100e-15;
        }

        g.vdd_idx = Some(vdd);
        g.vss_idx = Some(vss);

        // Enhancement transistor geometry: W=20um, L=10um (2:1 W/L)
        let enh_w = 20e-6;
        let enh_l = 10e-6;

        // Depletion load geometry: W=10um, L=40um (0.25:1 W/L)
        // High L/W ratio gives high impedance for ratioed logic
        let dep_w = 10e-6;
        let dep_l = 40e-6;

        // Inverter 1: input -> out1
        // Enhancement: gate=input, drain=out1, source=vss
        g.add_transistor(input, out1, vss, TransistorKind::Enhancement, enh_w, enh_l);
        // Depletion load: gate=out1 (tied to source), drain=vdd, source=out1
        g.add_transistor(out1, vdd, out1, TransistorKind::Depletion, dep_w, dep_l);

        // Inverter 2: out1 -> out2
        g.add_transistor(out1, out2, vss, TransistorKind::Enhancement, enh_w, enh_l);
        g.add_transistor(out2, vdd, out2, TransistorKind::Depletion, dep_w, dep_l);

        // Inverter 3: out2 -> out3
        g.add_transistor(out2, out3, vss, TransistorKind::Enhancement, enh_w, enh_l);
        g.add_transistor(out3, vdd, out3, TransistorKind::Depletion, dep_w, dep_l);

        g
    }
}

impl ChipSolverBridge for I4004 {
    fn fidelity(&self) -> SimulationFidelity {
        self.fidelity
    }

    fn set_fidelity(&mut self, fidelity: SimulationFidelity) {
        self.fidelity = fidelity;
    }

    fn subcircuit_names(&self) -> Vec<&str> {
        let mut names = vec!["clock_buffer"];
        names.extend_from_slice(JSON_SUBCIRCUIT_NAMES);
        names
    }

    fn subcircuit(&self, name: &str) -> Option<CircuitGraph> {
        match name {
            "clock_buffer" => Some(Self::build_clock_buffer()),
            n if JSON_SUBCIRCUIT_NAMES.contains(&n) => Self::load_subcircuit_json(n),
            _ => None,
        }
    }

    fn pin_map(&self) -> Vec<PinMapping> {
        vec![
            PinMapping {
                name: "CLK1".to_string(),
                node_id: 415,
                direction: PinDirection::Input,
            },
            PinMapping {
                name: "CLK2".to_string(),
                node_id: 1230,
                direction: PinDirection::Input,
            },
            PinMapping {
                name: "SYNC".to_string(),
                node_id: 1261,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "RESET".to_string(),
                node_id: 1232,
                direction: PinDirection::Input,
            },
            PinMapping {
                name: "TEST".to_string(),
                node_id: 1267,
                direction: PinDirection::Input,
            },
            PinMapping {
                name: "CMROM".to_string(),
                node_id: 716,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "D0".to_string(),
                node_id: 3442,
                direction: PinDirection::Bidirectional,
            },
            PinMapping {
                name: "D1".to_string(),
                node_id: 598,
                direction: PinDirection::Bidirectional,
            },
            PinMapping {
                name: "D2".to_string(),
                node_id: 3426,
                direction: PinDirection::Bidirectional,
            },
            PinMapping {
                name: "D3".to_string(),
                node_id: 2815,
                direction: PinDirection::Bidirectional,
            },
            PinMapping {
                name: "CMRAM0".to_string(),
                node_id: 3431,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "CMRAM1".to_string(),
                node_id: 3428,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "CMRAM2".to_string(),
                node_id: 2997,
                direction: PinDirection::Output,
            },
            PinMapping {
                name: "CMRAM3".to_string(),
                node_id: 405,
                direction: PinDirection::Output,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use mcs4_core::{
        process::ProcessParams,
        solver::{
            stimulus::{PulseSource, Stimulus, Waveform},
            DcSolver, SolverConfig, TransientConfig, TransientSolver,
        },
    };

    use super::*;

    fn make_4004() -> I4004 {
        I4004::new()
    }

    #[test]
    fn bridge_subcircuit_names() {
        let cpu = make_4004();
        let names = cpu.subcircuit_names();
        assert_eq!(names.len(), 12); // clock_buffer + 11 from JSON
        assert_eq!(names[0], "clock_buffer");
        assert!(names.contains(&"CLK1"));
        assert!(names.contains(&"CMRAM1"));
    }

    #[test]
    fn bridge_fidelity_default() {
        let cpu = make_4004();
        assert_eq!(cpu.fidelity(), SimulationFidelity::Behavioral);
    }

    #[test]
    fn bridge_unknown_subcircuit_returns_none() {
        let cpu = make_4004();
        assert!(cpu.subcircuit("nonexistent").is_none());
    }

    #[test]
    fn clock_buffer_graph_structure() {
        let cpu = make_4004();
        let g = cpu.subcircuit("clock_buffer").expect("should return clock_buffer");

        // 6 nodes: VDD, VSS, input, out1, out2, out3
        assert_eq!(g.nodes.len(), 6, "Expected 6 nodes");
        // 6 transistors: 3 enhancement + 3 depletion
        assert_eq!(g.transistor_count(), 6, "Expected 6 transistors");
        // 4 free nodes (input, out1, out2, out3)
        assert_eq!(g.free_node_count(), 4, "Expected 4 free nodes");

        // Power rails
        assert!(g.vdd_idx.is_some());
        assert!(g.vss_idx.is_some());

        // Verify transistor types
        let enh_count = g
            .transistors
            .iter()
            .filter(|t| t.kind == TransistorKind::Enhancement)
            .count();
        let dep_count = g
            .transistors
            .iter()
            .filter(|t| t.kind == TransistorKind::Depletion)
            .count();
        assert_eq!(enh_count, 3, "Expected 3 enhancement transistors");
        assert_eq!(dep_count, 3, "Expected 3 depletion loads");
    }

    #[test]
    fn clock_buffer_dc_convergence() {
        let cpu = make_4004();
        let mut g = cpu.subcircuit("clock_buffer").expect("graph");
        let params = ProcessParams::default();
        let config = SolverConfig::small_circuit();

        let solver = DcSolver::new(config, params);
        let result = solver.solve(&mut g);
        assert!(
            result.converged,
            "DC solver should converge for clock buffer (iterations={})",
            result.iterations
        );

        // With input at VSS (0V = logic high for pMOS):
        //   Enhancement pMOS: conducts when Vgs < Vth (Vth ~ -3V)
        //   Gate = input = 0V, source = VSS = 0V -> Vgs = 0V > Vth => OFF
        //   So with input = 0V, enhancement is OFF, depletion load pulls out1 -> VDD
        //   out1 ~ -15V -> enh2 gate = -15V, source = 0V -> Vgs = -15V < -3V => ON
        //   out2 ~ 0V -> enh3 OFF, out3 ~ -15V
        //
        // Expected: out1 ~ VDD (-15V), out2 ~ VSS (0V), out3 ~ VDD (-15V)

        let out1_idx = 3;
        let out3_idx = 5;

        // out1 should be near VDD (-15V) since input=0V turns enh OFF
        let out1_v = result.voltages[out1_idx];
        assert!(out1_v < -10.0, "out1 should be near VDD (-15V), got {:.2}V", out1_v);

        // out3 should also be near VDD (-15V) (odd inversions from "high" input)
        let out3_v = result.voltages[out3_idx];
        assert!(out3_v < -10.0, "out3 should be near VDD (-15V), got {:.2}V", out3_v);
    }

    #[test]
    fn clock_buffer_transient_produces_waveforms() {
        let cpu = make_4004();
        let mut g = cpu.subcircuit("clock_buffer").expect("graph");
        let params = ProcessParams::default();
        let solver_config = SolverConfig::small_circuit();

        // First get DC operating point to initialize
        let solver = DcSolver::new(solver_config, params.clone());
        let dc = solver.solve(&mut g);
        assert!(dc.converged, "DC must converge first");

        // Apply a pulse to the input: 0V -> -15V step (logic low)
        let input_idx = 2;
        let stimuli = vec![Stimulus {
            node_idx: input_idx,
            waveform: Waveform::Pulse(PulseSource {
                v_low: 0.0,
                v_high: -15.0,
                delay: 10e-9,
                t_rise: 1e-9,
                t_fall: 1e-9,
                t_width: 200e-9,
                period: 500e-9,
            }),
        }];

        let transient_config = TransientConfig {
            dt: 1e-9,
            t_stop: 100e-9,
            adaptive: false,
            ..TransientConfig::default()
        };

        let result = TransientSolver::run(&mut g, &params, &transient_config, &stimuli);

        // Verify we got waveform points
        assert!(
            result.waveforms.len() > 10,
            "Should have multiple waveform points, got {}",
            result.waveforms.len()
        );

        // Verify time progresses
        let first_time = result.waveforms.first().map(|w| w.time).unwrap_or(0.0);
        let last_time = result.waveforms.last().map(|w| w.time).unwrap_or(0.0);
        assert!(
            last_time > first_time,
            "Time should progress: first={:.2e}, last={:.2e}",
            first_time,
            last_time
        );
    }

    #[test]
    fn subcircuit_clk1_loads_437_transistors() {
        let cpu = make_4004();
        let g = cpu.subcircuit("CLK1").expect("should load CLK1 subcircuit");
        assert_eq!(g.transistor_count(), 437, "CLK1 should have 437 transistors");
        assert!(g.nodes.len() > 100, "CLK1 should have many nodes");
    }

    #[test]
    fn subcircuit_cmram1_loads_1_transistor() {
        let cpu = make_4004();
        let g = cpu.subcircuit("CMRAM1").expect("should load CMRAM1 subcircuit");
        assert_eq!(g.transistor_count(), 1, "CMRAM1 should have 1 transistor");
    }

    #[test]
    fn subcircuit_d2_pad_loads_57_transistors() {
        let cpu = make_4004();
        let g = cpu.subcircuit("D2_PAD").expect("should load D2_PAD subcircuit");
        assert_eq!(g.transistor_count(), 57, "D2_PAD should have 57 transistors");
        assert!(g.nodes.len() > 20, "D2_PAD should have many nodes");
    }
}
