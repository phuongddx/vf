use assert_cmd::Command;
use std::fs;

#[test]
fn version_works() {
    Command::cargo_bin("vf").unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains("vf 0.1.0"));
}

#[test]
fn config_command_prints_path() {
    Command::cargo_bin("vf").unwrap()
        .arg("config")
        .assert()
        .success()
        .stdout(predicates::str::contains("config.toml"));
}

#[test]
fn make_creates_project_with_state() {
    let tmp = std::env::temp_dir().join(format!("vf-make-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    Command::cargo_bin("vf").unwrap()
        .current_dir(&tmp)
        .arg("make")
        .arg("demo idea")
        .assert()
        .success()
        .stdout(predicates::str::contains("render"));

    let state =
        fs::read_to_string(tmp.join("vf-projects").join("demo-idea").join("state.json")).unwrap();
    assert!(state.contains("\"post\""));
    assert!(state.contains("\"done\""));

    let _ = fs::remove_dir_all(&tmp);
}
