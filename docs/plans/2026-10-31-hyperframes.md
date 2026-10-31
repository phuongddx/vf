# vf HyperFrames Engine Implementation Plan (B3, Delegate Mode)

> **For agentic workers:** REQUIRED SUB-SKILL: Use text2prod:subagent-driven-development (recommended) or text2prod:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `VF_ENGINE=hyperframes vf make <idea>` produces a real MP4 via the HyperFrames CLI — LLM writes the composition HTML directly (delegate), lint gate validates, CI renders a second demo video for the README.

**Architecture:** A compose stage (before the stage loop) writes `index.html` — offline vendored template or one LLM call with a vendored HyperFrames contract — then `npx hyperframes lint` gates it (retry → template fallback), and `HyperframesEngine` renders via `npx hyperframes render --output <ABS>`. The Remotion path is untouched.

**Tech Stack:** Rust 1.85/edition 2024, HyperFrames CLI (`npx hyperframes`), wiremock, assert_cmd, existing vf LLM machinery (providers, RetryingClient, usage).

**Spec:** `docs/specs/2026-10-31-hyperframes-design.md` (same repo)

## Global Constraints

- Local test command: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked` (workdir `vf/`) — no test may require the `tls` feature, npx, or Chrome. `Executor` mocks only.
- HyperFrames facts pinned from docs (2026-10-31): main composition = `index.html`; timed elements carry `data-start` / `data-duration` / `data-track-index` (seconds, 0-based track); fps from root `data-fps` attribute else 30; `npx hyperframes lint [dir]` — no browser, errors block render, warnings don't; `npx hyperframes render --output <file>` renders `index.html` by default (`--composition, -c`); `hyperframes.json` OPTIONAL — a default is written on first use (vendored template does not ship one).
- Render output path must be ABSOLUTE (B2 nested-path lesson) — command cwd is the project dir.
- `VF_LINT=0` disables the lint gate at runtime (tests + no-npx environments).
- `vf make` never dies on compose/lint/render problems — degrade + bilingual warning + exit 0 (render failures break the stage loop; state resumes).
- One commit per task, conventional style.

---

### Task 1: Vendored HyperFrames template

**Files:**
- Create: `src/engine/hf_template.rs`
- Modify: `src/engine/mod.rs` (`pub mod hf_template;`)

**Interfaces:**
- Produces: `pub const TEMPLATE_INDEX_HTML: &str` and `pub fn write_hf_template(project_dir: &Path) -> anyhow::Result<()>` writing `index.html` (minimal valid composition: 1080×1920, `data-fps="30"`, one timed text element).

- [ ] **Step 1: Write the failing test** (`src/engine/hf_template.rs` tests module included in file)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_minimal_valid_composition() {
        let dir = std::env::temp_dir().join(format!("vf-hft-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_hf_template(&dir).unwrap();
        let html = std::fs::read_to_string(dir.join("index.html")).unwrap();
        assert!(html.contains("data-fps=\"30\""));
        assert!(html.contains("data-start=\"0\""));
        assert!(html.contains("data-duration=\"4\""));
        assert!(html.contains("data-track-index=\"0\""));
        assert!(html.contains("<!doctype html>"));
        assert!(!dir.join("hyperframes.json").exists()); // optional per docs
        let _ = std::fs::remove_dir_all(&dir);
    }
}
```

- [ ] **Step 2: RED** — module undefined. Run: `... test --no-default-features --locked hf_template` → E0432/E0425.
- [ ] **Step 3: Implement**

```rust
use std::path::Path;

/// Minimal HyperFrames composition that always renders: 9:16, 4s @ 30fps,
/// one timed text element. `hyperframes.json` is intentionally absent — the
/// CLI writes a default on first use (docs: packages/cli.md, hyperframes.json).
pub const TEMPLATE_INDEX_HTML: &str = r##"<!doctype html>
<html data-fps="30">
<head>
<meta charset="utf-8">
<title>vf</title>
<style>
  html, body { margin: 0; width: 1080px; height: 1920px; background: #0F172A;
               color: #FFFFFF; font-family: Inter, sans-serif; overflow: hidden; }
  .center { display: flex; align-items: center; justify-content: center; height: 100%; }
  h1 { font-size: 96px; text-align: center; margin: 0; }
</style>
</head>
<body>
  <div class="center" data-start="0" data-duration="4" data-track-index="0">
    <h1>vf</h1>
  </div>
</body>
</html>
"##;

pub fn write_hf_template(project_dir: &Path) -> anyhow::Result<()> {
    std::fs::write(project_dir.join("index.html"), TEMPLATE_INDEX_HTML)?;
    Ok(())
}
```

