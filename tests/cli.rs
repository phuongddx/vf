use assert_cmd::Command;
use std::fs;

#[test]
fn version_works() {
    Command::cargo_bin("vf").unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains("vf 0.3.0"));
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

#[test]
fn status_shows_stages_after_make() {
    let tmp = std::env::temp_dir().join(format!("vf-stt-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    Command::cargo_bin("vf")
        .unwrap()
        .env("VF_ENGINE", "dry-run")
        .current_dir(&tmp)
        .arg("make")
        .arg("status demo")
        .assert()
        .success();

    Command::cargo_bin("vf")
        .unwrap()
        .current_dir(&tmp)
        .arg("status")
        .arg("status-demo")
        .assert()
        .success()
        .stdout(predicates::str::contains("render: done"))
        .stdout(predicates::str::contains("post: done"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn config_shows_builtin_profiles_and_hints() {
    let dir = std::env::temp_dir().join(format!("vf-cfg-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let cfg_path = dir.join("config.toml");
    fs::write(&cfg_path, "# empty file = built-ins only\n").unwrap();

    Command::cargo_bin("vf")
        .unwrap()
        .arg("config")
        .env("VF_CONFIG", &cfg_path)
        .assert()
        .success()
        .stdout(predicates::str::contains("glm-coding"))
        .stdout(predicates::str::contains("https://api.z.ai/api/anthropic"))
        .stdout(predicates::str::contains("ZAI_CODING_API_KEY"))
        .stdout(predicates::str::contains("glm"))
        .stdout(predicates::str::contains("https://api.z.ai/api/paas/v4"))
        .stdout(predicates::str::contains("export ZAI_CODING_API_KEY"));

    let _ = fs::remove_dir_all(&dir);
}

fn temp_dir_for(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("vf-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

#[tokio::test]
async fn make_uses_llm_when_key_present() -> anyhow::Result<()> {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let spec_json = serde_json::json!({
        "meta": { "title": "LLM Demo", "duration": 4, "fps": 30, "ratio": "9:16", "lang": "vi" },
        "style": { "preset": "mg-07", "palette": ["#0F172A"], "font": "Inter" },
        "scenes": [ { "id": "s1", "duration": 4, "narration": "xin chào" } ]
    })
    .to_string();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "content": [{ "type": "text", "text": format!("```json\n{spec_json}\n```") }],
            "usage": { "input_tokens": 10, "output_tokens": 20 }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let dir = temp_dir_for("make-llm");
    let cfg_path = dir.join("config.toml");
    fs::write(
        &cfg_path,
        format!(
            "[providers.glm-coding]\nprotocol = \"anthropic\"\nbase_url = \"{}\"\napi_key_env = \"VF_TEST_LLM_KEY\"\nmodel = \"glm-5.2\"\n\n[routing]\nspec = \"glm-coding\"\n",
            server.uri()
        ),
    )
    .unwrap();

    Command::cargo_bin("vf")
        .unwrap()
        .arg("make")
        .arg("llm driven idea")
        .current_dir(&dir)
        .env("VF_CONFIG", &cfg_path)
        .env("VF_TEST_LLM_KEY", "dummy")
        .env("VF_ENGINE", "dry-run")
        .env("VF_LANG", "vi")
        .assert()
        .success()
        .stdout(predicates::str::contains("spec 10/20 · codegen 0/0"));

    let spec = fs::read_to_string(dir.join("vf-projects/llm-driven-idea/video.spec.json"))?;
    assert!(spec.contains("LLM Demo"));
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

#[tokio::test]
async fn make_falls_back_to_template_on_garbage() -> anyhow::Result<()> {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "content": [{ "type": "text", "text": "sorry, no json today" }]
        })))
        .expect(2) // initial + one validation-feedback retry
        .mount(&server)
        .await;

    let dir = temp_dir_for("make-fallback");
    let cfg_path = dir.join("config.toml");
    fs::write(
        &cfg_path,
        format!(
            "[providers.glm-coding]\nprotocol = \"anthropic\"\nbase_url = \"{}\"\napi_key_env = \"VF_TEST_LLM_KEY\"\nmodel = \"glm-5.2\"\n\n[routing]\nspec = \"glm-coding\"\n",
            server.uri()
        ),
    )
    .unwrap();

    Command::cargo_bin("vf")
        .unwrap()
        .arg("make")
        .arg("fallback idea")
        .current_dir(&dir)
        .env("VF_CONFIG", &cfg_path)
        .env("VF_TEST_LLM_KEY", "dummy")
        .env("VF_ENGINE", "dry-run")
        .env("VF_LANG", "vi")
        .assert()
        .success()
        .stdout(predicates::str::contains("spec mẫu"));

    assert!(dir.join("vf-projects/fallback-idea/video.spec.json").exists());
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn make_offline_mode_warns_without_key() {
    let dir = temp_dir_for("make-offline");
    let cfg_path = dir.join("config.toml");
    fs::write(&cfg_path, "# defaults only\n").unwrap();

    Command::cargo_bin("vf")
        .unwrap()
        .arg("make")
        .arg("offline idea")
        .current_dir(&dir)
        .env("VF_CONFIG", &cfg_path)
        .env("VF_ENGINE", "dry-run")
        .env("VF_LANG", "vi")
        .env("ZAI_CODING_API_KEY", "")
        .env("ZAI_API_KEY", "")
        .assert()
        .success()
        .stdout(predicates::str::contains("chế độ offline"));

    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn make_codegen_writes_custom_scene() -> anyhow::Result<()> {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let spec_json = serde_json::json!({
        "meta": { "title": "CG", "duration": 4, "fps": 30, "ratio": "9:16", "lang": "vi" },
        "style": { "preset": "mg-07", "palette": ["#0F172A"], "font": "Inter" },
        "scenes": [ { "id": "s1", "duration": 4, "custom": true } ]
    })
    .to_string();
    let tsx = "```tsx\nexport default () => null;\n```";

    let server = MockServer::start().await;
    let spec_resp = serde_json::json!({
        "content": [{ "type": "text", "text": format!("```json\n{spec_json}\n```") }],
        "usage": { "input_tokens": 10, "output_tokens": 20 }
    });
    let code_resp = serde_json::json!({
        "content": [{ "type": "text", "text": tsx.to_string() }],
        "usage": { "input_tokens": 5, "output_tokens": 7 }
    });
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(spec_resp))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(code_resp))
        .mount(&server)
        .await;

    let dir = temp_dir_for("make-cg");
    let cfg_path = dir.join("config.toml");
    fs::write(
        &cfg_path,
        format!(
            "[providers.glm-coding]\nprotocol = \"anthropic\"\nbase_url = \"{}\"\napi_key_env = \"VF_TEST_LLM_KEY\"\nmodel = \"glm-5.2\"\n\n[routing]\nspec = \"glm-coding\"\ncodegen = \"glm-coding\"\n",
            server.uri()
        ),
    )
    .unwrap();

    Command::cargo_bin("vf")
        .unwrap()
        .arg("make")
        .arg("codegen demo")
        .current_dir(&dir)
        .env("VF_CONFIG", &cfg_path)
        .env("VF_TEST_LLM_KEY", "dummy")
        .env("VF_ENGINE", "dry-run")
        .env("VF_LANG", "vi")
        .env("VF_TSC", "0")
        .assert()
        .success()
        .stdout(predicates::str::contains("đã viết 1 component scene custom"))
        .stdout(predicates::str::contains("spec 10/20 · codegen 5/7"));

    let reg = fs::read_to_string(dir.join("vf-projects/codegen-demo/src/scenes/registry.ts"))?;
    assert!(reg.contains("Custom_s1"));
    let custom = fs::read_to_string(dir.join("vf-projects/codegen-demo/src/scenes/Custom_s1.tsx"))?;
    assert!(custom.contains("export default"));
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

#[test]
fn make_offline_codegen_skips_silently() {
    let dir = temp_dir_for("make-cgoff");
    let cfg_path = dir.join("config.toml");
    fs::write(&cfg_path, "# defaults only\n").unwrap();
    Command::cargo_bin("vf")
        .unwrap()
        .arg("make")
        .arg("offline cg")
        .current_dir(&dir)
        .env("VF_CONFIG", &cfg_path)
        .env("VF_ENGINE", "dry-run")
        .env("VF_LANG", "vi")
        .env("ZAI_CODING_API_KEY", "")
        .assert()
        .success()
        .stdout(predicates::str::contains("chế độ offline"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn make_hyperframes_offline_composes_template() {
    let dir = temp_dir_for("make-hfoff");
    let cfg_path = dir.join("config.toml");
    fs::write(&cfg_path, "# defaults only\n").unwrap();
    Command::cargo_bin("vf")
        .unwrap()
        .arg("make")
        .arg("hf offline idea")
        .current_dir(&dir)
        .env("VF_CONFIG", &cfg_path)
        .env("VF_ENGINE", "hyperframes")
        .env("VF_LANG", "vi")
        .env("ZAI_CODING_API_KEY", "")
        .env("VF_LINT", "0")
        .env("VF_RENDER_SKIP", "1")
        .assert()
        .success()
        .stdout(predicates::str::contains("compose offline"));
    let html = fs::read_to_string(dir.join("vf-projects/hf-offline-idea/index.html")).unwrap();
    assert!(html.contains("data-composition-id"));
    let _ = fs::remove_dir_all(&dir);
}
