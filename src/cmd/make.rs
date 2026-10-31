use crate::exec::Executor;
use crate::state::State;

pub async fn run(
    idea: &str,
    template: Option<&str>,
    plan: bool,
    max_cost: Option<f64>,
) -> anyhow::Result<()> {
    let t = crate::templates::Template::from_str(template.unwrap_or("default"))?;
    let calls = estimate_calls(&t);
    let est = calls as f64 * 0.002;
    if let Some(max) = max_cost {
        if est > max {
            anyhow::bail!("estimated cost ${est:.3} exceeds --max-cost ${max:.3}");
        }
    }
    if plan {
        for s in crate::state::STAGES {
            println!("plan: {s}");
        }
        println!("estimate: {calls} LLM calls (~${est:.3})");
        let plan_cfg = crate::config::Config::load()?.effective();
        let target = plan_cfg.routing.spec.clone().unwrap_or_default();
        let has_key = plan_cfg
            .providers
            .get(&target)
            .map(|p| std::env::var(&p.api_key_env).map(|v| !v.is_empty()).unwrap_or(false))
            .unwrap_or(false);
        println!("provider: {target} ({})", if has_key { "llm" } else { "offline" });
        return Ok(());
    }
    let lang = crate::i18n::Lang::from_env();
    let slug = slugify(idea);
    let project_dir = std::path::Path::new("vf-projects").join(&slug);
    std::fs::create_dir_all(&project_dir)?;

    let state_path = project_dir.join("state.json");
    let mut state = if state_path.exists() {
        State::load(&state_path)?
    } else {
        State::new(&slug)
    };

    let cfg = crate::config::Config::load()?.effective();
    let engine_name = std::env::var("VF_ENGINE").unwrap_or_else(|_| "remotion".into());
    if engine_name == "hyperframes" {
        let exec = crate::exec::TokioExecutor::new();
        match compose_hf_stage(&cfg, idea, &project_dir, &exec).await {
            ComposeOutcome::Skipped => {}
            ComposeOutcome::Offline => println!("{}", crate::i18n::compose_offline(lang)),
            ComposeOutcome::Composed { usage } => {
                println!("{}", crate::i18n::compose_written(lang));
                if let Some(u) = usage {
                    let model = cfg
                        .providers
                        .get(cfg.routing.spec.as_deref().unwrap_or("glm-coding"))
                        .map(|p| p.model.clone())
                        .unwrap_or_default();
                    let cost = crate::llm::pricing::estimate_cost(&model, &u);
                    println!(
                        "{}",
                        crate::i18n::usage_line(lang, u.input_tokens, u.output_tokens, cost)
                    );
                }
            }
            ComposeOutcome::Fallback { reason } => {
                println!("{}", crate::i18n::compose_fallback(lang, &reason));
            }
        }
    } else {
    let spec_path = project_dir.join("video.spec.json");
    let mut spec_usage: Option<crate::llm::Usage> = None;
    if !spec_path.exists() {
        match generate_spec_with_llm(&cfg, idea, &t).await {
            LlmSpecOutcome::Generated { spec, usage, .. } => {
                std::fs::write(&spec_path, spec.to_json_string()?)?;
                spec_usage = Some(usage);
            }
            LlmSpecOutcome::Offline(_) => {
                println!("{}", crate::i18n::offline_mode(lang));
                std::fs::write(&spec_path, t.build_spec(&slug).to_json_string()?)?;
            }
            LlmSpecOutcome::Fallback(reason) => {
                println!("{}", crate::i18n::llm_fallback(lang, &reason));
                std::fs::write(&spec_path, t.build_spec(&slug).to_json_string()?)?;
            }
        }
    }

    // Codegen stage: LLM writes Custom_*.tsx for custom scenes (best effort).
    let spec_disk = crate::ir::VideoSpec::from_json_str(&std::fs::read_to_string(&spec_path)?)?;
    let mut codegen_usage: Option<crate::llm::Usage> = None;
    let has_custom = spec_disk.scenes.iter().any(|s| s.custom);
    if has_custom {
        let exec = crate::exec::TokioExecutor::new();
        match codegen_custom_scenes(&cfg, &spec_disk, &project_dir, &exec).await {
            CodegenOutcome::Skipped => println!("{}", crate::i18n::codegen_skip(lang)),
            CodegenOutcome::Done { usage, written, fallbacks } => {
                codegen_usage = usage;
                println!("{}", crate::i18n::codegen_written(lang, written));
                for id in fallbacks {
                    println!("{}", crate::i18n::codegen_fallback(lang, &id, "validation"));
                }
            }
        }
    }

    if spec_usage.is_some() || codegen_usage.is_some() {
        let su = spec_usage.unwrap_or(crate::llm::Usage { input_tokens: 0, output_tokens: 0 });
        let cu = codegen_usage.unwrap_or(crate::llm::Usage { input_tokens: 0, output_tokens: 0 });
        let model = cfg
            .providers
            .get(cfg.routing.spec.as_deref().unwrap_or("glm-coding"))
            .map(|p| p.model.clone())
            .unwrap_or_default();
        let cost = crate::llm::pricing::estimate_cost(
            &model,
            &crate::llm::Usage {
                input_tokens: su.input_tokens + cu.input_tokens,
                output_tokens: su.output_tokens + cu.output_tokens,
            },
        );
        println!(
            "{}",
            crate::i18n::codegen_usage_combined(
                lang,
                su.input_tokens,
                su.output_tokens,
                cu.input_tokens,
                cu.output_tokens,
                cost
            )
        );
    }
    } // end remotion branch

    while let Some(stage) = state.next_stage() {
        if stage == "style" {
            let style_name = std::env::var("VF_STYLE").unwrap_or_else(|_| "mg-07".into());
            let style = crate::styles::resolve(&style_name).ok_or_else(|| {
                anyhow::anyhow!("unknown style '{style_name}' — presets: mg-01..mg-15")
            })?;
            std::fs::write(
                project_dir.join("style.json"),
                serde_json::to_string_pretty(&style)?,
            )?;
        }
        if stage == "render" {
            if let Err(e) = run_render(&project_dir).await {
                if !e.to_string().contains("VF_RENDER_SKIPPED") {
                    let lang2 = crate::i18n::Lang::from_env();
                    let manual2 = if engine_name == "hyperframes" {
                        "npx hyperframes render --output draft.mp4"
                    } else {
                        "npm install && npx remotion render src/index.tsx vf-main draft.mp4"
                    };
                    println!("{}", crate::i18n::render_skipped_manual(lang2, manual2));
                    eprintln!("render error: {e}");
                }
                break;
            }
        }
        if stage == "post" {
            if run_post(&project_dir).await.is_err() {
                println!("{}", crate::i18n::post_skipped(lang));
                break;
            }
        }
        println!("{}", crate::i18n::stage_ok(lang, stage));
        state.mark_done(stage, None);
        state.save(&state_path)?;
    }

    println!("{}", crate::i18n::project_line(lang, &project_dir));
    Ok(())
}

