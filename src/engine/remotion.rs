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
                // SDD ruling (plan defect #5): plan doc said "src/index.ts";
                // scaffold writer (Task 4) emits src/index.tsx — unified here.
                "src/index.tsx".into(),
                "vf-video".into(),
                out.to_string_lossy().into_owned(),
                "--props=video.spec.json".into(),
            ],
            cwd: Some(project_dir.to_path_buf()),
        }
    }
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
        std::fs::write(project_dir.join("video.spec.json"), spec.to_json_string()?)?;
        let cmd = Self::build_command(project_dir, out);
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
        std::fs::create_dir_all(&dir)?;
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
        std::fs::create_dir_all(&dir).unwrap();
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
}
