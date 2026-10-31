use crate::exec::{CommandSpec, Executor};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillEntry {
    pub name: String,
    pub url: String,
    pub sha: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SkillLock {
    pub skills: Vec<SkillEntry>,
}

impl SkillLock {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(s) => Ok(serde_json::from_str(&s)?),
            Err(_) => Ok(Self::default()),
        }
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Insert or replace by name.
    pub fn upsert(&mut self, entry: SkillEntry) {
        self.skills.retain(|e| e.name != entry.name);
        self.skills.push(entry);
    }
}

pub fn name_from_url(url: &str) -> String {
    url.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("skill")
        .trim_end_matches(".git")
        .to_string()
}

pub struct SkillManager {
    exec: Box<dyn Executor>,
}

impl SkillManager {
    pub fn new(exec: Box<dyn Executor>) -> Self {
        Self { exec }
    }

    /// Clone `url` into `skills_dir/<name>` and pin its HEAD sha.
    /// Uses the system git binary via the Executor (see ledger ruling: no gix).
    pub async fn install(&self, url: &str, skills_dir: &Path) -> anyhow::Result<SkillEntry> {
        let name = name_from_url(url);
        let dir = skills_dir.join(&name);
        let clone = CommandSpec {
            program: "git".into(),
            args: vec![
                "clone".into(),
                "--depth".into(),
                "1".into(),
                url.to_string(),
                dir.to_string_lossy().into_owned(),
            ],
            cwd: None,
        };
        let out = self.exec.run(&clone).await?;
        if out.status != 0 {
            anyhow::bail!("git clone failed (exit {}): {}", out.status, out.stderr);
        }
        let rev = CommandSpec {
            program: "git".into(),
            args: vec![
                "-C".into(),
                dir.to_string_lossy().into_owned(),
                "rev-parse".into(),
                "HEAD".into(),
            ],
            cwd: Some(dir),
        };
        let out = self.exec.run(&rev).await?;
        if out.status != 0 {
            anyhow::bail!(
                "git rev-parse failed (exit {}): {}",
                out.status,
                out.stderr
            );
        }
        Ok(SkillEntry {
            name,
            url: url.to_string(),
            sha: out.stdout.trim().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exec::testing::MockExecutor;
    use crate::exec::{CommandSpec, RunOutput};
    use std::sync::Arc;

    struct Recorder {
        inner: Arc<MockExecutor>,
    }

    #[async_trait::async_trait]
    impl crate::exec::Executor for Recorder {
        async fn run(&self, spec: &CommandSpec) -> anyhow::Result<RunOutput> {
            self.inner.run(spec).await
        }
    }

    fn ok_out(stdout: &str) -> RunOutput {
        RunOutput {
            status: 0,
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    #[tokio::test]
    async fn install_clones_and_pins_sha() -> anyhow::Result<()> {
        let dir = std::env::temp_dir().join(format!("vf-sk-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let mock = Arc::new(MockExecutor::with_outputs(vec![ok_out(""), ok_out("abc123\n")]));
        let mgr = SkillManager::new(Box::new(Recorder { inner: mock.clone() }));
        let entry = mgr
            .install("https://github.com/x/my-skill.git", &dir)
            .await?;
        assert_eq!(entry.name, "my-skill");
        assert_eq!(entry.sha, "abc123");
        let calls = mock.calls.lock().unwrap();
        assert_eq!(calls[0].program, "git");
        assert_eq!(
            &calls[0].args[0..3],
            &["clone".to_string(), "--depth".to_string(), "1".to_string()]
        );
        assert_eq!(
            &calls[1].args[0..2],
            &[
                "-C".to_string(),
                dir.join("my-skill").to_string_lossy().into_owned()
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn name_from_url_handles_suffixes() {
        assert_eq!(name_from_url("https://github.com/a/b.git"), "b");
        assert_eq!(name_from_url("https://github.com/a/b/"), "b");
        assert_eq!(name_from_url("/local/path/text2prod"), "text2prod");
    }

    #[test]
    fn lock_round_trip_and_upsert() {
        let dir = std::env::temp_dir().join(format!("vf-lock-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("vf.lock");
        let mut lock = SkillLock::default();
        lock.upsert(SkillEntry {
            name: "a".into(),
            url: "u".into(),
            sha: "s1".into(),
        });
        lock.upsert(SkillEntry {
            name: "a".into(),
            url: "u2".into(),
            sha: "s2".into(),
        });
        lock.upsert(SkillEntry {
            name: "b".into(),
            url: "u".into(),
            sha: "s".into(),
        });
        lock.save(&path).unwrap();
        let back = SkillLock::load(&path).unwrap();
        assert_eq!(back.skills.len(), 2);
        assert_eq!(back.skills[0].sha, "s2");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