pub fn slugify(idea: &str) -> String {
    let slug: String = idea
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    slug.trim_matches('-').to_string()
}

/// Rough LLM call estimate: 1 spec call + calls per template body.
/// Placeholder pricing: $0.002 per call (guardrail math only).
pub fn estimate_calls(t: &crate::templates::Template) -> u64 {
    use crate::templates::Template;
    match t {
        Template::Default => 2,
        Template::Launch | Template::Explainer => 4,
    }
}

async fn run_render(project_dir: &std::path::Path) -> anyhow::Result<()> {
    if std::env::var("VF_RENDER_SKIP").map(|v| v == "1").unwrap_or(false) {
        return Ok(()); // test knob: compose/lint stages verified, render skipped
    }
    let engine_name = std::env::var("VF_ENGINE").unwrap_or_else(|_| "remotion".into());
    let engine = crate::engine::build_engine(&engine_name)?;
    let spec = if engine_name == "hyperframes" {
        // Delegate mode: index.html is the artifact; the spec is unused.
        crate::ir::VideoSpec::demo("hyperframes")
    } else {
        let spec_path = project_dir.join("video.spec.json");
        crate::ir::VideoSpec::from_json_str(&std::fs::read_to_string(&spec_path)?)?
    };
    if engine_name != "hyperframes" {
        crate::engine::scaffold::write_scaffold(project_dir, &spec)?;
        // Scaffold rewrite resets registry.ts — re-register existing custom scenes.
        crate::engine::scaffold::rebuild_registry(project_dir)?;
    }
    if engine_name != "dry-run" {
        let lang = crate::i18n::Lang::from_env();
        let manual = if engine_name == "hyperframes" {
            "npx hyperframes render --output draft.mp4"
        } else {
            "npm install && npx remotion render src/index.tsx vf-main draft.mp4"
        };
        let exec = crate::exec::TokioExecutor::new();
        let ready = crate::cmd::doctor::tool_available(&exec, "node").await
            && crate::cmd::doctor::tool_available(&exec, "npx").await;
        if !ready {
            println!("{}", crate::i18n::render_skipped_manual(lang, manual));
            anyhow::bail!("VF_RENDER_SKIPPED");
        }
        if engine_name == "remotion" {
            println!("{}", crate::i18n::npm_install_running(lang));
        }
        println!("{}", crate::i18n::render_running(lang));
        println!("{}", crate::i18n::chrome_first_run_note(lang));
    }
    engine
        .render(&spec, project_dir, &project_dir.join("draft.mp4"))
        .await?;
    Ok(())
}

