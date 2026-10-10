use std::{fs, process::Command};
fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sonahop"))
}

#[test]
fn invalid_duration_is_rejected_before_creating_output() {
    for seconds in ["0", "61"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("must-not-exist.wav");
        let output = command()
            .args(["capture", "--seconds", seconds, "--output"])
            .arg(&path)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(!path.exists());
    }
}

#[test]
fn capture_requires_an_explicit_output_file() {
    let output = command().arg("capture").output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--output"));
}

#[test]
fn existing_output_is_preserved_before_device_access() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("existing.wav");
    fs::write(&path, b"keep existing data").unwrap();
    let output = command()
        .args([
            "capture",
            "--device",
            "intentionally-invalid-device",
            "--output",
        ])
        .arg(&path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(fs::read(path).unwrap(), b"keep existing data");
    assert!(String::from_utf8_lossy(&output.stderr).contains("already exists"));
}

#[test]
fn help_identifies_system_playback_and_microphone_direction() {
    let output = command().args(["capture", "--help"]).output().unwrap();
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("system playback"));
    assert!(help.contains("microphone"));
}
