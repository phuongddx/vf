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
        if stage == "render" {
            run_render(&slug, &project_dir).await?;
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
    // EVERY engine (Quy tắc vàng: stage ghi artifact), not just Remotion.
    std::fs::write(&spec_path, spec.to_json_string()?)?;
    crate::engine::scaffold::write_scaffold(project_dir)?;
    engine
        .render(&spec, project_dir, &project_dir.join("draft.mp4"))
        .await?;
    Ok(())
}
