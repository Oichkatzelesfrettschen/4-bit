//! Deterministic typed Verilog export command.

use std::{
    env,
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use mcs4_fpga::{ChipTarget, ExportFlavor, ExportRequest, VerilogExporter};
use serde_json::json;
use sha2::{Digest, Sha256};

const USAGE: &str =
    "Usage: mcs4-fpga-export --chip <chip> --flavor <behavioral|fpga> --output <path> [--manifest <path>]";

#[derive(Debug)]
struct Arguments {
    chip: ChipTarget,
    flavor: ExportFlavor,
    output: PathBuf,
    manifest: PathBuf,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mcs4-fpga-export: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = parse_arguments(env::args_os().skip(1))?;
    let request = ExportRequest::new(arguments.chip, arguments.flavor);
    let exporter = VerilogExporter;
    let mut rendered = Vec::new();
    exporter
        .export_request(request, &mut rendered)
        .map_err(|error| error.to_string())?;

    let digest = hex_digest(&rendered);
    atomic_write(&arguments.output, &rendered)?;

    let revision = git_output(["rev-parse", "HEAD"])?;
    let dirty = !git_output(["status", "--porcelain=v1"])?.is_empty();
    let manifest = json!({
        "schema_version": 1,
        "generator": "mcs4-fpga-export",
        "generator_version": env!("CARGO_PKG_VERSION"),
        "source_revision": revision,
        "source_dirty": dirty,
        "request": {
            "chip": arguments.chip.as_str(),
            "flavor": arguments.flavor.as_str(),
        },
        "output": arguments.output,
        "output_sha256": digest,
    });
    let manifest_bytes =
        serde_json::to_vec_pretty(&manifest).map_err(|error| format!("serialize export manifest: {error}"))?;
    atomic_write(&arguments.manifest, &manifest_bytes)?;

    println!("Generated: {}", arguments.output.display());
    println!("Manifest: {}", arguments.manifest.display());
    Ok(())
}

fn parse_arguments(arguments: impl Iterator<Item = OsString>) -> Result<Arguments, String> {
    let mut chip = None;
    let mut flavor = None;
    let mut output = None;
    let mut manifest = None;
    let mut iterator = arguments;

    while let Some(flag) = iterator.next() {
        let flag = flag
            .into_string()
            .map_err(|_| "arguments must be valid UTF-8".to_owned())?;
        match flag.as_str() {
            "--chip" => {
                let value = next_argument(&mut iterator, "--chip")?;
                chip = Some(
                    value
                        .parse()
                        .map_err(|error: mcs4_fpga::verilog::ParseChipTargetError| error.to_string())?,
                );
            }
            "--flavor" => {
                let value = next_argument(&mut iterator, "--flavor")?;
                flavor = Some(
                    value
                        .parse()
                        .map_err(|error: mcs4_fpga::verilog::ParseExportFlavorError| error.to_string())?,
                );
            }
            "--output" => output = Some(PathBuf::from(next_argument(&mut iterator, "--output")?)),
            "--manifest" => manifest = Some(PathBuf::from(next_argument(&mut iterator, "--manifest")?)),
            "--help" | "-h" => return Err(USAGE.to_owned()),
            _ => return Err(format!("unknown argument {flag:?}; {USAGE}")),
        }
    }

    let chip = chip.ok_or_else(|| format!("missing --chip; {USAGE}"))?;
    let flavor = flavor.ok_or_else(|| format!("missing --flavor; {USAGE}"))?;
    let output = output.ok_or_else(|| format!("missing --output; {USAGE}"))?;
    let manifest = manifest.unwrap_or_else(|| PathBuf::from(format!("{}.manifest.json", output.display())));

    Ok(Arguments {
        chip,
        flavor,
        output,
        manifest,
    })
}

fn next_argument(arguments: &mut impl Iterator<Item = OsString>, flag: &str) -> Result<String, String> {
    arguments
        .next()
        .ok_or_else(|| format!("missing value for {flag}; {USAGE}"))?
        .into_string()
        .map_err(|_| format!("value for {flag} must be valid UTF-8"))
}

fn git_output<const N: usize>(arguments: [&str; N]) -> Result<String, String> {
    let output = Command::new("git")
        .args(arguments)
        .output()
        .map_err(|error| format!("run git for export provenance: {error}"))?;
    if !output.status.success() {
        return Err(format!("git provenance command failed with {}", output.status));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| format!("git provenance output is not UTF-8: {error}"))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| format!("create {}: {error}", parent.display()))?;

    let file_name = path
        .file_name()
        .ok_or_else(|| format!("output path {} has no filename", path.display()))?;
    let temporary = parent.join(format!(".{}.{}.tmp", file_name.to_string_lossy(), std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("create {}: {error}", temporary.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("write {}: {error}", temporary.display()))?;
    file.sync_all()
        .map_err(|error| format!("sync {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("rename {} to {}: {error}", temporary.display(), path.display()))
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_numeric_chip_argument_and_default_manifest() {
        let arguments = parse_arguments(
            ["--chip", "4003", "--flavor", "fpga", "--output", "target/i4003.v"]
                .into_iter()
                .map(OsString::from),
        )
        .expect("numeric chip input parses");
        assert_eq!(arguments.chip, ChipTarget::I4003);
        assert_eq!(arguments.flavor, ExportFlavor::Fpga);
        assert_eq!(arguments.manifest, PathBuf::from("target/i4003.v.manifest.json"));
    }

    #[test]
    fn rejects_missing_required_arguments() {
        let error = parse_arguments(["--chip", "4003"].into_iter().map(OsString::from))
            .expect_err("flavor and output are required");
        assert!(error.contains("missing --flavor"));
    }
}
