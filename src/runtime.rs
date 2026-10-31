use std::path::{Path, PathBuf};

/// vf home directory: VF_HOME env override, else ./.vf (project-local).
pub fn home_dir() -> PathBuf {
    std::env::var("VF_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".vf"))
}

pub struct RuntimePaths {
    pub home: PathBuf,
}

impl RuntimePaths {
    pub fn resolve() -> Self {
        Self { home: home_dir() }
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.home.join("skills")
    }

    pub fn runtime_dir(&self) -> PathBuf {
        self.home.join("runtime")
    }

    pub fn ensure_all(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(self.skills_dir())?;
        std::fs::create_dir_all(self.runtime_dir())?;
        Ok(())
    }
}

/// Download command shape for managed-runtime components (node/chrome/ffmpeg
/// sidecars). Shape only — actual downloads are deferred (see ledger ruling).
pub fn bootstrap_command(url: &str, dest: &Path) -> crate::exec::CommandSpec {
    crate::exec::CommandSpec {
        program: "curl".into(),
        args: vec![
            "-L".into(),
            "-o".into(),
            dest.to_string_lossy().into_owned(),
            url.to_string(),
        ],
        cwd: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_layout() {
        let rp = RuntimePaths {
            home: std::path::PathBuf::from("/h"),
        };
        assert_eq!(rp.skills_dir(), std::path::PathBuf::from("/h/skills"));
        assert_eq!(rp.runtime_dir(), std::path::PathBuf::from("/h/runtime"));
    }

    #[test]
    fn default_home_is_relative_vf() {
        // VF_HOME is not set in this test process; default must be ./.vf
        assert_eq!(home_dir(), std::path::PathBuf::from(".vf"));
    }

    #[test]
    fn ensure_all_creates_dirs() {
        let dir = std::env::temp_dir().join(format!("vf-rt-{}", std::process::id()));
        let rp = RuntimePaths { home: dir.clone() };
        rp.ensure_all().unwrap();
        assert!(dir.join("skills").is_dir());
        assert!(dir.join("runtime").is_dir());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bootstrap_command_shape() {
        let cmd = bootstrap_command("https://x/y.tar.gz", std::path::Path::new("/d/y.tar.gz"));
        assert_eq!(cmd.program, "curl");
        assert_eq!(cmd.args[0], "-L");
        assert_eq!(cmd.args[1], "-o");
        assert_eq!(cmd.args[2], "/d/y.tar.gz");
        assert_eq!(cmd.args[3], "https://x/y.tar.gz");
    }
}
