//! Black-box contract checks for the phase-trace command boundary.

use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::{Arc, Barrier},
    thread,
};

use mcs4_system::{Mcs4System, ReplayCheckpoint, ReplaySession, TraceFrame, TRACE_FRAME_SCHEMA_VERSION};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}

fn run_frame_capture(checkpoint_path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mcs4-phase-trace"))
        .args([
            "--architecture",
            "mcs4",
            "--fixture",
            fixture_path("src_wrm_rdm.hex").to_str().expect("fixture path is UTF-8"),
            "--phases",
            "1",
            "--format",
            "frame-jsonl",
            "--checkpoint",
            checkpoint_path.to_str().expect("checkpoint path is UTF-8"),
        ])
        .output()
        .expect("run phase trace command")
}

#[test]
fn frame_jsonl_capture_has_provenance_and_a_restorable_checkpoint() {
    let temporary = tempfile::tempdir().expect("create temporary directory");
    let checkpoint_path = temporary.path().join("mcs4-checkpoint.json");
    let output = Command::new(env!("CARGO_BIN_EXE_mcs4-phase-trace"))
        .args([
            "--architecture",
            "mcs4",
            "--fixture",
            fixture_path("src_wrm_rdm.hex").to_str().expect("fixture path is UTF-8"),
            "--warmup",
            "2",
            "--phases",
            "3",
            "--format",
            "frame-jsonl",
            "--checkpoint",
            checkpoint_path.to_str().expect("checkpoint path is UTF-8"),
        ])
        .output()
        .expect("run phase trace command");

    assert!(output.status.success(), "phase trace must succeed: {:?}", output.status);
    let stdout = String::from_utf8(output.stdout).expect("phase trace stdout is UTF-8");
    let frames: Vec<TraceFrame> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("parse trace frame JSONL line"))
        .collect();
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].sequence, 3);
    assert_eq!(frames[2].sequence, 5);
    for frame in &frames {
        assert_eq!(frame.schema_version, TRACE_FRAME_SCHEMA_VERSION);
        assert!(frame.provenance.stimulus_sha256.is_some());
        frame.validate().expect("validate trace frame");
    }

    let checkpoint: ReplayCheckpoint =
        serde_json::from_slice(&std::fs::read(&checkpoint_path).expect("read checkpoint")).expect("parse checkpoint");
    assert_eq!(checkpoint.completed_sequence, 5);
    assert_eq!(checkpoint.expected_frame, frames[2]);
    let restored =
        ReplaySession::<Mcs4System>::restore_from_checkpoint(checkpoint.clone()).expect("restore replay checkpoint");
    assert_eq!(restored.last_frame(), Some(&checkpoint.expected_frame));
}

#[test]
fn phase_json_capture_rejects_checkpoint_path() {
    let temporary = tempfile::tempdir().expect("create temporary directory");
    let checkpoint_path = temporary.path().join("unexpected-checkpoint.json");
    let output = Command::new(env!("CARGO_BIN_EXE_mcs4-phase-trace"))
        .args([
            "--architecture",
            "mcs4",
            "--checkpoint",
            checkpoint_path.to_str().expect("checkpoint path is UTF-8"),
        ])
        .output()
        .expect("run phase trace command");

    assert!(!output.status.success(), "legacy trace must reject checkpoint output");
    assert!(!checkpoint_path.exists(), "legacy trace must not write a checkpoint");
    let stderr = String::from_utf8(output.stderr).expect("phase trace stderr is UTF-8");
    assert!(stderr.contains("--checkpoint requires --format frame-jsonl"));
}

#[test]
fn checkpoint_publication_does_not_replace_an_existing_file() {
    let temporary = tempfile::tempdir().expect("create temporary directory");
    let checkpoint_path = temporary.path().join("existing-checkpoint.json");
    std::fs::write(&checkpoint_path, b"preserve-existing-checkpoint\n").expect("write sentinel checkpoint");

    let output = run_frame_capture(&checkpoint_path);

    assert!(!output.status.success(), "existing checkpoint must reject publication");
    assert_eq!(
        std::fs::read(&checkpoint_path).expect("read sentinel checkpoint"),
        b"preserve-existing-checkpoint\n"
    );
}

#[test]
fn concurrent_checkpoint_publication_has_exactly_one_winner() {
    let temporary = tempfile::tempdir().expect("create temporary directory");
    let checkpoint_path = temporary.path().join("shared-checkpoint.json");
    let barrier = Arc::new(Barrier::new(2));
    let first_path = checkpoint_path.clone();
    let first_barrier = Arc::clone(&barrier);
    let first = thread::spawn(move || {
        first_barrier.wait();
        run_frame_capture(&first_path)
    });
    let second_path = checkpoint_path.clone();
    let second_barrier = Arc::clone(&barrier);
    let second = thread::spawn(move || {
        second_barrier.wait();
        run_frame_capture(&second_path)
    });

    let outputs = [
        first.join().expect("join first capture"),
        second.join().expect("join second capture"),
    ];
    assert_eq!(outputs.iter().filter(|output| output.status.success()).count(), 1);
    let checkpoint: ReplayCheckpoint =
        serde_json::from_slice(&std::fs::read(&checkpoint_path).expect("read published checkpoint"))
            .expect("parse atomically published checkpoint");
    assert_eq!(checkpoint.schema_version, mcs4_system::REPLAY_SCHEMA_VERSION);
}