/// ffmpeg remux command: copy draft into final without re-encoding.
pub fn post_command(
    draft: &std::path::Path,
    final_path: &std::path::Path,
    project_dir: &std::path::Path,
) -> crate::exec::CommandSpec {
    crate::exec::CommandSpec {
        program: "ffmpeg".into(),
        args: vec![
            "-y".into(),
            "-i".into(),
            draft.to_string_lossy().into_owned(),
            "-c".into(),
            "copy".into(),
            final_path.to_string_lossy().into_owned(),
        ],
        cwd: Some(project_dir.to_path_buf()),
    }
}

async fn run_post(project_dir: &std::path::Path) -> anyhow::Result<()> {
    let exec = crate::exec::TokioExecutor::new();
    run_post_with(project_dir, &exec).await
}

async fn run_post_with(
    project_dir: &std::path::Path,
    exec: &dyn Executor,
) -> anyhow::Result<()> {
    let draft = project_dir.join("draft.mp4");
    let final_path = project_dir.join("final.mp4");
    let dry = std::env::var("VF_ENGINE")
        .map(|v| v == "dry-run")
        .unwrap_or(false);
    if dry {
        std::fs::copy(&draft, &final_path)?;
        return Ok(());
    }
    match exec
        .run(&post_command(&draft, &final_path, project_dir))
        .await
    {
        Ok(res) if res.status == 0 => {}
        Ok(_) | Err(_) => {
            // ffmpeg failed or is missing — degrade to a plain copy.
            println!("{}", crate::i18n::ffmpeg_copy_fallback(crate::i18n::Lang::from_env()));
            std::fs::copy(&draft, &final_path)?;
        }
    }
    Ok(())
}

pub enum LlmSpecOutcome {
    Generated { spec: crate::ir::VideoSpec, usage: crate::llm::Usage, model: String },
    Offline(String),
    Fallback(String),
}

/// Strip markdown fences / surrounding prose; return the outermost {...} span.
pub fn extract_json(text: &str) -> Option<String> {
    let t = text.trim();
    let t = t.strip_prefix("```json").or_else(|| t.strip_prefix("```")).unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    let t = t.trim();
    let start = t.find('{')?;
    let end = t.rfind('}')?;
    (start <= end).then(|| t[start..=end].to_string())
}

fn spec_request(
    idea: &str,
    t: &crate::templates::Template,
    feedback: Option<&str>,
) -> crate::llm::ChatRequest {
    let system = "You are a video producer. Reply with ONLY a JSON object of shape:\n\
{ \"meta\": { \"title\": string, \"duration\": number, \"fps\": number, \
\"ratio\": \"9:16\"|\"16:9\"|\"1:1\", \"lang\": string }, \
\"style\": { \"preset\": string, \"palette\": string[], \"font\": string }, \
\"scenes\": [ { \"id\": string, \"duration\": number, \"narration\": string, \
\"elements\": [], \"transition\": string } ] }\n\
Rules: at least one scene; meta.duration equals the sum of scene durations; fps > 0.";
    let mut user = format!("Idea: {idea}\nTemplate: {t:?}\nOutput the JSON object only.");
    if let Some(fb) = feedback {
        user.push_str(&format!("\nYour previous output failed validation: {fb}. Fix it."));
    }
    crate::llm::ChatRequest { system: Some(system.into()), user }
}

