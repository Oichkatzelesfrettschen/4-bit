//! Shared trace-frame fixture compatibility tests.

use mcs4_system::{
    compare_trace_frames, Mcs4System, ReplayInput, ReplaySession, TraceBackend, TraceComparisonError, TraceFidelity,
    TraceFrame, TraceLogic, TraceStimulusKind, TraceValue,
};

const I4003_VERILATOR_FRAME: &str = include_str!("../fixtures/traces/i4003-verilator-frame-v1.jsonl");
const MCS4_SYSTEM_VERILATOR_FRAME: &str = include_str!("../fixtures/traces/mcs4-system-verilator-frame-v1.jsonl");

#[test]
fn verilator_i4003_adapter_frame_matches_the_shared_contract() {
    let frame: TraceFrame = serde_json::from_str(I4003_VERILATOR_FRAME.trim()).expect("parse adapter frame");
    frame.validate().expect("validate adapter frame");

    assert_eq!(frame.provenance.backend, TraceBackend::Verilator);
    assert_eq!(frame.provenance.fidelity, TraceFidelity::Fpga);
    assert_eq!(frame.provenance.model_id, "i4003-fpga-verilator");
    assert!(frame.phase.is_none());
    assert!(frame.physical_time_ps.is_none());

    let parallel_output = frame.signal("i4003.parallel_out").expect("parallel output");
    assert_eq!(parallel_output.value, TraceValue::Bits { width: 10, value: 0 });
    let enable_n = frame.signal("i4003.enable_n").expect("active-low E");
    assert_eq!(enable_n.value, TraceValue::Logic { value: TraceLogic::One });
}

#[test]
fn comparison_requires_a_declared_stimulus_hash() {
    let frame: TraceFrame = serde_json::from_str(I4003_VERILATOR_FRAME.trim()).expect("parse adapter frame");
    assert_eq!(
        compare_trace_frames(&frame, &frame),
        Err(TraceComparisonError::MissingLeftStimulus)
    );
}

#[test]
fn comparison_rejects_unmapped_chip_adapter_signals() {
    let mut behavioral = ReplaySession::<Mcs4System>::new();
    behavioral
        .apply_input(ReplayInput::LoadRom { bytes: vec![0; 4] })
        .expect("load behavioral ROM");
    let behavioral_frame = behavioral.step_phase().expect("step behavioral model");

    let mut adapter: TraceFrame = serde_json::from_str(I4003_VERILATOR_FRAME.trim()).expect("parse adapter frame");
    adapter.provenance.stimulus_sha256 = behavioral_frame.provenance.stimulus_sha256.clone();
    adapter.provenance.stimulus_kind = behavioral_frame.provenance.stimulus_kind;

    assert_eq!(
        compare_trace_frames(&behavioral_frame, &adapter),
        Err(TraceComparisonError::NoSharedSignals)
    );
}

#[test]
fn verilator_system_adapter_frame_exposes_mapped_mcs4_signals() {
    let frame: TraceFrame =
        serde_json::from_str(MCS4_SYSTEM_VERILATOR_FRAME.trim()).expect("parse system adapter frame");
    frame.validate().expect("validate system adapter frame");

    assert_eq!(frame.provenance.backend, TraceBackend::Verilator);
    assert_eq!(frame.provenance.fidelity, TraceFidelity::Fpga);
    assert_eq!(frame.provenance.model_id, "mcs4-system-fpga-verilator");
    assert!(frame.provenance.stimulus_sha256.is_some());
    assert_eq!(
        frame.signal("mcs4.phase").expect("phase signal").value,
        TraceValue::Bits { width: 3, value: 0 }
    );
    assert_eq!(
        frame.signal("mcs4.cpu.pc").expect("program counter signal").value,
        TraceValue::Bits { width: 12, value: 0 }
    );
}

#[test]
fn system_adapter_rejects_a_behavioral_transcript_even_when_hashes_are_forced_equal() {
    let adapter: TraceFrame =
        serde_json::from_str(MCS4_SYSTEM_VERILATOR_FRAME.trim()).expect("parse system adapter frame");
    let mut behavioral = ReplaySession::<Mcs4System>::new();
    behavioral
        .apply_input(ReplayInput::LoadRom { bytes: vec![0; 4] })
        .expect("load behavioral ROM");
    let mut behavioral_frame = behavioral.step_phase().expect("step behavioral model");
    behavioral_frame.provenance.stimulus_sha256 = adapter.provenance.stimulus_sha256.clone();

    assert_eq!(
        compare_trace_frames(&behavioral_frame, &adapter),
        Err(TraceComparisonError::StimulusKindMismatch {
            left: TraceStimulusKind::ReplayInputTranscript,
            right: TraceStimulusKind::ScenarioJson,
        })
    );
}