Add `pub mod hf_template;` in `src/engine/mod.rs` next to the other module declarations.

- [ ] **Step 4: GREEN** (83 tests). - [ ] **Step 5: Commit** — `git add -A && git commit -m "feat: vendored hyperframes template composition"`

---

### Task 2: i18n strings for the compose stage

**Files:** Modify `src/i18n.rs`

**Interfaces:** Produces `compose_offline(lang)`, `compose_written(lang)`, `compose_fallback(lang, reason)`. (Manual render hint reuses `render_skipped_manual(lang, cmd)` — it is already parameterized; the spec's `render_hf_manual` is satisfied by passing the hyperframes command. Deviation noted in self-review.)

- [ ] **Step 1: Failing test** (append to `b2_strings_bilingual`):

```rust
        assert!(compose_offline(Lang::Vi).contains("compose offline"));
        assert!(compose_written(Lang::En).contains("composition"));
        assert!(compose_fallback(Lang::Vi, "lint").contains("lint"));
```

- [ ] **Step 2: RED** → **Step 3: Implement** (after `post_skipped`):

```rust
pub fn compose_offline(lang: Lang) -> String {
    match lang {
        Lang::En => "compose offline: no key — using vendored template composition".into(),
        Lang::Vi => "compose offline: chưa có key — dùng composition mẫu có sẵn".into(),
    }
}

pub fn compose_written(lang: Lang) -> String {
    match lang {
        Lang::En => "compose: LLM wrote the hyperframes composition".into(),
        Lang::Vi => "compose: LLM đã viết composition hyperframes".into(),
    }
}

pub fn compose_fallback(lang: Lang, reason: &str) -> String {
    match lang {
        Lang::En => format!("compose: LLM output unusable ({reason}) — using vendored template"),
        Lang::Vi => format!("compose: kết quả LLM không dùng được ({reason}) — dùng composition mẫu"),
    }
}
```

- [ ] **Step 4: GREEN** → **Step 5: Commit** — `git commit -m "feat: bilingual compose-stage strings"`

---

### Task 3: `extract_html` + `compose_request`

**Files:** Modify `src/cmd/make.rs`

**Interfaces:**
- Produces: `pub fn extract_html(text: &str) -> Option<String>` (strip ```html/``` fences; require `<!doctype html` OR `<html`); `fn compose_request(idea: &str, seconds: u32, feedback: Option<&str>) -> crate::llm::ChatRequest` (vendored contract prompt).

- [ ] **Step 1: Failing tests** (append in make.rs tests):

```rust
    #[test]
    fn extract_html_strips_fences() {
        let doc = "```html\n<!doctype html>\n<html data-fps=\"30\"></html>\n```";
        assert!(super::extract_html(doc).unwrap().contains("<html"));
        assert!(super::extract_html("<html></html>").is_some());
        assert!(super::extract_html("just text <div/>").is_none());
    }

    #[test]
    fn compose_request_contract() {
        let req = super::compose_request("demo idea", 8, None);
        assert!(req.system.as_deref().unwrap_or("").contains("data-start"));
        assert!(req.user.contains("demo idea"));
        assert!(req.user.contains("8"));
        let retry = super::compose_request("demo idea", 8, Some("missing data-duration"));
        assert!(retry.user.contains("missing data-duration"));
    }
```

- [ ] **Step 2: RED** → **Step 3: Implement** (above the tests module):

```rust
pub fn extract_html(text: &str) -> Option<String> {
    let t = text.trim();
    let t = t
        .strip_prefix("```html")
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    let t = t.trim();
    if t.contains("<html") {
        Some(t.to_string())
    } else {
        None
    }
}