pub async fn generate_spec_with_llm(
    cfg: &crate::config::Config,
    idea: &str,
    t: &crate::templates::Template,
) -> LlmSpecOutcome {
    use crate::llm::LlmClient as _;
    let Some(provider_name) = cfg.routing.spec.clone() else {
        return LlmSpecOutcome::Offline("no routing".into());
    };
    let Some(p) = cfg.providers.get(&provider_name) else {
        return LlmSpecOutcome::Offline(format!("unknown provider '{provider_name}'"));
    };
    let key_missing = std::env::var(&p.api_key_env).map(|v| v.is_empty()).unwrap_or(true);
    if key_missing {
        return LlmSpecOutcome::Offline(p.api_key_env.clone());
    }
    if crate::llm::check_tls(&p.base_url).is_err() {
        return LlmSpecOutcome::Offline("no tls in this binary".into());
    }
    let Ok(inner) = crate::llm::registry::build(&provider_name, cfg) else {
        return LlmSpecOutcome::Offline("provider build failed".into());
    };
    let client = crate::llm::retry::RetryingClient::new(inner);
    let mut feedback: Option<String> = None;
    for _attempt in 0..2 {
        let req = spec_request(idea, t, feedback.as_deref());
        match client.complete(&req).await {
            Err(e) => {
                let reason = crate::i18n::llm_error(crate::i18n::Lang::En, &e, &p.api_key_env);
                return LlmSpecOutcome::Fallback(reason);
            }
            Ok(resp) => {
                let parsed = extract_json(&resp.text)
                    .and_then(|s| crate::ir::VideoSpec::from_json_str(&s).ok());
                match parsed {
                    Some(spec) => match spec.validate() {
                        Ok(()) => {
                            return LlmSpecOutcome::Generated {
                                spec,
                                usage: resp.usage.unwrap_or(crate::llm::Usage {
                                    input_tokens: 0,
                                    output_tokens: 0,
                                }),
                                model: p.model.clone(),
                            }
                        }
                        Err(e) => feedback = Some(e),
                    },
                    None => feedback = Some("output was not a valid JSON spec".into()),
                }
            }
        }
    }
    LlmSpecOutcome::Fallback(feedback.unwrap_or_else(|| "invalid spec after retry".into()))
}

pub enum CodegenOutcome {
    Skipped,
    Done { usage: Option<crate::llm::Usage>, written: usize, fallbacks: Vec<String> },
}

/// Strip code fences; keep the code only when it has a default export.
pub fn extract_code(text: &str) -> Option<String> {
    let t = text.trim();
    let t = t
        .strip_prefix("```tsx")
        .or_else(|| t.strip_prefix("```typescript"))
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    let t = t.trim();
    if t.contains("export default") {
        Some(t.to_string())
    } else {
        None
    }
}

fn codegen_request(
    scene: &crate::ir::Scene,
    style: &crate::ir::Style,
    feedback: Option<&str>,
) -> crate::llm::ChatRequest {
    let system = "You write Remotion scene components. Return ONLY TypeScript TSX code in a \
```tsx fence. Contract: default export a React functional component accepting props \
{ scene, palette, font } (types from ../scenes/registry). You may import from \"remotion\" \
(AbsoluteFill, useCurrentFrame, useVideoConfig, interpolate, spring, Img, staticFile). \
Animate using useCurrentFrame() relative to the scene; the component fills the whole frame. \
No external packages, no network, no files outside staticFile(\"assets/...\").";
    let scene_json = serde_json::to_string(scene).unwrap_or_default();
    let style_json = serde_json::to_string(style).unwrap_or_default();
    let mut user = format!("Scene JSON: {scene_json}\nStyle JSON: {style_json}\nWrite the component now.");
    if let Some(fb) = feedback {
        user.push_str(&format!("\nYour previous code failed validation: {fb}. Fix it."));
    }
    crate::llm::ChatRequest { system: Some(system.into()), user }
}

