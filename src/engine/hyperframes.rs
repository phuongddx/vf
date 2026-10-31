use super::Engine;
use crate::exec::{CommandSpec, Executor};
use crate::ir::VideoSpec;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

pub struct HyperframesEngine {
    exec: Box<dyn Executor>,
}

impl HyperframesEngine {
    pub fn new(exec: Box<dyn Executor>) -> Self {
        Self { exec }
    }

    pub fn render_command(project_dir: &Path, out: &Path) -> CommandSpec {
        CommandSpec {
            program: "npx".into(),
            args: vec![
                "hyperframes".into(),
                "render".into(),
                "--output".into(),
                out.to_string_lossy().into_owned(),
            ],
            cwd: Some(project_dir.to_path_buf()),
        }
    }
}

#[async_trait]
impl Engine for HyperframesEngine {
    fn name(&self) -> &'static str {
        "hyperframes"
    }

    async fn render(
        &self,
        _spec: &VideoSpec, // delegate mode: index.html is the artifact; spec unused
        project_dir: &Path,
        out: &Path,
    ) -> anyhow::Result<PathBuf> {
        let out_abs = if out.is_absolute() {
            out.to_path_buf()
        } else {
            std::env::current_dir()?.join(out)
        };
        let cmd = Self::render_command(project_dir, &out_abs);
        let res = self.exec.run(&cmd).await?;
        if res.status != 0 {
            anyhow::bail!("hyperframes render failed (exit {}): {}", res.status, res.stderr);
        }
        Ok(out_abs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Engine;
    use crate::exec::testing::MockExecutor;
    use crate::exec::RunOutput;
    use std::sync::Arc;

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
        crate::ir::VideoSpec::demo("hf")
    }

    #[tokio::test]
    async fn renders_via_npx_with_absolute_output() -> anyhow::Result<()> {
        let dir = std::env::temp_dir().join(format!("vf-hfe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        let mock = Arc::new(MockExecutor::with_outputs(vec![RunOutput {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }]));
        let engine = HyperframesEngine::new(Box::new(Recorder { inner: mock.clone() }));
        let rel_out = std::path::Path::new("rel-hf.mp4");
        engine.render(&spec(), &dir, rel_out).await?;
        let calls = mock.calls.lock().unwrap();
        assert_eq!(calls[0].program, "npx");
        assert!(calls[0].args.contains(&"hyperframes".to_string()));
        assert!(calls[0].args.contains(&"render".to_string()));
        let out_arg = calls[0].args.last().unwrap();
        assert!(out_arg.starts_with('/'), "out must be absolute, got {out_arg}");
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[tokio::test]
    async fn nonzero_exit_is_error() {
        let dir = std::env::temp_dir().join(format!("vf-hfef-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let engine = HyperframesEngine::new(Box::new(MockExecutor::with_outputs(vec![RunOutput {
            status: 1,
            stdout: String::new(),
            stderr: "boom".into(),
        }])));
        assert!(engine.render(&spec(), &dir, &dir.join("x.mp4")).await.is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