fn compose_request(idea: &str, seconds: u32, feedback: Option<&str>) -> crate::llm::ChatRequest {
    let system = "You write HyperFrames compositions. Return ONLY one complete HTML document \
inside a ```html fence. Contract: the root tag is <html data-fps=\"30\">; body is exactly \
1080x1920 px with overflow hidden (9:16 vertical); every timed element carries data-start \
(seconds from film start), data-duration (seconds), and data-track-index (0 unless layered); \
animate with CSS keyframes only (if you use GSAP you MUST include its <script> tag or lint \
fails); total film length = last data-start + data-duration; no external network assets — \
only relative assets/ paths. Plain, readable, high-contrast typography.";
    let mut user = format!(
        "Idea: {idea}\nTotal duration: about {seconds} seconds.\nWrite the composition now."
    );
    if let Some(fb) = feedback {
        user.push_str(&format!("\nYour previous composition failed validation: {fb}. Fix it."));
    }
    crate::llm::ChatRequest { system: Some(system.into()), user }
}
```

- [ ] **Step 4: GREEN** → **Step 5: Commit** — `git commit -m "feat: html extraction and hyperframes compose prompt"`

---

### Task 4: Compose stage — `ComposeOutcome` + `compose_hf_stage`

**Files:** Modify `src/cmd/make.rs`

**Interfaces:**
- Consumes: `write_hf_template`, `compose_request`, `extract_html`, provider machinery, `Executor`.
- Produces: `pub enum ComposeOutcome { Skipped, Offline, Composed { usage: Option<crate::llm::Usage> }, Fallback { reason: String } }`; `pub async fn compose_hf_stage(cfg: &crate::config::Config, idea: &str, project_dir: &std::path::Path, exec: &dyn crate::exec::Executor) -> ComposeOutcome`. Lint gate: `npx hyperframes lint` (cwd=project) exit 0 → pass; nonzero → one retry with stderr feedback → replace with vendored template. `VF_LINT=0` disables (light check only: `extract_html` already gates `<html`).

- [ ] **Step 1: Failing unit tests** (append in make.rs tests):

```rust
    #[tokio::test]
    async fn compose_hf_offline_writes_template() -> anyhow::Result<()> {
        let dir = temp::post_dir();
        let cfg = crate::config::Config::builtin();
        let exec = crate::exec::TokioExecutor::new();
        let out = super::compose_hf_stage(&cfg, "offline hf", &dir, &exec).await;
        assert!(matches!(out, super::ComposeOutcome::Offline));
        assert!(dir.join("index.html").exists());
        let html = std::fs::read_to_string(dir.join("index.html"))?;
        assert!(html.contains("data-fps"));
        std::fs::remove_dir_all(&dir).ok();
        Ok(())
    }

    #[tokio::test]
    async fn compose_hf_idempotent_when_index_exists() {
        let dir = temp::post_dir();
        std::fs::write(dir.join("index.html"), "<html>existing</html>").unwrap();
        let cfg = crate::config::Config::builtin();
        let exec = crate::exec::TokioExecutor::new();
        let out = super::compose_hf_stage(&cfg, "x", &dir, &exec).await;
        assert!(matches!(out, super::ComposeOutcome::Skipped));
        assert_eq!(std::fs::read_to_string(dir.join("index.html")).unwrap(), "<html>existing</html>");
        std::fs::remove_dir_all(&dir).ok();
    }
```

- [ ] **Step 2: RED** → **Step 3: Implement**:

```rust
pub enum ComposeOutcome {
    Skipped,
    Offline,
    Composed { usage: Option<crate::llm::Usage> },
    Fallback { reason: String },
}

async fn lint_passes(exec: &dyn crate::exec::Executor, project_dir: &std::path::Path) -> bool {
    if std::env::var("VF_LINT").map(|v| v == "0").unwrap_or(false) {
        return true;
    }
    let cmd = crate::exec::CommandSpec {
        program: "npx".into(),
        args: vec!["hyperframes".into(), "lint".into()],
        cwd: Some(project_dir.to_path_buf()),
    };
    match exec.run(&cmd).await {
        Ok(out) => out.status == 0,
        Err(_) => true, // no npx — light check only (extract_html gated <html)
    }
}

