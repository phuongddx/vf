use crate::exec::Executor;
use crate::state::State;

pub async fn run(idea: &str) -> anyhow::Result<()> {
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
            run_render(&slug, &project_dir).await?;
        }
        if stage == "post" {
            run_post(&project_dir).await?;
        }
        println!("stage: {stage} ok");
        state.mark_done(stage, None);
        state.save(&state_path)?;
    }

    println!("project: {}", project_dir.display());
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

async fn run_render(slug: &str, project_dir: &std::path::Path) -> anyhow::Result<()> {
    let engine_name = std::env::var("VF_ENGINE").unwrap_or_else(|_| "remotion".into());
    let engine = crate::engine::build_engine(&engine_name)?;
    let spec_path = project_dir.join("video.spec.json");
    let spec = if spec_path.exists() {
        crate::ir::VideoSpec::from_json_str(&std::fs::read_to_string(&spec_path)?)?
    } else {
        crate::ir::VideoSpec::demo(slug)
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
}
