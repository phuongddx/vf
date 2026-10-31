use std::path::Path;

/// Minimal HyperFrames composition that always renders: 9:16, 4s @ 30fps,
/// one timed text element. `hyperframes.json` is intentionally absent — the
/// CLI writes a default on first use (docs: packages/cli.md, hyperframes.json).
pub const TEMPLATE_INDEX_HTML: &str = r##"<!doctype html>
<html data-fps="30">
<head>
<meta charset="utf-8">
<title>vf</title>
<style>
  html, body { margin: 0; width: 1080px; height: 1920px; background: #0F172A;
               color: #FFFFFF; font-family: Inter, sans-serif; overflow: hidden; }
  .center { display: flex; align-items: center; justify-content: center; height: 100%; }
  h1 { font-size: 96px; text-align: center; margin: 0; }
</style>
</head>
<body>
  <div class="center" data-start="0" data-duration="4" data-track-index="0">
    <h1>vf</h1>
  </div>
</body>
</html>
"##;

pub fn write_hf_template(project_dir: &Path) -> anyhow::Result<()> {
    std::fs::write(project_dir.join("index.html"), TEMPLATE_INDEX_HTML)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_minimal_valid_composition() {
        let dir = std::env::temp_dir().join(format!("vf-hft-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_hf_template(&dir).unwrap();
        let html = std::fs::read_to_string(dir.join("index.html")).unwrap();
        assert!(html.contains("data-fps=\"30\""));
        assert!(html.contains("data-start=\"0\""));
        assert!(html.contains("data-duration=\"4\""));
        assert!(html.contains("data-track-index=\"0\""));
        assert!(html.contains("<!doctype html>"));
        assert!(!dir.join("hyperframes.json").exists()); // optional per docs
        let _ = std::fs::remove_dir_all(&dir);
    }
}