pub async fn compose_hf_stage(
    cfg: &crate::config::Config,
    idea: &str,
    project_dir: &std::path::Path,
    exec: &dyn crate::exec::Executor,
) -> ComposeOutcome {
    use crate::llm::LlmClient as _;
    if project_dir.join("index.html").exists() {
        return ComposeOutcome::Skipped;
    }
    let Some(provider_name) = cfg.routing.spec.clone() else {
        crate::engine::hf_template::write_hf_template(project_dir).ok();
        return ComposeOutcome::Offline;
    };
    let Some(p) = cfg.providers.get(&provider_name) else {
        crate::engine::hf_template::write_hf_template(project_dir).ok();
        return ComposeOutcome::Offline;
    };
    let key_missing = std::env::var(&p.api_key_env).map(|v| v.is_empty()).unwrap_or(true);
    if key_missing || crate::llm::check_tls(&p.base_url).is_err() {
        crate::engine::hf_template::write_hf_template(project_dir).ok();
        return ComposeOutcome::Offline;
    }
    let Ok(inner) = crate::llm::registry::build(&provider_name, cfg) else {
        crate::engine::hf_template::write_hf_template(project_dir).ok();
        return ComposeOutcome::Offline;
    };
    let client = crate::llm::retry::RetryingClient::new(inner);
    let mut feedback: Option<String> = None;
    for _attempt in 0..2 {
        let req = compose_request(idea, 8, feedback.as_deref());
        match client.complete(&req).await {
            Err(e) => {
                let reason = crate::i18n::llm_error(crate::i18n::Lang::En, &e, &p.api_key_env);
                crate::engine::hf_template::write_hf_template(project_dir).ok();
                return ComposeOutcome::Fallback { reason };
            }
            Ok(resp) => {
                if let Some(doc) = extract_html(&resp.text) {
                    std::fs::write(project_dir.join("index.html"), &doc).ok();
                    if lint_passes(exec, project_dir).await {
                        return ComposeOutcome::Composed { usage: resp.usage };
                    }
                    feedback = Some("hyperframes lint reported errors".into());
                } else {
                    feedback = Some("output was not an HTML document".into());
                }
            }
        }
    }
    crate::engine::hf_template::write_hf_template(project_dir).ok();
    ComposeOutcome::Fallback { reason: feedback.unwrap_or_else(|| "invalid composition".into()) }
}
```

- [ ] **Step 4: GREEN** (87 tests). - [ ] **Step 5: Commit** — `git commit -m "feat: hyperframes compose stage with lint gate and template fallback"`

---

### Task 5: `HyperframesEngine` + registration

**Files:**
- Create: `src/engine/hyperframes.rs`
- Modify: `src/engine/mod.rs` (module + `build_engine` arm)

**Interfaces:**
- Consumes: `Engine` trait, `Executor`.
- Produces: `HyperframesEngine { exec: Box<dyn Executor> }`, `new(exec)`, `render_command(project_dir, out) -> CommandSpec` = `npx hyperframes render --output <ABS>` (cwd=project; out absolutized via `std::env::current_dir()` when relative). `Engine::render` ignores the spec (delegate mode — HTML is the artifact).

- [ ] **Step 1: Failing tests** (`src/engine/hyperframes.rs`, mirrors remotion.rs Recorder pattern):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Engine;
    use crate::exec::testing::MockExecutor;
    use crate::exec::RunOutput;
    use std::sync::Arc;

    struct Recorder { inner: Arc<MockExecutor> }

    #[async_trait::async_trait]
    impl crate::exec::Executor for Recorder {
        async fn run(&self, spec: &crate::exec::CommandSpec) -> anyhow::Result<RunOutput> {
            self.inner.run(spec).await
        }
    }

    fn spec() -> crate::ir::VideoSpec { crate::ir::VideoSpec::demo("hf") }

    #[tokio::test]
    async fn renders_via_npx_with_absolute_output() -> anyhow::Result<()> {
        let dir = std::env::temp_dir().join(format!("vf-hfe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        let mock = Arc::new(MockExecutor::with_outputs(vec![RunOutput {
            status: 0, stdout: String::new(), stderr: String::new(),
        }]));
        let engine = HyperframesEngine::new(Box::new(Recorder { inner: mock.clone() }));
        let rel_out = std::path::Path::new("rel-hf.mp4");
        engine.render(&spec(), &dir, rel_out).await?;
        let calls = mock.calls.lock().unwrap();
        assert_eq!(calls[0].program, "npx");
        assert!(calls[0].args.contains(&"hyperframes".to_string()));
        assert!(calls[0].args.contains(&"render".to_string()));
        let out_arg = calls[0].args.last().unwrap();
        assert!(out_arg.starts_with('/'), "out must be absolute, got {out_arg}");
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[tokio::test]
    async fn nonzero_exit_is_error() {
        let dir = std::env::temp_dir().join(format!("vf-hfef-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let engine = HyperframesEngine::new(Box::new(MockExecutor::with_outputs(vec![RunOutput {
            status: 1, stdout: String::new(), stderr: "boom".into(),
        }])));
        assert!(engine.render(&spec(), &dir, &dir.join("x.mp4")).await.is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
```

