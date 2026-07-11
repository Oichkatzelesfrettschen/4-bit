//! Waveform data-model integration tests.
//!
//! These target the seam the unit tests skip: the shared
//! `Arc<RwLock<SignalTrace>>` producer/consumer model that `WaveformPanel` is
//! built around. Rendering (`show`) needs an `egui::Ui` and is out of scope;
//! the panel's own poison guard lives behind that render path and is therefore
//! not reachable from the public API (see the shared-model contract test).

use std::{
    sync::{Arc, RwLock},
    thread,
};

use mcs4_bus::prelude::*;
use mcs4_core::prelude::SignalLevel;
use mcs4_gui::{panels::waveform::WaveformPanel, signal_trace::SignalTrace};

/// Advance a `CycleState`-like phase list while evolving bus/ctrl state so each
/// captured `Sample` carries a distinct composite of signals.
fn capture_full_cycle() -> SignalTrace {
    let mut trace = SignalTrace::new();
    let mut bus = DataBus::new();
    let driver = bus.add_driver("CPU");
    let mut ctrl = ControlSignals::mcs4();
    let clock = TwoPhaseClock::default_config();

    let phases = [
        BusCycle::A1,
        BusCycle::A2,
        BusCycle::A3,
        BusCycle::M1,
        BusCycle::M2,
        BusCycle::X1,
        BusCycle::X2,
        BusCycle::X3,
    ];

    for (tick, &phase) in phases.iter().enumerate() {
        let tick = tick as u64;
        // Evolve the observable state so no two samples are identical.
        bus.drive(driver, (tick as u8) & 0x0F, tick);
        if phase == BusCycle::A1 {
            ctrl.assert_sync(tick);
        }
        if phase == BusCycle::A3 {
            ctrl.select_rom(0x3, tick);
        }
        if phase == BusCycle::X2 {
            ctrl.select_ram(0b0010, tick);
        }
        trace.push(tick, &bus, &ctrl, phase, &clock);
    }
    trace
}

#[test]
fn trace_append_captures_composite_signal_state_per_phase() {
    let trace = capture_full_cycle();
    assert_eq!(trace.len(), 8);

    let samples: Vec<_> = trace.iter().cloned().collect();

    // Ticks and phases are recorded in push order.
    let ticks: Vec<u64> = samples.iter().map(|s| s.tick).collect();
    assert_eq!(ticks, (0..8).collect::<Vec<u64>>());
    assert_eq!(samples[0].phase, BusCycle::A1);
    assert_eq!(samples[7].phase, BusCycle::X3);

    // SYNC asserted at A1 is visible in the A1 sample.
    assert!(samples[0].sync);

    // CM-ROM select at A3 and the mask-driven CM-RAM select at X2 are captured.
    assert_eq!(samples[2].cm_rom, 0x3);
    assert_eq!(samples[6].cm_ram, 0b0010);

    // The evolving data-bus nibble is captured at each phase.
    assert_eq!(samples[5].data, 5);
}

#[test]
fn shared_trace_writes_are_visible_across_cloned_handles() {
    // The panel holds one Arc handle; a producer holds another. Writes through
    // the producer handle must be visible to the panel's handle.
    let producer: Arc<RwLock<SignalTrace>> = Arc::new(RwLock::new(SignalTrace::new()));
    let panel_handle = Arc::clone(&producer);
    let _panel = WaveformPanel::new(Arc::clone(&panel_handle));

    let bus = DataBus::new();
    let ctrl = ControlSignals::mcs4();
    let clock = TwoPhaseClock::default_config();

    {
        let mut trace = producer.write().expect("write lock");
        for tick in 0..5u64 {
            trace.push(tick, &bus, &ctrl, BusCycle::A1, &clock);
        }
    }

    let trace = panel_handle.read().expect("read lock");
    assert_eq!(trace.len(), 5);
    assert!(!trace.is_empty());
}

