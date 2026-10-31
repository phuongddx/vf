use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const STAGES: [&str; 6] = ["brief", "spec", "style", "codegen", "render", "post"];

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct State {
    pub project: String,
    #[serde(default)]
    pub stages: BTreeMap<String, StageRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageRecord {
    pub status: Status,
    #[serde(default)]
    pub artifact_hash: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pending,
    Done,
    Failed,
}

impl State {
    pub fn new(project: &str) -> Self {
        let stages = STAGES
            .iter()
            .map(|s| {
                (
                    s.to_string(),
                    StageRecord {
                        status: Status::Pending,
                        artifact_hash: None,
                    },
                )
            })
            .collect();
        Self {
            project: project.to_string(),
            stages,
        }
    }

    pub fn load(path: &Path) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn mark_done(&mut self, stage: &str, hash: Option<String>) {
        if let Some(rec) = self.stages.get_mut(stage) {
            rec.status = Status::Done;
            rec.artifact_hash = hash;
        }
    }

    pub fn next_stage(&self) -> Option<&'static str> {
        STAGES
            .iter()
            .find(|s| self.stages.get(**s).map(|r| r.status) != Some(Status::Done))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_starts_at_brief() {
        let s = State::new("demo");
        assert_eq!(s.next_stage(), Some("brief"));
    }

    #[test]
    fn mark_done_advances_and_round_trips() {
        let dir = std::env::temp_dir().join(format!("vf-state-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        let mut s = State::new("demo");
        s.mark_done("brief", Some("abc".into()));
        s.save(&path).unwrap();
        let back = State::load(&path).unwrap();
        assert_eq!(back.stages["brief"].status, Status::Done);
        assert_eq!(back.stages["brief"].artifact_hash.as_deref(), Some("abc"));
        assert_eq!(back.next_stage(), Some("spec"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
