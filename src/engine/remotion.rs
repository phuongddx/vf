use super::Engine;
use crate::exec::{CommandSpec, Executor};
use crate::ir::VideoSpec;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

pub struct RemotionEngine {
    exec: Box<dyn Executor>,
}

impl RemotionEngine {
    pub fn new(exec: Box<dyn Executor>) -> Self {
        Self { exec }
    }

    pub fn build_command(project_dir: &Path, out: &Path) -> CommandSpec {
        CommandSpec {
            program: "npx".into(),
            args: vec![
                "remotion".into(),
                "render".into(),
                "src/index.tsx".into(),
                "vf-main".into(),
                out.to_string_lossy().into_owned(),
            ],
            cwd: Some(project_dir.to_path_buf()),
        }
    }

    pub fn npm_install_command(project_dir: &Path) -> CommandSpec {
        CommandSpec {
            program: "npm".into(),
            args: vec!["install".into(), "--no-audit".into(), "--no-fund".into()],
            cwd: Some(project_dir.to_path_buf()),
        }
    }
}

/// Stable hash of package.json — used to skip redundant npm installs.
fn install_hash(pkg_json: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    pkg_json.hash(&mut h);
    h.finish()
}

#[async_trait]
impl Engine for RemotionEngine {
    fn name(&self) -> &'static str {
        "remotion"
    }

    async fn render(
        &self,
        spec: &VideoSpec,
        project_dir: &Path,
        out: &Path,
    ) -> anyhow::Result<PathBuf> {
        // The render command runs with cwd = project_dir; absolutize the output
        // so a caller-relative path doesn't nest inside the project dir.
        let out_abs = if out.is_absolute() {
            out.to_path_buf()
        } else {
            std::env::current_dir()?.join(out)
        };
        std::fs::write(project_dir.join("video.spec.json"), spec.to_json_string()?)?;
        let pkg = std::fs::read_to_string(project_dir.join("package.json")).unwrap_or_default();
        let hash_file = project_dir.join(".vf-install-hash");
        let installed = project_dir.join("node_modules").exists()
            && std::fs::read_to_string(&hash_file)
                .map(|s| s.trim() == install_hash(&pkg).to_string())
                .unwrap_or(false);
        if !installed {
            let res = self.exec.run(&Self::npm_install_command(project_dir)).await?;
            if res.status != 0 {
                anyhow::bail!("npm install failed (exit {}): {}", res.status, res.stderr);
            }
            std::fs::write(&hash_file, install_hash(&pkg).to_string())?;
        }
        let cmd = Self::build_command(project_dir, &out_abs);
        let res = self.exec.run(&cmd).await?;
        if res.status != 0 {
            anyhow::bail!(
                "remotion render failed (exit {}): {}",
                res.status,
                res.stderr
            );
        }
        Ok(out.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Engine;
    use crate::exec::testing::MockExecutor;
    use crate::exec::RunOutput;
    use std::sync::Arc;

    // Forwards to a shared MockExecutor so tests can inspect recorded calls,
    // while RemotionEngine owns a Box<dyn Executor>.
    struct Recorder {
        inner: Arc<MockExecutor>,
    }

    #[async_trait::async_trait]
    impl crate::exec::Executor for Recorder {
        async fn run(&self, spec: &crate::exec::CommandSpec) -> anyhow::Result<RunOutput> {
            self.inner.run(spec).await
        }
    }

    fn spec() -> crate::ir::VideoSpec {
        crate::ir::VideoSpec::from_json_str(
            r##"{
          "meta": { "title": "T", "duration": 4, "fps": 30, "ratio": "9:16", "lang": "vi" },
          "style": { "preset": "p", "palette": ["#000000"], "font": "Inter" },
          "scenes": [ { "id": "s1", "duration": 4 } ]
        }"##,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn builds_command_and_writes_props_file() -> anyhow::Result<()> {
        let dir = std::env::temp_dir().join(format!("vf-rem-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        let pkg = "{\"name\":\"vf-project\"}";
        std::fs::write(dir.join("package.json"), pkg)?;
        std::fs::create_dir_all(dir.join("node_modules"))?;
        std::fs::write(dir.join(".vf-install-hash"), install_hash(pkg).to_string())?;
        let mock = Arc::new(MockExecutor::with_outputs(vec![RunOutput {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }]));
        let engine = RemotionEngine::new(Box::new(Recorder { inner: mock.clone() }));
        let out = dir.join("draft.mp4");
        let res = engine.render(&spec(), &dir, &out).await?;
        assert_eq!(res, out);
        let calls = mock.calls.lock().unwrap();
        assert_eq!(calls[0].program, "npx");
        assert_eq!(
            &calls[0].args[0..2],
            &["remotion".to_string(), "render".to_string()]
        );
        assert!(calls[0].args.contains(&"src/index.tsx".to_string()));
        assert!(calls[0].args.contains(&"vf-main".to_string()));
        assert!(!calls[0].args.iter().any(|a| a.starts_with("--props")));
        assert_eq!(calls[0].cwd.as_deref(), Some(dir.as_path()));
        drop(calls);
        let props = std::fs::read_to_string(dir.join("video.spec.json"))?;
        assert!(props.contains("\"title\""));
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[tokio::test]
    async fn nonzero_exit_is_error() {
        let dir = std::env::temp_dir().join(format!("vf-remf-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let pkg = "{\"name\":\"vf-project\"}";
        std::fs::write(dir.join("package.json"), pkg).unwrap();
        std::fs::create_dir_all(dir.join("node_modules")).unwrap();
        std::fs::write(dir.join(".vf-install-hash"), install_hash(pkg).to_string()).unwrap();
        let engine = RemotionEngine::new(Box::new(MockExecutor::with_outputs(vec![RunOutput {
            status: 1,
            stdout: String::new(),
            stderr: "boom".into(),
        }])));
        let err = engine
            .render(&spec(), &dir, &dir.join("x.mp4"))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("remotion render failed"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn npm_install_runs_then_render_when_no_node_modules() -> anyhow::Result<()> {
        let dir = std::env::temp_dir().join(format!("vf-remi-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("package.json"), "{\"name\":\"vf-project\"}")?;
        let mock = Arc::new(MockExecutor::with_outputs(vec![
            RunOutput { status: 0, stdout: String::new(), stderr: String::new() },
            RunOutput { status: 0, stdout: String::new(), stderr: String::new() },
        ]));
        let engine = RemotionEngine::new(Box::new(Recorder { inner: mock.clone() }));
        engine.render(&spec(), &dir, &dir.join("draft.mp4")).await?;
        let calls = mock.calls.lock().unwrap();
        assert_eq!(calls[0].program, "npm");
        assert!(calls[0].args.contains(&"install".to_string()));
        assert_eq!(calls[1].program, "npx");
        assert!(dir.join(".vf-install-hash").exists());
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[tokio::test]
    async fn npm_install_skipped_when_hash_matches() -> anyhow::Result<()> {
        let dir = std::env::temp_dir().join(format!("vf-remh-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        let pkg = "{\"name\":\"vf-project\"}";
        std::fs::write(dir.join("package.json"), pkg)?;
        std::fs::create_dir_all(dir.join("node_modules"))?;
        std::fs::write(dir.join(".vf-install-hash"), install_hash(pkg).to_string())?;
        let mock = Arc::new(MockExecutor::with_outputs(vec![RunOutput {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }]));
        let engine = RemotionEngine::new(Box::new(Recorder { inner: mock.clone() }));
        engine.render(&spec(), &dir, &dir.join("draft.mp4")).await?;
        let calls = mock.calls.lock().unwrap();
        assert_eq!(calls.len(), 1); // render only
        assert_eq!(calls[0].program, "npx");
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[tokio::test]
    async fn relative_output_path_becomes_absolute() -> anyhow::Result<()> {
        let dir = std::env::temp_dir().join(format!("vf-rema-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("proj"))?;
        let pkg = "{\"name\":\"vf-project\"}";
        std::fs::write(dir.join("proj/package.json"), pkg)?;
        std::fs::create_dir_all(dir.join("proj/node_modules"))?;
        std::fs::write(dir.join("proj/.vf-install-hash"), install_hash(pkg).to_string())?;
        let mock = Arc::new(MockExecutor::with_outputs(vec![RunOutput {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }]));
        let engine = RemotionEngine::new(Box::new(Recorder { inner: mock.clone() }));
        let rel_out = std::path::Path::new("rel-out.mp4");
        engine.render(&spec(), &dir.join("proj"), rel_out).await?;
        let calls = mock.calls.lock().unwrap();
        let out_arg = calls[0].args.last().unwrap();
        assert!(out_arg.starts_with('/'), "out arg must be absolute, got: {out_arg}");
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }
}