/// tsc --noEmit gate: true when pass, disabled via VF_TSC=0, or unavailable
/// (no node — light check only, done by extract_code). false = compile error.
async fn tsc_ok(exec: &dyn crate::exec::Executor, project_dir: &std::path::Path) -> bool {
    if std::env::var("VF_TSC").map(|v| v == "0").unwrap_or(false) {
        return true;
    }
    let cmd = crate::exec::CommandSpec {
        program: "npx".into(),
        args: vec!["tsc".into(), "--noEmit".into()],
        cwd: Some(project_dir.to_path_buf()),
    };
    match exec.run(&cmd).await {
        Ok(out) => out.status == 0,
        Err(_) => true,
    }
}

pub async fn codegen_custom_scenes(
    cfg: &crate::config::Config,
    spec: &crate::ir::VideoSpec,
    project_dir: &std::path::Path,
    exec: &dyn crate::exec::Executor,
) -> CodegenOutcome {
    use crate::llm::LlmClient as _;
    let custom: Vec<&crate::ir::Scene> = spec.scenes.iter().filter(|s| s.custom).collect();
    if custom.is_empty() {
        return CodegenOutcome::Skipped;
    }
    let Some(provider_name) = cfg.routing.codegen.clone() else {
        return CodegenOutcome::Skipped;
    };
    let Some(p) = cfg.providers.get(&provider_name) else {
        return CodegenOutcome::Skipped;
    };
    let key_missing = std::env::var(&p.api_key_env).map(|v| v.is_empty()).unwrap_or(true);
    if key_missing || crate::llm::check_tls(&p.base_url).is_err() {
        return CodegenOutcome::Skipped;
    }
    let Ok(inner) = crate::llm::registry::build(&provider_name, cfg) else {
        return CodegenOutcome::Skipped;
    };
    let client = crate::llm::retry::RetryingClient::new(inner);
    let mut usage_total: Option<crate::llm::Usage> = None;
    let mut written = 0usize;
    let mut fallbacks: Vec<String> = Vec::new();
    for scene in custom {
        let path = project_dir
            .join("src/scenes")
            .join(format!("Custom_{}.tsx", scene.id));
        let mut feedback: Option<String> = None;
        let mut done = false;
        for _attempt in 0..2 {
            let req = codegen_request(scene, &spec.style, feedback.as_deref());
            match client.complete(&req).await {
                Err(_) => break,
                Ok(resp) => {
                    if let Some(u) = resp.usage {
                        usage_total = Some(match usage_total {
                            None => u,
                            Some(prev) => crate::llm::Usage {
                                input_tokens: prev.input_tokens + u.input_tokens,
                                output_tokens: prev.output_tokens + u.output_tokens,
                            },
                        });
                    }
                    if let Some(code) = extract_code(&resp.text) {
                        if let Some(parent) = path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let _ = std::fs::write(&path, &code);
                        if tsc_ok(exec, project_dir).await {
                            written += 1;
                            done = true;
                            break;
                        }
                        feedback = Some("TypeScript compile failed".into());
                    } else {
                        feedback = Some("no export default found".into());
                    }
                }
            }
        }
        if !done {
            let _ = std::fs::remove_file(&path);
            fallbacks.push(scene.id.clone());
        }
    }
    let _ = crate::engine::scaffold::rebuild_registry(project_dir);
    CodegenOutcome::Done { usage: usage_total, written, fallbacks }
}

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
    let system = "You write HyperFrames compositions — a designed 9:16 vertical video. \
Return ONLY one complete HTML document inside a ```html fence. START from this exact \
skeleton and replace the content with a rich, well-designed scene sequence — keep ALL \
structural attributes (data-*, class=\"clip\", the GSAP script tag, the timeline \
registration) exactly as shown:\n\
<!doctype html>\n<html lang=\"vi\"><head><meta charset=\"UTF-8\" />\n<script src=\"https://cdn.jsdelivr.net/npm/gsap@3.15.0/dist/gsap.min.js\"></script>\n<style>/* body 1080x1920 overflow hidden */</style></head><body>\n<div id=\"root\" data-composition-id=\"vf-main\" data-start=\"0\" data-duration=\"TOTAL\" data-width=\"1080\" data-height=\"1920\">\n<!-- your timed children: class=\"clip\" data-start=\"..\" data-duration=\"..\" data-track-index=\"0\" -->\n</div>\n<script>\nconst tl = gsap.timeline({ paused: true });\n// your tweens here — every frame must be seekable via tl
window.__timelines[\"vf-main\"] = tl; tl.seek(0);\n</script></body></html>\n\
RULES:\n(1) Multiple .clip scenes with staggered entrance animations (scale, y, opacity tweens on tl).\n(2) CRITICAL lint rule gsap_css_transform_conflict: NEVER set CSS `transform` on any \
element you animate with GSAP — GSAP overwrites the whole transform. Center elements \
with flexbox/margins/position, never `transform: translateX(-50%)` or similar.\n(3) All animation as tweens on the paused timeline tl (seekable); tl.seek(0) last.\n(4) body exactly 1080x1920 px, overflow hidden, high-contrast typography, designed look \
(dark gradient background, accent color, visual hierarchy).\n(5) Media files only from relative assets/ paths (library CDN scripts are fine).";
    let mut user = format!(
        "Idea: {idea}\nTotal duration: about {seconds} seconds.\nWrite the composition now."
    );
    if let Some(fb) = feedback {
        user.push_str(&format!("\nYour previous composition failed validation: {fb}. Fix it."));
    }
    crate::llm::ChatRequest { system: Some(system.into()), user }
}

