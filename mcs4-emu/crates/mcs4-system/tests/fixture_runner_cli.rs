//! Black-box checks for the fixture-runner command boundary.

use std::{path::PathBuf, process::Command};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}

#[test]
fn fixture_runner_reports_machine_state_for_valid_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_fixture_runner"))
        .arg(fixture_path("src_wrm_rdm.hex"))
        .arg("12")
        .output()
        .expect("run fixture runner");

    assert!(output.status.success(), "fixture runner must succeed");

    let stdout = String::from_utf8(output.stdout).expect("fixture runner stdout is UTF-8");
    assert!(stdout.contains("cycles=12"));
    assert!(stdout.contains("acc=0xa"));
}

#[test]
fn fixture_runner_reports_missing_fixture_without_panic() {
    let missing_fixture = fixture_path("does-not-exist.hex");
    let output = Command::new(env!("CARGO_BIN_EXE_fixture_runner"))
        .arg(&missing_fixture)
        .output()
        .expect("run fixture runner");

    assert!(!output.status.success(), "missing fixture must fail");

    let stderr = String::from_utf8(output.stderr).expect("fixture runner stderr is UTF-8");
    assert!(stderr.contains("failed to load fixture"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn fixture_runner_rejects_malformed_hex_before_execution() {
    let malformed_fixture = fixture_path("malformed_hex.hex");
    let fixture_display = malformed_fixture.display().to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_fixture_runner"))
        .arg(&malformed_fixture)
        .arg("12")
        .output()
        .expect("run fixture runner");

    assert_eq!(
        output.status.code(),
        Some(1),
        "malformed fixture must use the load-error exit code"
    );
    assert!(
        output.stdout.is_empty(),
        "malformed fixture must not execute or report machine state"
    );

    let stderr = String::from_utf8(output.stderr).expect("fixture runner stderr is UTF-8");
    assert!(stderr.contains("failed to load fixture"));
    assert!(stderr.contains(&fixture_display));
    assert!(stderr.contains("fixture parse error on line 2: invalid byte \"GG\""));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn fixture_runner_rejects_rom_larger_than_the_configured_system() {
    let temporary = tempfile::tempdir().expect("create temporary fixture directory");
    let oversized_fixture = temporary.path().join("oversized.hex");
    let bytes = std::iter::repeat_n("00", 257).collect::<Vec<_>>().join(" ");
    std::fs::write(&oversized_fixture, bytes).expect("write oversized fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_fixture_runner"))
        .arg(&oversized_fixture)
        .output()
        .expect("run fixture runner");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("fixture runner stderr is UTF-8");
    assert!(stderr.contains("fixture too large: 257 bytes parsed, max is 256"));
    assert!(!stderr.contains("panicked at"));
}

#[cfg(unix)]
#[test]
fn fixture_runner_rejects_unreadable_fixture_when_the_host_enforces_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let temporary = tempfile::tempdir().expect("create temporary fixture directory");
    let unreadable_fixture = temporary.path().join("unreadable.hex");
    std::fs::write(&unreadable_fixture, "00").expect("write unreadable fixture");
    std::fs::set_permissions(&unreadable_fixture, std::fs::Permissions::from_mode(0o000))
        .expect("remove fixture read permission");
    if std::fs::File::open(&unreadable_fixture).is_ok() {
        eprintln!("skipping unreadable-fixture assertion because this host bypasses file permissions");
        return;
    }

    let output = Command::new(env!("CARGO_BIN_EXE_fixture_runner"))
        .arg(&unreadable_fixture)
        .output()
        .expect("run fixture runner");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("fixture runner stderr is UTF-8");
    assert!(stderr.contains("failed to load fixture"));
    assert!(!stderr.contains("panicked at"));
}
