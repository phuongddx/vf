use assert_cmd::Command;

#[test]
fn version_works() {
    Command::cargo_bin("vf").unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains("vf 0.1.0"));
}
