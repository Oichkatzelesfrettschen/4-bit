//! Emit deterministic post-phase traces for a minimal MCS-4-family system.

use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::ExitCode,
};

use mcs4_system::{
    fixture::load_hex_bytes_bounded, Mcs40System, Mcs4System, PhaseTrace, ReplayCheckpoint, ReplayInput, ReplaySession,
    TraceFrame, TraceReplayTarget,
};

const USAGE: &str = "Usage: mcs4-phase-trace --architecture <mcs4|mcs40> [--fixture <hex-path>] [--warmup <non-negative-integer>] [--phases <positive-integer>] [--format <phase-json|frame-jsonl>] [--checkpoint <json-path>]";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    PhaseJson,
    FrameJsonl,
}

struct Arguments {
    architecture: String,
    fixture: Option<PathBuf>,
    warmup: usize,
    phases: usize,
    output_format: OutputFormat,
    checkpoint: Option<PathBuf>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mcs4-phase-trace: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = parse_arguments()?;
    if arguments.output_format == OutputFormat::PhaseJson && arguments.checkpoint.is_some() {
        return Err(format!("--checkpoint requires --format frame-jsonl; {USAGE}"));
    }

    match arguments.output_format {
        OutputFormat::PhaseJson => emit_phase_trace(&arguments),
        OutputFormat::FrameJsonl => emit_frame_trace(&arguments),
    }
}

fn emit_phase_trace(arguments: &Arguments) -> Result<(), String> {
    let trace = match arguments.architecture.as_str() {
        "mcs4" => {
            let mut system = Mcs4System::minimal();
            load_program(&mut system, arguments.fixture.as_deref())?;
            for _ in 0..arguments.warmup {
                system.step();
            }
            (0..arguments.phases).map(|_| system.step_traced()).collect::<Vec<_>>()
        }
        "mcs40" => {
            let mut system = Mcs40System::minimal();
            load_program_mcs40(&mut system, arguments.fixture.as_deref())?;
            for _ in 0..arguments.warmup {
                system.step();
            }
            (0..arguments.phases).map(|_| system.step_traced()).collect::<Vec<_>>()
        }
        _ => {
            return Err(format!(
                "unsupported architecture {:?}; {USAGE}",
                arguments.architecture
            ))
        }
    };
    print_trace(&trace)
}

fn emit_frame_trace(arguments: &Arguments) -> Result<(), String> {
    let program = load_program_bytes(arguments.fixture.as_deref())?;
    let (frames, checkpoint) = match arguments.architecture.as_str() {
        "mcs4" => capture_frames::<Mcs4System>(&program, arguments.warmup, arguments.phases)?,
        "mcs40" => capture_frames::<Mcs40System>(&program, arguments.warmup, arguments.phases)?,
        _ => {
            return Err(format!(
                "unsupported architecture {:?}; {USAGE}",
                arguments.architecture
            ))
        }
    };

    print_frame_trace(&frames)?;
    if let Some(path) = arguments.checkpoint.as_deref() {
        write_checkpoint(path, &checkpoint)?;
    }
    Ok(())
}

