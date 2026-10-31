use crate::state::State;

pub async fn run(project: &str) -> anyhow::Result<()> {
    let path = std::path::Path::new("vf-projects")
        .join(project)
        .join("state.json");
    if !path.exists() {
        anyhow::bail!("no project '{project}' (looked at {})", path.display());
    }
    let state = State::load(&path)?;
    for stage in crate::state::STAGES {
        let status = match state.stages.get(stage).map(|r| r.status) {
            Some(crate::state::Status::Done) => "done",
            Some(crate::state::Status::Failed) => "failed",
            _ => "pending",
        };
        println!("{stage}: {status}");
    }
    Ok(())
}
