use std::path::Path;

/// Minimal HyperFrames composition modeled on the canonical `hyperframes init`
/// scaffold (lint-clean: root wrapper id/dimensions, clip class, GSAP timeline
/// registry). 9:16, 4s @ default 30fps. `hyperframes.json` is intentionally
/// absent — the CLI writes a default on first use.
pub const TEMPLATE_INDEX_HTML: &str = r##"<!doctype html>
<html lang="vi">
  <head>
    <meta charset="UTF-8" />
    <script src="https://cdn.jsdelivr.net/npm/gsap@3.15.0/dist/gsap.min.js"></script>
    <style>
      * { margin: 0; padding: 0; box-sizing: border-box; }
      html, body { margin: 0; width: 1080px; height: 1920px; overflow: hidden; background: #0F172A; }
      #root { width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; }
      #title { color: #FFFFFF; font-family: Inter, sans-serif; font-size: 96px; font-weight: 700; text-align: center; }
    </style>
  </head>
  <body>
    <div id="root" data-composition-id="vf-main" data-start="0" data-duration="4" data-width="1080" data-height="1920">
      <h1 id="title" class="clip" data-start="0" data-duration="4" data-track-index="0">vf</h1>
    </div>
    <script>
      const tl = gsap.timeline({ paused: true });
      window.__timelines["vf-main"] = tl;
      tl.seek(0);
    </script>
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
        // Canonical contract (from `hyperframes init` + real lint rules):
        assert!(html.contains("data-composition-id=\"vf-main\""));
        assert!(html.contains("data-width=\"1080\""));
        assert!(html.contains("data-height=\"1920\""));
        assert!(html.contains("data-start=\"0\""));
        assert!(html.contains("data-duration=\"4\""));
        assert!(html.contains("data-track-index=\"0\""));
        assert!(html.contains("class=\"clip\""));
        assert!(html.contains("__timelines"));
        assert!(html.contains("gsap.min.js"));
        assert!(html.contains("<!doctype html>"));
        assert!(!dir.join("hyperframes.json").exists()); // optional per docs
        let _ = std::fs::remove_dir_all(&dir);
    }
}
