//! Execute the common MCS-4 stimulus through the behavioral phase model.

use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

use mcs4_system::{CommonStimulus, CommonStimulusAction, Mcs4System, ReplayInput, ReplaySession, TraceStimulusKind};
use sha2::{Digest, Sha256};

const MAXIMUM_STIMULUS_BYTES: u64 = 8 * 1024 * 1024;
const USAGE: &str = "Usage: mcs4-common-stimulus --stimulus <json-path>";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mcs4-common-stimulus: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let path = parse_path()?;
    let metadata = fs::metadata(&path).map_err(|error| format!("stat {}: {error}", path.display()))?;
    if metadata.len() > MAXIMUM_STIMULUS_BYTES {
        return Err("common stimulus JSON exceeds the byte safety limit".to_owned());
    }
    let bytes = fs::read(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let stimulus = CommonStimulus::parse(&bytes)?;
    let stimulus_sha256 = format!("{:x}", Sha256::digest(&bytes));
    let mut session = ReplaySession::<Mcs4System>::new();
    session
        .apply_input(ReplayInput::LoadRom {
            bytes: stimulus.rom_bytes()?,
        })
        .map_err(|error| format!("load common stimulus ROM: {error}"))?;

    let stdout = io::stdout();
    let mut output = stdout.lock();
    for (index, action) in stimulus.actions.iter().enumerate() {
        let input_event_id = u64::try_from(index + 1).map_err(|_| "common stimulus action index overflows u64")?;
        match action {
            CommonStimulusAction::Reset => {
                session
                    .apply_input(ReplayInput::Reset)
                    .map_err(|error| format!("apply reset: {error}"))?;
            }
            CommonStimulusAction::SetTest { value } => {
                session
                    .apply_input(ReplayInput::SetTestPin { high: *value })
                    .map_err(|error| format!("apply TEST input: {error}"))?;
            }
            CommonStimulusAction::RunPhases { value } => {
                for _ in 0..*value {
                    let mut frame = session
                        .step_phase()
                        .map_err(|error| format!("step common stimulus phase: {error}"))?;
                    frame.input_event_id = input_event_id;
                    frame.provenance.stimulus_sha256 = Some(stimulus_sha256.clone());
                    frame.provenance.stimulus_kind = Some(TraceStimulusKind::ScenarioJson);
                    serde_json::to_writer(&mut output, &frame)
                        .map_err(|error| format!("serialize common behavioral frame: {error}"))?;
                    writeln!(output).map_err(|error| format!("write common behavioral frame: {error}"))?;
                }
            }
        }
    }
    Ok(())
}

fn parse_path() -> Result<PathBuf, String> {
    let mut arguments = env::args().skip(1);
    let Some(option) = arguments.next() else {
        return Err(USAGE.to_owned());
    };
    if option == "--help" || option == "-h" {
        return Err(USAGE.to_owned());
    }
    if option != "--stimulus" {
        return Err(format!("unknown argument {option:?}; {USAGE}"));
    }
    let path = arguments
        .next()
        .ok_or_else(|| format!("missing value for --stimulus; {USAGE}"))?;
    if arguments.next().is_some() {
        return Err(format!("too many arguments; {USAGE}"));
    }
    Ok(PathBuf::from(path))
}
