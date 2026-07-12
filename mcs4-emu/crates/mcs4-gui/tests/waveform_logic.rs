//! GUI trace-model integration tests.
//!
//! The GUI consumes immutable, post-phase `TraceFrame` values on its UI thread.
//! The behavioral worker owns mutable emulator state and never shares an
//! `Mcs4System` lock with the renderer.

use mcs4_gui::{
    panels::waveform::{CursorState, WaveformPanel},
    signal_trace::{FrameId, SignalTrace},
};
use mcs4_system::{Mcs4System, ReplaySession, TracePhase};

fn capture_full_cycle() -> SignalTrace {
    let mut session = ReplaySession::<Mcs4System>::new();
    let mut trace = SignalTrace::new();
    for _ in 0..8 {
        let frame = session.step_phase().expect("step phase");
        trace.push_frame(frame).expect("retain frame");
    }
    trace
}

#[test]
fn trace_retains_one_post_phase_frame_per_bus_phase() {
    let trace = capture_full_cycle();
    assert_eq!(trace.len(), 8);

    let frames: Vec<_> = trace.iter().collect();
    assert_eq!(frames[0].sequence, 1);
    assert_eq!(frames[7].sequence, 8);
    assert_eq!(
        frames[0].phase.as_ref().expect("first phase").completed_phase,
        TracePhase::A1
    );
    assert_eq!(
        frames[7].phase.as_ref().expect("last phase").completed_phase,
        TracePhase::X3
    );
}

#[test]
fn frame_identity_stays_unique_when_machine_cycle_is_shared() {
    let trace = capture_full_cycle();
    let frames: Vec<_> = trace.iter().collect();
    let first = FrameId::from(frames[0]);
    let second = FrameId::from(frames[1]);

    assert_eq!(frames[0].phase.as_ref().expect("phase").machine_cycles, 0);
    assert_eq!(frames[1].phase.as_ref().expect("phase").machine_cycles, 0);
    assert_ne!(first, second);
    assert_eq!(trace.frame(second).expect("second frame").sequence, 2);
}

#[test]
fn waveform_measurement_uses_frame_identity_not_display_offset() {
    let trace = capture_full_cycle();
    let first = FrameId::from(trace.frame_at(0).expect("first frame"));
    let last = FrameId::from(trace.frame_at(7).expect("last frame"));
    let mut cursors = CursorState::default();
    cursors.set_marker_a(first);
    cursors.set_marker_b(last);

    let delta = cursors.marker_delta().expect("same run marker delta");
    assert_eq!(delta.phase_delta, 7);
}

#[test]
fn waveform_window_math_uses_frames_per_pixel() {
    let mut panel = WaveformPanel::new();
    panel.set_zoom(40.0);

    assert_eq!(panel.pixel_to_frame_offset(0.0), 0);
    assert_eq!(panel.pixel_to_frame_offset(39.0), 0);
    assert_eq!(panel.pixel_to_frame_offset(40.0), 1);
    assert_eq!(panel.pixel_to_frame_offset(119.0), 2);
}

#[test]
fn clearing_trace_drops_old_run_history_before_a_reset_run_arrives() {
    let mut trace = capture_full_cycle();
    let old = FrameId::from(trace.frame_at(0).expect("old frame"));
    trace.clear();
    assert!(trace.is_empty());
    assert!(trace.frame(old).is_none());

    let mut session = ReplaySession::<Mcs4System>::new();
    let _ = session.step_phase().expect("first frame");
    session
        .apply_input(mcs4_system::ReplayInput::Reset)
        .expect("reset session");
    let new = session.step_phase().expect("new run frame");
    trace.push_frame(new).expect("retain new frame");
    assert_eq!(trace.frame_at(0).expect("retained frame").run_id, 2);
    assert_eq!(trace.frame_at(0).expect("retained frame").sequence, 1);
}
