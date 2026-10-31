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
        .env("VF_ENGINE", "dry-run")
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

#[test]
fn make_dry_run_renders_draft() {
    let tmp = std::env::temp_dir().join(format!("vf-dry-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    Command::cargo_bin("vf").unwrap()
        .env("VF_ENGINE", "dry-run")
        .current_dir(&tmp)
        .arg("make")
        .arg("demo idea")
        .assert()
        .success();

    let dir = tmp.join("vf-projects").join("demo-idea");
    assert_eq!(fs::read(dir.join("draft.mp4")).unwrap(), b"dry-run");
    assert!(dir.join("package.json").exists());
    assert!(dir.join("video.spec.json").exists());
    assert_eq!(fs::read(dir.join("final.mp4")).unwrap(), b"dry-run");

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn skills_install_and_list_from_local_repo() {
    let tmp = std::env::temp_dir().join(format!("vf-ski-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    // Fixture git repo — local clone source, no network.
    let fix = tmp.join("fixture-skill");
    fs::create_dir_all(&fix).unwrap();
    fs::write(fix.join("SKILL.md"), "# fixture\n").unwrap();
    let git = |args: &[&str], cwd: &std::path::Path| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap()
            .status
            .success()
    };
    assert!(git(&["init", "-q", "-b", "main"], &fix));
    assert!(git(&["config", "user.email", "t@t.local"], &fix));
    assert!(git(&["config", "user.name", "t"], &fix));
    assert!(git(&["add", "."], &fix));
    assert!(git(&["commit", "-q", "-m", "init"], &fix));

    let work = tmp.join("work");
    fs::create_dir_all(&work).unwrap();

    Command::cargo_bin("vf")
        .unwrap()
        .env("VF_SKILLS_DIR", work.join(".vf").join("skills"))
        .env("VF_LOCK_PATH", work.join("vf.lock"))
        .current_dir(&work)
        .arg("skills")
        .arg(fix.to_string_lossy().as_ref())
        .assert()
        .success()
        .stdout(predicates::str::contains("fixture-skill"));

    Command::cargo_bin("vf")
        .unwrap()
        .env("VF_SKILLS_DIR", work.join(".vf").join("skills"))
        .env("VF_LOCK_PATH", work.join("vf.lock"))
        .current_dir(&work)
        .arg("skills")
        .assert()
        .success()
        .stdout(predicates::str::contains("fixture-skill"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn make_writes_style_json() {
    let tmp = std::env::temp_dir().join(format!("vf-sty-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    Command::cargo_bin("vf")
        .unwrap()
        .env("VF_ENGINE", "dry-run")
        .env("VF_STYLE", "mg-03")
        .current_dir(&tmp)
        .arg("make")
        .arg("style demo")
        .assert()
        .success();

    let style =
        fs::read_to_string(tmp.join("vf-projects").join("style-demo").join("style.json"))
            .unwrap();
    assert!(style.contains("mg-03"));
    assert!(style.contains("Montserrat"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn make_with_launch_template_shape() {
    let tmp = std::env::temp_dir().join(format!("vf-tpl-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    Command::cargo_bin("vf")
        .unwrap()
        .env("VF_ENGINE", "dry-run")
        .current_dir(&tmp)
        .arg("make")
        .arg("launch demo")
        .arg("--template")
        .arg("launch")
        .assert()
        .success();

    let spec = fs::read_to_string(
        tmp.join("vf-projects")
            .join("launch-demo")
            .join("video.spec.json"),
    )
    .unwrap();
    assert!(spec.contains("\"duration\": 15"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn doctor_reports_tools() {
    Command::cargo_bin("vf")
        .unwrap()
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicates::str::contains("doctor: git OK"))
        .stdout(predicates::str::contains("doctor: ffmpeg MISSING"));
}

#[test]
fn make_vietnamese_output() {
    let tmp = std::env::temp_dir().join(format!("vf-vi-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    Command::cargo_bin("vf")
        .unwrap()
        .env("VF_ENGINE", "dry-run")
        .env("VF_LANG", "vi")
        .current_dir(&tmp)
        .arg("make")
        .arg("vi demo")
        .assert()
        .success()
        .stdout(predicates::str::contains("giai đoạn"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn make_plan_mode_prints_estimate_without_running() {
    let tmp = std::env::temp_dir().join(format!("vf-planm-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    Command::cargo_bin("vf")
        .unwrap()
        .env("VF_ENGINE", "dry-run")
        .current_dir(&tmp)
        .arg("make")
        .arg("plan demo")
        .arg("--plan")
        .assert()
        .success()
        .stdout(predicates::str::contains("estimate:"))
        .stdout(predicates::str::contains("plan: render"));
    assert!(!tmp.join("vf-projects").exists());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn make_max_cost_zero_bails() {
    let tmp = std::env::temp_dir().join(format!("vf-maxc-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    Command::cargo_bin("vf")
        .unwrap()
        .current_dir(&tmp)
        .arg("make")
        .arg("x")
        .arg("--max-cost")
        .arg("0")
        .assert()
        .failure()
        .stderr(predicates::str::contains("exceeds --max-cost"));

    let _ = fs::remove_dir_all(&tmp);
}