fn capture_frames<T: TraceReplayTarget>(
    program: &[u8],
    warmup: usize,
    phases: usize,
) -> Result<(Vec<TraceFrame>, ReplayCheckpoint), String> {
    let mut session = ReplaySession::<T>::new();
    session
        .apply_input(ReplayInput::LoadRom {
            bytes: program.to_vec(),
        })
        .map_err(|error| format!("load replay ROM: {error}"))?;
    for _ in 0..warmup {
        let _ = session
            .step_phase()
            .map_err(|error| format!("warm trace frame: {error}"))?;
    }
    let frames = (0..phases)
        .map(|_| {
            session
                .step_phase()
                .map_err(|error| format!("capture trace frame: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let checkpoint = session
        .checkpoint()
        .map_err(|error| format!("checkpoint trace capture: {error}"))?;
    Ok((frames, checkpoint))
}

fn load_program(system: &mut Mcs4System, fixture: Option<&Path>) -> Result<(), String> {
    if let Some(path) = fixture {
        system
            .load_rom_hex_file(path)
            .map_err(|error| format!("load {}: {error}", path.display()))?;
    } else {
        system.load_rom(&[0x00]);
    }
    Ok(())
}

fn load_program_mcs40(system: &mut Mcs40System, fixture: Option<&Path>) -> Result<(), String> {
    if let Some(path) = fixture {
        system
            .load_rom_hex_file(path)
            .map_err(|error| format!("load {}: {error}", path.display()))?;
    } else {
        system.load_rom(&[0x00]);
    }
    Ok(())
}

fn load_program_bytes(fixture: Option<&Path>) -> Result<Vec<u8>, String> {
    match fixture {
        Some(path) => load_hex_bytes_bounded(path, 256).map_err(|error| format!("load {}: {error}", path.display())),
        None => Ok(vec![0x00]),
    }
}

fn parse_arguments() -> Result<Arguments, String> {
    let mut architecture = None;
    let mut fixture = None;
    let mut warmup = 0usize;
    let mut phases = 8usize;
    let mut output_format = OutputFormat::PhaseJson;
    let mut checkpoint = None;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--architecture" => {
                architecture = Some(
                    arguments
                        .next()
                        .ok_or_else(|| format!("missing value for --architecture; {USAGE}"))?,
                );
            }
            "--phases" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| format!("missing value for --phases; {USAGE}"))?;
                phases = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid --phases value {value:?}; {USAGE}"))?;
            }
            "--warmup" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| format!("missing value for --warmup; {USAGE}"))?;
                warmup = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid --warmup value {value:?}; {USAGE}"))?;
            }
            "--fixture" => {
                fixture = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| format!("missing value for --fixture; {USAGE}"))?,
                ));
            }
            "--format" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| format!("missing value for --format; {USAGE}"))?;
                output_format = match value.as_str() {
                    "phase-json" => OutputFormat::PhaseJson,
                    "frame-jsonl" => OutputFormat::FrameJsonl,
                    _ => return Err(format!("unsupported --format value {value:?}; {USAGE}")),
                };
            }
            "--checkpoint" => {
                checkpoint = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| format!("missing value for --checkpoint; {USAGE}"))?,
                ));
            }
            "--help" | "-h" => return Err(USAGE.to_owned()),
            _ => return Err(format!("unknown argument {argument:?}; {USAGE}")),
        }
    }
    if phases == 0 {
        return Err(format!("--phases must be positive; {USAGE}"));
    }
    Ok(Arguments {
        architecture: architecture.ok_or_else(|| format!("missing --architecture; {USAGE}"))?,
        fixture,
        warmup,
        phases,
        output_format,
        checkpoint,
    })
}

fn print_trace(trace: &[PhaseTrace]) -> Result<(), String> {
    serde_json::to_writer_pretty(std::io::stdout(), trace)
        .map_err(|error| format!("serialize phase trace: {error}"))?;
    println!();
    Ok(())
}

fn print_frame_trace(trace: &[TraceFrame]) -> Result<(), String> {
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    for frame in trace {
        serde_json::to_writer(&mut output, frame).map_err(|error| format!("serialize trace frame: {error}"))?;
        writeln!(output).map_err(|error| format!("write trace frame: {error}"))?;
    }
    Ok(())
}

fn write_checkpoint(path: &Path, checkpoint: &ReplayCheckpoint) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let filename = path
        .file_name()
        .ok_or_else(|| format!("checkpoint path has no filename: {}", path.display()))?;
    let temporary = parent.join(format!(
        ".{}-{}.partial",
        filename.to_string_lossy(),
        std::process::id()
    ));
    if temporary.exists() {
        return Err(format!(
            "checkpoint temporary path already exists: {}",
            temporary.display()
        ));
    }

    let result = (|| -> Result<(), String> {
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| format!("create checkpoint temporary file {}: {error}", temporary.display()))?;
        serde_json::to_writer_pretty(&mut output, checkpoint)
            .map_err(|error| format!("serialize checkpoint: {error}"))?;
        writeln!(output).map_err(|error| format!("write checkpoint: {error}"))?;
        output
            .sync_all()
            .map_err(|error| format!("sync checkpoint temporary file {}: {error}", temporary.display()))?;
        drop(output);
        fs::hard_link(&temporary, path)
            .map_err(|error| format!("publish checkpoint without replacement {}: {error}", path.display()))?;
        fs::remove_file(&temporary)
            .map_err(|error| format!("remove checkpoint temporary file {}: {error}", temporary.display()))?;
        sync_parent_directory(parent)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(unix)]
fn sync_parent_directory(path: &Path) -> Result<(), String> {
    std::fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("sync checkpoint parent directory {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn sync_parent_directory(_path: &Path) -> Result<(), String> {
    Ok(())
}