- [ ] **Step 2: RED** → **Step 3: Implement**

```rust
use super::Engine;
use crate::exec::{CommandSpec, Executor};
use crate::ir::VideoSpec;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

pub struct HyperframesEngine {
    exec: Box<dyn Executor>,
}

impl HyperframesEngine {
    pub fn new(exec: Box<dyn Executor>) -> Self {
        Self { exec }
    }

    pub fn render_command(project_dir: &Path, out: &Path) -> CommandSpec {
        CommandSpec {
            program: "npx".into(),
            args: vec![
                "hyperframes".into(),
                "render".into(),
                "--output".into(),
                out.to_string_lossy().into_owned(),
            ],
            cwd: Some(project_dir.to_path_buf()),
        }
    }
}

#[async_trait]
impl Engine for HyperframesEngine {
    fn name(&self) -> &'static str {
        "hyperframes"
    }

    async fn render(
        &self,
        _spec: &VideoSpec, // delegate mode: index.html is the artifact; spec unused
        project_dir: &Path,
        out: &Path,
    ) -> anyhow::Result<PathBuf> {
        let out_abs = if out.is_absolute() {
            out.to_path_buf()
        } else {
            std::env::current_dir()?.join(out)
        };
        let cmd = Self::render_command(project_dir, &out_abs);
        let res = self.exec.run(&cmd).await?;
        if res.status != 0 {
            anyhow::bail!("hyperframes render failed (exit {}): {}", res.status, res.stderr);
        }
        Ok(out_abs)
    }
}
```

In `src/engine/mod.rs`: add `pub mod hyperframes;` and a `build_engine` arm:

```rust
        "hyperframes" => Ok(Box::new(crate::engine::hyperframes::HyperframesEngine::new(
            Box::new(crate::exec::TokioExecutor::new()),
        )))),
```

Update the unknown-engine error message to include 'hyperframes'.

- [ ] **Step 4: GREEN** (89 tests). - [ ] **Step 5: Commit** — `git commit -m "feat: hyperframes engine rendering via npx with absolute output"`

---

### Task 6: make wiring — compose stage + engine branch

**Files:** Modify `src/cmd/make.rs`

**Interfaces:**
- Consumes: Tasks 1–5.
- Produces: in `run()` — read `VF_ENGINE` once; when `hyperframes`: run `compose_hf_stage` before the stage loop (print `compose_offline` / `compose_written` / `compose_fallback` + reuse the combined-usage line with the compose usage in the codegen column); skip the spec-LLM and codegen blocks. In `run_render`: branch — hyperframes skips spec-file load + Remotion scaffold/registry, passes `VideoSpec::demo("hyperframes")` to the engine, and the toolchain pre-check/manual-hint block covers it (`engine_name != "dry-run"`), with the manual command naming `npx hyperframes render --output draft.mp4`.