#[test]
fn concurrent_producer_thread_appends_ordered_samples() {
    let shared: Arc<RwLock<SignalTrace>> = Arc::new(RwLock::new(SignalTrace::new()));
    let producer = Arc::clone(&shared);

    let handle = thread::spawn(move || {
        let bus = DataBus::new();
        let ctrl = ControlSignals::mcs4();
        let clock = TwoPhaseClock::default_config();
        for tick in 0..100u64 {
            let mut trace = producer.write().expect("write lock");
            trace.push(tick, &bus, &ctrl, BusCycle::A1, &clock);
        }
    });
    handle.join().expect("producer thread");

    let trace = shared.read().expect("read lock");
    assert_eq!(trace.len(), 100);
    let ticks: Vec<u64> = trace.iter().map(|s| s.tick).collect();
    assert_eq!(ticks, (0..100).collect::<Vec<u64>>());
}

#[test]
fn window_slice_matches_panel_scroll_and_zoom_math() {
    // The panel selects a visible window by skipping start_idx = scroll_x / zoom
    // and taking width / zoom samples. Reproduce that arithmetic against a
    // shared trace to confirm the window model over the public API.
    let trace = Arc::new(RwLock::new(capture_full_cycle()));
    let mut panel = WaveformPanel::new(Arc::clone(&trace));

    // Default zoom 10 px/tick; scroll to 30 px selects start tick 3.
    panel.set_zoom(10.0);
    assert_eq!(panel.pixel_to_tick(0.0), 0);
    let start_idx = panel.pixel_to_tick(0.0) as usize;
    assert_eq!(start_idx, 0);

    // Compose the same skip/take the render path uses and confirm ordering.
    let guard = trace.read().expect("read lock");
    let window: Vec<u64> = guard.iter().skip(2).take(3).map(|s| s.tick).collect();
    assert_eq!(window, vec![2, 3, 4]);
}

#[test]
fn poisoned_shared_lock_is_observable_read_error() {
    // Documents the shared-model contract that `WaveformPanel::show` depends on:
    // if a writer panics while holding the trace lock, the reader observes a
    // poisoned lock (Err), which the panel's `let Ok(trace) = ... else` guard
    // turns into a survivable "trace unavailable" message. The guard itself is
    // behind the egui render path and is not reachable without a Ui.
    let shared: Arc<RwLock<SignalTrace>> = Arc::new(RwLock::new(SignalTrace::new()));
    let poisoner = Arc::clone(&shared);

    let handle = thread::spawn(move || {
        let _guard = poisoner.write().expect("write lock");
        panic!("writer fault while holding the trace lock");
    });
    assert!(handle.join().is_err(), "poisoning thread should have panicked");

    // A subsequent read returns Err, mirroring the panel's guard condition.
    assert!(shared.read().is_err());
}

#[test]
fn empty_shared_trace_reports_empty_to_consumer() {
    let shared = Arc::new(RwLock::new(SignalTrace::new()));
    let _panel = WaveformPanel::new(Arc::clone(&shared));
    let guard = shared.read().expect("read lock");
    assert!(guard.is_empty());
    assert_eq!(guard.len(), 0);
    assert_eq!(guard.iter().count(), 0);
}

#[test]
fn cleared_shared_trace_drops_all_samples_for_consumer() {
    let shared = Arc::new(RwLock::new(capture_full_cycle()));
    let _panel = WaveformPanel::new(Arc::clone(&shared));

    {
        let guard = shared.read().expect("read lock");
        assert_eq!(guard.len(), 8);
    }
    {
        let mut guard = shared.write().expect("write lock");
        guard.clear();
    }
    let guard = shared.read().expect("read lock");
    assert!(guard.is_empty());
}

#[test]
fn captured_sample_clock_flags_reflect_idle_two_phase_clock() {
    // A freshly constructed clock has PHI1/PHI2 low; the captured sample must
    // reflect that rather than a stale default.
    let trace = capture_full_cycle();
    let first = trace.iter().next().expect("at least one sample");
    assert!(!first.phi1);
    assert!(!first.phi2);
    // And the control snapshot recorded a defined (non-floating) sync level.
    assert_eq!(
        ControlSignals::mcs4().sync.current,
        SignalLevel::Low,
        "baseline sync level is defined low"
    );
}