pub enum ComposeOutcome {
    Skipped,
    Offline,
    Composed { usage: Option<crate::llm::Usage> },
    Fallback { reason: String },
}

/// None = pass; Some(findings) = lint failed with its output (for retry feedback).
async fn lint_findings(
    exec: &dyn crate::exec::Executor,
    project_dir: &std::path::Path,
) -> Option<String> {
    if std::env::var("VF_LINT").map(|v| v == "0").unwrap_or(false) {
        return None;
    }
    let cmd = crate::exec::CommandSpec {
        program: "npx".into(),
        args: vec!["hyperframes".into(), "lint".into()],
        cwd: Some(project_dir.to_path_buf()),
    };
    match exec.run(&cmd).await {
        Ok(out) if out.status == 0 => None,
        Ok(out) => Some(format!("{}\n{}", out.stdout, out.stderr)),
        Err(_) => None, // no npx — light check only (extract_html gated <html)
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
    for _attempt in 0..3 {
        let req = compose_request(idea, 10, feedback.as_deref());
        match client.complete(&req).await {
            Err(e) => {
                let reason = crate::i18n::llm_error(crate::i18n::Lang::En, &e, &p.api_key_env);
                crate::engine::hf_template::write_hf_template(project_dir).ok();
                return ComposeOutcome::Fallback { reason };
            }
            Ok(resp) => {
                if let Some(doc) = extract_html(&resp.text) {
                    std::fs::write(project_dir.join("index.html"), &doc).ok();
                    match lint_findings(exec, project_dir).await {
                        None => return ComposeOutcome::Composed { usage: resp.usage },
                        Some(findings) => {
                            let errors: String = findings
                                .lines()
                                .filter(|l| l.contains('✗'))
                                .collect::<Vec<_>>()
                                .join("; ");
                            feedback = Some(if errors.is_empty() {
                                "hyperframes lint reported errors".to_string()
                            } else {
                                format!("hyperframes lint errors: {errors}")
                            });
                        }
                    }
                } else {
                    feedback = Some("output was not an HTML document".into());
                }
            }
        }
    }
    crate::engine::hf_template::write_hf_template(project_dir).ok();
    ComposeOutcome::Fallback { reason: feedback.unwrap_or_else(|| "invalid composition".into()) }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn post_command_shape() {
        let cmd = super::post_command(Path::new("/d.mp4"), Path::new("/f.mp4"), Path::new("/p"));
        assert_eq!(cmd.program, "ffmpeg");
        assert_eq!(cmd.args[0], "-y");
        assert_eq!(cmd.args[1], "-i");
        assert_eq!(cmd.args[2], "/d.mp4");
        assert_eq!(cmd.args[3], "-c");
        assert_eq!(cmd.args[4], "copy");
        assert_eq!(cmd.args[5], "/f.mp4");
        assert_eq!(cmd.cwd.as_deref(), Some(Path::new("/p")));
    }

    #[test]
    fn estimate_calls_by_template() {
        use crate::templates::Template;
        assert_eq!(super::estimate_calls(&Template::Default), 2);
        assert_eq!(super::estimate_calls(&Template::Launch), 4);
        assert_eq!(super::estimate_calls(&Template::Explainer), 4);
    }

    #[test]
    fn extract_json_strips_fences_and_prose() {
        let fenced = "```json\n{\"a\":1}\n```";
        assert_eq!(super::extract_json(fenced).as_deref(), Some("{\"a\":1}"));
        let prose = "Here you go:\n{\"b\":2} hope it helps";
        assert_eq!(super::extract_json(prose).as_deref(), Some("{\"b\":2}"));
        assert_eq!(super::extract_json("no json at all"), None);
    }

    #[test]
    fn spec_request_appends_feedback_on_second_attempt() {
        use crate::templates::Template;
        let t = Template::Default;
        let first = super::spec_request("idea", &t, None);
        assert!(first.user.contains("idea"));
        assert!(first.system.as_deref().unwrap_or("").contains("meta.duration"));
        let second = super::spec_request("idea", &t, Some("scenes empty"));
        assert!(second.user.contains("scenes empty"));
    }

    mod temp {
        /// Unique dir per call — parallel tests must not share one dir (race:
        /// a shared dir caused a 0-byte read window on CI runners).
        pub fn post_dir(name: &str) -> std::path::PathBuf {
            let d = std::env::temp_dir().join(format!("vf-post-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            d
        }
    }

    #[tokio::test]
    async fn post_falls_back_to_copy_without_ffmpeg() -> anyhow::Result<()> {
        let dir = temp::post_dir("copy");
        std::fs::write(dir.join("draft.mp4"), b"abc")?;
        let exec = crate::exec::testing::MockExecutor::with_outputs(vec![crate::exec::RunOutput {
            status: 127,
            stdout: String::new(),
            stderr: "not found".into(),
        }]);
        super::run_post_with(&dir, &exec).await?;
        assert_eq!(std::fs::read(dir.join("final.mp4"))?, b"abc");
        std::fs::remove_dir_all(&dir).ok();
        Ok(())
    }

    #[test]
    fn extract_code_strips_fences() {
        let tsx = "```tsx\nexport default () => <div/>;\n```";
        assert!(super::extract_code(tsx).unwrap().contains("export default"));
        assert!(super::extract_code("export default 1;").unwrap().contains("export default"));
        assert!(super::extract_code("no code here").is_none());
    }

    #[test]
    fn codegen_request_contract() {
        use crate::ir::VideoSpec;
        let spec = VideoSpec::demo("d");
        let req = super::codegen_request(&spec.scenes[0], &spec.style, None);
        assert!(req.system.as_deref().unwrap_or("").contains("default export"));
        assert!(req.user.contains("s1"));
        let retry = super::codegen_request(&spec.scenes[0], &spec.style, Some("TS2345"));
        assert!(retry.user.contains("TS2345"));
    }

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

    #[tokio::test]
    async fn compose_hf_offline_writes_template() -> anyhow::Result<()> {
        let dir = temp::post_dir("hf-offline");
        let cfg = crate::config::Config::builtin();
        let exec = crate::exec::TokioExecutor::new();
        let out = super::compose_hf_stage(&cfg, "offline hf", &dir, &exec).await;
        assert!(matches!(out, super::ComposeOutcome::Offline));
        assert!(dir.join("index.html").exists());
        let html = std::fs::read_to_string(dir.join("index.html"))?;
        assert!(html.contains("data-composition-id"));
        std::fs::remove_dir_all(&dir).ok();
        Ok(())
    }

    #[tokio::test]
    async fn compose_hf_skips_when_index_exists() {
        let dir = temp::post_dir("hf-skip");
        std::fs::write(dir.join("index.html"), "<html>existing</html>").unwrap();
        let cfg = crate::config::Config::builtin();
        let exec = crate::exec::TokioExecutor::new();
        let out = super::compose_hf_stage(&cfg, "x", &dir, &exec).await;
        assert!(matches!(out, super::ComposeOutcome::Skipped));
        assert_eq!(
            std::fs::read_to_string(dir.join("index.html")).unwrap(),
            "<html>existing</html>"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
