use crate::state::State;

pub fn run(idea: &str) -> anyhow::Result<()> {
    let slug: String = idea
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let slug = slug.trim_matches('-').to_string();

    let project_dir = std::path::Path::new("vf-projects").join(&slug);
    std::fs::create_dir_all(&project_dir)?;

    let state_path = project_dir.join("state.json");
    let mut state = if state_path.exists() {
        State::load(&state_path)?
    } else {
        State::new(&slug)
    };

    while let Some(stage) = state.next_stage() {
        println!("stage: {stage} ok"); // stub: real work lands in Plans 2-5
        state.mark_done(stage, None);
        state.save(&state_path)?;
    }

    println!("project: {}", project_dir.display());
    Ok(())
}
