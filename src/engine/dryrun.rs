use super::Engine;
use crate::ir::VideoSpec;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

pub struct DryRunEngine;

#[async_trait]
impl Engine for DryRunEngine {
    fn name(&self) -> &'static str {
        "dry-run"
    }

    async fn render(
        &self,
        _spec: &VideoSpec,
        _project_dir: &Path,
        out: &Path,
    ) -> anyhow::Result<PathBuf> {
        std::fs::write(out, b"dry-run")?;
        Ok(out.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::build_engine;
    use crate::engine::Engine;

    #[tokio::test]
    async fn dry_run_engine_writes_marker_file() -> anyhow::Result<()> {
        let dir = std::env::temp_dir().join(format!("vf-dry-unit-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let spec = crate::ir::VideoSpec::demo("t");
        let out = dir.join("draft.mp4");
        let e = super::DryRunEngine;
        let res = e.render(&spec, &dir, &out).await?;
        assert_eq!(res, out);
        assert_eq!(std::fs::read(&out)?, b"dry-run");
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn factory_builds_dry_run_and_rejects_unknown() {
        assert_eq!(build_engine("dry-run").unwrap().name(), "dry-run");
        assert_eq!(build_engine("remotion").unwrap().name(), "remotion");
        assert!(build_engine("bogus").is_err());
    }
}
