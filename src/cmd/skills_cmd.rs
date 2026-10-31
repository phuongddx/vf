use crate::exec::TokioExecutor;
use crate::skills::{SkillLock, SkillManager};
use std::path::PathBuf;

pub async fn run(url: Option<String>) -> anyhow::Result<()> {
    let skills_dir: PathBuf = std::env::var("VF_SKILLS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".vf/skills"));
    let lock_path: PathBuf = std::env::var("VF_LOCK_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("vf.lock"));
    match url {
        Some(url) => {
            let mgr = SkillManager::new(Box::new(TokioExecutor::new()));
            let entry = mgr.install(&url, &skills_dir).await?;
            let short: String = entry.sha.chars().take(7).collect();
            let mut lock = SkillLock::load(&lock_path)?;
            lock.upsert(entry.clone());
            lock.save(&lock_path)?;
            println!("skill {} @ {short} installed", entry.name);
        }
        None => {
            let lock = SkillLock::load(&lock_path)?;
            if lock.skills.is_empty() {
                println!("no skills installed");
            }
            for s in &lock.skills {
                let short: String = s.sha.chars().take(7).collect();
                println!("{} @ {short} — {}", s.name, s.url);
            }
        }
    }
    Ok(())
}
