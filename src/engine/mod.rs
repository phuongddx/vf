use crate::ir::VideoSpec;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

#[async_trait]
pub trait Engine: Send + Sync {
    fn name(&self) -> &'static str;
    async fn render(
        &self,
        spec: &VideoSpec,
        project_dir: &Path,
        out: &Path,
    ) -> anyhow::Result<PathBuf>;
}

pub fn build_engine(name: &str) -> anyhow::Result<Box<dyn Engine>> {
    match name {
        "remotion" => Ok(Box::new(crate::engine::remotion::RemotionEngine::new(Box::new(
            crate::exec::TokioExecutor::new(),
        )))),
        "dry-run" => Ok(Box::new(crate::engine::dryrun::DryRunEngine)),
        other => anyhow::bail!("unknown engine '{other}' — expected 'remotion' or 'dry-run'"),
    }
}

pub mod dryrun;
pub mod hf_template;
pub mod remotion;
pub mod scaffold;

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

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

    struct StubEngine;

    #[async_trait]
    impl Engine for StubEngine {
        fn name(&self) -> &'static str {
            "stub"
        }

        async fn render(
            &self,
            _spec: &crate::ir::VideoSpec,
            _project_dir: &std::path::Path,
            out: &std::path::Path,
        ) -> anyhow::Result<std::path::PathBuf> {
            Ok(out.to_path_buf())
        }
    }

    #[tokio::test]
    async fn trait_object_works() -> anyhow::Result<()> {
        let e: Box<dyn Engine> = Box::new(StubEngine);
        assert_eq!(e.name(), "stub");
        let out = e
            .render(&spec(), std::path::Path::new("/p"), std::path::Path::new("/o.mp4"))
            .await?;
        assert_eq!(out, std::path::PathBuf::from("/o.mp4"));
        Ok(())
    }
}
