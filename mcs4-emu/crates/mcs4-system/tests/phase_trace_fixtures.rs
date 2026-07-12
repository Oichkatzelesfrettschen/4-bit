//! Deterministic MCS-4 and MCS-40 post-phase regression fixtures.

use std::{fs, path::Path};

use mcs4_system::{Mcs40System, Mcs4System, PhaseTrace, SystemArchitecture, TracePhase};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TraceFixture {
    schema_version: u32,
    architecture: SystemArchitecture,
    program_fixture: String,
    warmup_phases: usize,
    samples: Vec<TraceSample>,
}

#[derive(Debug, Deserialize)]
struct TraceSample {
    completed_phase: TracePhase,
    next_phase: TracePhase,
    machine_cycles: u64,
    instruction_count: u64,
    pc: u16,
    accumulator: u8,
    bus_value: u8,
    selected_ram: Option<u8>,
    io_op: Option<String>,
}

fn fixture_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/traces").join(name)
}

fn load_fixture(name: &str) -> TraceFixture {
    serde_json::from_slice(&fs::read(fixture_path(name)).expect("read phase trace fixture"))
        .expect("parse phase trace fixture")
}

fn expected_trace(architecture: SystemArchitecture, sample: &TraceSample) -> PhaseTrace {
    PhaseTrace {
        schema_version: 1,
        architecture,
        completed_phase: sample.completed_phase,
        next_phase: sample.next_phase,
        machine_cycles: sample.machine_cycles,
        instruction_count: sample.instruction_count,
        pc: sample.pc,
        accumulator: sample.accumulator,
        carry: false,
        bus_value: sample.bus_value,
        bus_valid: true,
        bus_contention: false,
        selected_rom: Some(0),
        selected_ram: sample.selected_ram,
        io_op: sample.io_op.clone(),
    }
}

#[test]
fn mcs4_src_wrm_rdm_phase_trace_matches_fixture() {
    let fixture = load_fixture("mcs4-src-wrm-rdm-v1.json");
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.architecture, SystemArchitecture::Mcs4);

    let mut system = Mcs4System::minimal();
    let program = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(&fixture.program_fixture);
    system.load_rom_hex_file(program).expect("load MCS-4 trace ROM");
    for _ in 0..fixture.warmup_phases {
        system.step();
    }
    let actual: Vec<_> = fixture
        .samples
        .iter()
        .map(|_| {
            let sample = system.step_traced();
            assert_eq!(system.phase(), system.cpu.cycle_state().phase);
            assert_eq!(system.cycles(), system.cpu.cycle_state().cycle_count);
            sample
        })
        .collect();
    let expected: Vec<_> = fixture
        .samples
        .iter()
        .map(|sample| expected_trace(SystemArchitecture::Mcs4, sample))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn mcs40_src_wrm_rdm_phase_trace_matches_fixture() {
    let fixture = load_fixture("mcs40-src-wrm-rdm-v1.json");
    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.architecture, SystemArchitecture::Mcs40);

    let mut system = Mcs40System::minimal();
    let program = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(&fixture.program_fixture);
    system.load_rom_hex_file(program).expect("load MCS-40 trace ROM");
    for _ in 0..fixture.warmup_phases {
        system.step();
    }
    let actual: Vec<_> = fixture
        .samples
        .iter()
        .map(|_| {
            let sample = system.step_traced();
            assert_eq!(system.phase(), system.cpu.cycle_state().phase);
            assert_eq!(system.total_cycles, system.cpu.cycle_state().cycle_count);
            sample
        })
        .collect();
    let expected: Vec<_> = fixture
        .samples
        .iter()
        .map(|sample| expected_trace(SystemArchitecture::Mcs40, sample))
        .collect();
    assert_eq!(actual, expected);
}
