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
            run_render(&slug, &project_dir, template).await?;
        }
        if stage == "post" {
            run_post(&project_dir).await?;
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

async fn run_render(
    slug: &str,
    project_dir: &std::path::Path,
    template: Option<&str>,
) -> anyhow::Result<()> {
    let engine_name = std::env::var("VF_ENGINE").unwrap_or_else(|_| "remotion".into());
    let engine = crate::engine::build_engine(&engine_name)?;
    let spec_path = project_dir.join("video.spec.json");
    let spec = if spec_path.exists() {
        crate::ir::VideoSpec::from_json_str(&std::fs::read_to_string(&spec_path)?)?
    } else {
        let t = crate::templates::Template::from_str(template.unwrap_or("default"))?;
        t.build_spec(slug)
    };
    // Pipeline owns the IR artifact: video.spec.json lands in project_dir for
    // EVERY engine (stage ghi artifact), not just Remotion.
    std::fs::write(&spec_path, spec.to_json_string()?)?;
    crate::engine::scaffold::write_scaffold(project_dir)?;
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
    let draft = project_dir.join("draft.mp4");
    let final_path = project_dir.join("final.mp4");
    let dry = std::env::var("VF_ENGINE")
        .map(|v| v == "dry-run")
        .unwrap_or(false);
    if dry {
        std::fs::copy(&draft, &final_path)?;
    } else {
        let exec = crate::exec::TokioExecutor::new();
        let res = exec
            .run(&post_command(&draft, &final_path, project_dir))
            .await?;
        if res.status != 0 {
            anyhow::bail!("ffmpeg post failed (exit {}): {}", res.status, res.stderr);
        }
    }
    Ok(())
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
}