- [ ] **Step 1: Failing integration test** (tests/cli.rs):

```rust
#[test]
fn make_hyperframes_offline_composes_template() {
    let dir = temp_dir_for("make-hfoff");
    let cfg_path = dir.join("config.toml");
    fs::write(&cfg_path, "# defaults only\n").unwrap();
    Command::cargo_bin("vf").unwrap()
        .arg("make").arg("hf offline idea")
        .current_dir(&dir)
        .env("VF_CONFIG", &cfg_path)
        .env("VF_ENGINE", "hyperframes")
        .env("VF_LANG", "vi")
        .env("ZAI_CODING_API_KEY", "")
        .env("VF_LINT", "0")
        .assert()
        .success()
        .stdout(predicates::str::contains("compose offline"));
    let html = fs::read_to_string(dir.join("vf-projects/hf-offline-idea/index.html")).unwrap();
    assert!(html.contains("data-fps"));
    let _ = fs::remove_dir_all(&dir);
}
```

(On the gateway the render stage hits node/npx → real `npx hyperframes render` would download the CLI — TOO SLOW for tests. This test asserts the compose artifacts; `vf make` is expected to reach the render stage and degrade. To keep the test fast, ALSO set `VF_RENDER_SKIP=1` — a new test-only knob below.)

- [ ] **Step 2: RED** → **Step 3: Implement**

(a) `run_render` gains at the top:

```rust
    if std::env::var("VF_RENDER_SKIP").map(|v| v == "1").unwrap_or(false) {
        return Ok(()); // test knob: compose/lint stages verified, render skipped
    }
```

(b) Branch the existing Remotion-specific body:

```rust
    let spec = if engine_name == "hyperframes" {
        // Delegate mode: index.html is the artifact; the spec is unused.
        crate::ir::VideoSpec::demo("hyperframes")
    } else {
        crate::ir::VideoSpec::from_json_str(&std::fs::read_to_string(&spec_path)?)?
    };
    if engine_name != "hyperframes" {
        crate::engine::scaffold::write_scaffold(project_dir, &spec)?;
        // Scaffold rewrite resets registry.ts — re-register existing custom scenes.
        crate::engine::scaffold::rebuild_registry(project_dir)?;
    }
```

(c) Pre-check block: change `if engine_name == "remotion" {` to `if engine_name != "dry-run" {` and pick the manual command per engine:

```rust
        let manual = if engine_name == "hyperframes" {
            "npx hyperframes render --output draft.mp4"
        } else {
            "npm install && npx remotion render src/index.tsx vf-main draft.mp4"
        };
```

(use `manual` in both the skip print and the caller's error branch — factor the caller's string to `manual` equivalently.)

(d) In `run()`, before the spec-generation block:

```rust
    let engine_name = std::env::var("VF_ENGINE").unwrap_or_else(|_| "remotion".into());
    if engine_name == "hyperframes" {
        let exec = crate::exec::TokioExecutor::new();
        match compose_hf_stage(&cfg, idea, &project_dir, &exec).await {
            ComposeOutcome::Skipped => {}
            ComposeOutcome::Offline => println!("{}", crate::i18n::compose_offline(lang)),
            ComposeOutcome::Composed { usage } => {
                println!("{}", crate::i18n::compose_written(lang));
                if let Some(u) = usage {
                    let model = cfg.providers.get(cfg.routing.spec.as_deref().unwrap_or("glm-coding"))
                        .map(|p| p.model.clone()).unwrap_or_default();
                    let cost = crate::llm::pricing::estimate_cost(&model, &u);
                    println!("{}", crate::i18n::usage_line(lang, u.input_tokens, u.output_tokens, cost));
                }
            }
            ComposeOutcome::Fallback { reason } => {
                println!("{}", crate::i18n::compose_fallback(lang, &reason));
            }
        }
    } else {
```

…and wrap the existing spec-LLM + codegen blocks in this `else` (the Remotion path is unchanged). The existing `run_render` still reads `VF_ENGINE` itself — acceptable duplication; do NOT refactor its signature this task.

