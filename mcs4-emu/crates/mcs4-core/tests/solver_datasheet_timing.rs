//! Transient-solver propagation delay checked against the MCS-4 datasheet
//! clock windows (timing::clock_spec T0D1/T0D2).
//!
//! The two-phase clock leaves t0D2 = 150 ns (minimum) between a phi2 edge
//! and the next phi1 edge; ratioed pMOS logic clocked on one phase must
//! settle inside that gap. The test drives a single ratioed inverter --
//! enhancement driver W/L = 20/10 um, depletion load W/L = 10/40 um, the
//! same geometry as the 4004 clock-buffer bridge circuit -- with a pulse
//! shaped by the datasheet edge rates (t0R/t0F = 50 ns) and asserts the
//! simulated edge-start-to-output-mid-rail delay fits the T0D2 window,
//! recording the simulation-vs-datasheet delta in the assertion message.

use mcs4_core::{
    circuit::graph::{CircuitGraph, TransistorKind},
    process::ProcessParams,
    solver::{
        stimulus::{PulseSource, Stimulus, Waveform},
        TransientConfig, TransientSolver,
    },
    timing::clock_spec,
};

const VDD: f64 = -15.0;
const VSS: f64 = 0.0;
const V_MID: f64 = (VDD + VSS) / 2.0;

fn seconds(picoseconds: u64) -> f64 {
    picoseconds as f64 * 1e-12
}

/// First waveform time at which `node` crosses `threshold` moving in the
/// sign of `direction` (+1.0 rising toward VSS, -1.0 falling toward VDD).
fn crossing_time(
    waveforms: &[mcs4_core::solver::WaveformPoint],
    node: usize,
    threshold: f64,
    direction: f64,
) -> Option<f64> {
    waveforms.windows(2).find_map(|w| {
        let (a, b) = (w[0].voltages[node], w[1].voltages[node]);
        let crossed = if direction > 0.0 {
            a < threshold && b >= threshold
        } else {
            a > threshold && b <= threshold
        };
        crossed.then(|| {
            // Linear interpolation between the bracketing samples.
            let frac = (threshold - a) / (b - a);
            w[0].time + frac * (w[1].time - w[0].time)
        })
    })
}

#[test]
fn inverter_propagation_delay_fits_datasheet_clock_windows() {
    let process = ProcessParams::default();

    let mut g = CircuitGraph::new();
    let vdd = g.add_node(-1);
    g.set_power_rail(vdd, VDD, "VDD");
    let vss = g.add_node(-2);
    g.set_power_rail(vss, VSS, "VSS");
    g.vdd_idx = Some(vdd);
    g.vss_idx = Some(vss);

    let input = g.add_node(10);
    g.nodes[input].voltage = VSS;
    g.nodes[input].capacitance = 50e-15;
    let output = g.add_node(11);
    g.nodes[output].voltage = VDD;
    g.nodes[output].capacitance = 100e-15;

    // Ratioed pMOS inverter: enhancement driver pulls the output to VSS when
    // the input asserts VDD; the source-tied depletion load restores VDD.
    g.add_transistor(input, output, vss, TransistorKind::Enhancement, 20e-6, 10e-6);
    g.add_transistor(output, vdd, output, TransistorKind::Depletion, 10e-6, 40e-6);

    // Input pulse shaped by the datasheet clock: 50 ns edges (t0R/t0F),
    // 480 ns asserted width (t0PW typical), one minimum clock period.
    let stimuli = [Stimulus {
        node_idx: input,
        waveform: Waveform::Pulse(PulseSource {
            v_low: VSS,
            v_high: VDD,
            delay: 100e-9,
            t_rise: seconds(clock_spec::T0R),
            t_fall: seconds(clock_spec::T0F),
            t_width: 480e-9,
            period: seconds(clock_spec::TCY_MIN),
        }),
    }];

    let config = TransientConfig {
        dt: 1e-9,
        t_stop: 800e-9,
        adaptive: false,
        ..TransientConfig::default()
    };
    let result = TransientSolver::run(&mut g, &process, &config, &stimuli);
    assert!(
        result.waveforms.len() > 100,
        "transient produced too few points: {}",
        result.waveforms.len()
    );

    // The input crosses mid-rail after the driver threshold, so a fast
    // ratioed stage legitimately crosses mid-rail before its slow input
    // does; 50%-to-50% delay goes negative and measures nothing useful
    // against a phase gap. What must fit inside t0D2 is the whole
    // response: from the start of the driving clock edge to the output
    // reaching mid-rail. The edge start is the pulse delay; the measured
    // interval conservatively includes the full 50 ns input edge.
    let t_edge_start = 100e-9;
    let t_in_mid =
        crossing_time(&result.waveforms, input, V_MID, -1.0).expect("input never crossed mid-rail while asserting");
    assert!(
        t_in_mid > t_edge_start,
        "input mid-rail crossing {t_in_mid:.3e}s precedes the edge start"
    );
    let t_out = crossing_time(&result.waveforms, output, V_MID, 1.0).expect("output never crossed mid-rail toward VSS");
    let delay = t_out - t_edge_start;

    let t0d2_min = seconds(clock_spec::T0D2_MIN);
    let t0d1_min = seconds(clock_spec::T0D1_MIN);

    assert!(
        delay > 0.0,
        "output crossed before the clock edge began: delay={delay:.3e}s"
    );
    assert!(
        delay < t0d2_min,
        "edge-start-to-output delay {:.1} ns exceeds the t0D2 minimum \
         phase gap {:.1} ns (delta {:.1} ns): logic would not settle between \
         phi2 and the next phi1",
        delay * 1e9,
        t0d2_min * 1e9,
        (delay - t0d2_min) * 1e9,
    );
    assert!(
        delay < t0d1_min,
        "edge-start-to-output delay {:.1} ns exceeds the t0D1 minimum \
         phase gap {:.1} ns",
        delay * 1e9,
        t0d1_min * 1e9,
    );

    // Record the simulation-vs-datasheet delta: the margin between the
    // simulated stage delay and the tightest datasheet phase gap.
    let margin_ns = (t0d2_min - delay) * 1e9;
    println!(
        "solver-vs-datasheet: stage delay {:.2} ns, t0D2 window 150 ns, margin {:.2} ns",
        delay * 1e9,
        margin_ns
    );
}