- [ ] **Step 4: GREEN** — full suite (91 tests; all existing dry-run tests unaffected). - [ ] **Step 5: Commit** — `git commit -m "feat: make wires hyperframes compose stage and engine branch"`

---

### Task 7: README — second engine + second demo placeholder

**Files:** Modify `README.md`

- [ ] **Step 1:** "## Demo" gains a second centered `<video src="docs/demo-hyperframes.mp4" controls muted width="270"></video>` + caption naming HyperFrames; new "### Đổi engine" subsection after "### Render thật": `VF_ENGINE=hyperframes` (delegate: LLM viết HTML → lint → render; cần node/npx), table of both engines; English mirror sentence. Status bullet: add "✅ **HyperFrames engine** (delegate: LLM viết composition HTML, lint gate, render qua npx)".
- [ ] **Step 2:** Full suite green (docs-only). - [ ] **Step 3: Commit** — `git commit -m "docs: hyperframes engine section and second demo embed"`

---

### Task 8: Demo workflow — hyperframes render on CI

**Files:** Modify `.github/workflows/demo.yml`, `docs/.demo-trigger`

- [ ] **Step 1:** Add a second job `render-hf-demo` (runs after nothing — parallel to the Remotion job): same build steps, then:

```yaml
        env:
          VF_ENGINE: hyperframes
          VF_LANG: vi
          ZAI_CODING_API_KEY: *** secrets.ZAI_CODING_API_KEY }}
        run: |
          mkdir -p "$HOME/hf-run"
          cd "$HOME/hf-run"
          "$GITHUB_WORKSPACE/target/release/vf" make "vf hyperframes demo"
          test -f vf-projects/vf-hyperframes-demo/draft.mp4
      - name: Commit hyperframes demo
        run: |
          cp "$HOME/hf-run/vf-projects/vf-hyperframes-demo/draft.mp4" docs/demo-hyperframes.mp4
          git config user.name "github-actions[bot]"
          git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
          git add docs/demo-hyperframes.mp4
          git commit -m "docs: hyperframes demo video generated by vf (ci render)" || echo "no changes"
          git push
```

- [ ] **Step 2:** Bump `docs/.demo-trigger` (append line), commit both, push — triggers BOTH demo jobs.
- [ ] **Step 3:** Poll the hyperframes job until success (public API, workflow=demo.yml, head_sha filter). On failure: fetch logs (curl -L + extract-zip pattern), fix, re-bump.
- [ ] **Step 4:** `git pull`, verify `docs/demo-hyperframes.mp4` exists and ≤5 MB.
- [ ] **Step 5:** Ledger entry + note the first real E2E of the compose/lint/render chain.

---

### Task 9: Final verification

- [ ] **Step 1:** Full local suite `--no-default-features --locked` — expect 91/91.
- [ ] **Step 2:** CI matrix green for the final head (check-ci-both.sh pattern).
- [ ] **Step 3:** Ledger: PLAN COMPLETE entry with commit list + defect count; memory update; report with links (README with 2 videos, release untouched — v0.4.0 tag is the owner's call after review).

---

## Self-Review (plan author)

- **Spec coverage:** compose (T3/T4), lint gate + fallback (T4), engine + render ABS (T5), wiring + invariants (T6), i18n (T2), template (T1), README (T7), CI demo (T8), verification (T9). D2–D4 honored (IR skipped — `VideoSpec::demo` placeholder; vendored contract; remotion default).
- **Placeholders:** none.
- **Type consistency:** `ComposeOutcome{Skipped, Offline, Composed{usage}, Fallback{reason}}`; `compose_hf_stage(cfg, idea, dir, &dyn Executor)`; `HyperframesEngine::render_command` mirrors Remotion; `VF_LINT=0` / `VF_RENDER_SKIP=1` knobs documented.
- **Spec deviation:** `render_hf_manual` fn replaced by reusing parameterized `render_skipped_manual(lang, cmd)` (Laziness Protocol — same behavior, no duplicate string pair).
- **Known risk:** the vendored template's lint validity is assumed from the documented attribute contract; T8's real CI lint+render is the E2E proof — if lint flags the template, fix from its output (documented in T8 Step 3).
