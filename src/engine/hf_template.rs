use std::path::Path;

/// Vendored HyperFrames composition — a designed 10s 3-scene demo card
/// (intro brand, feature lines, closing tagline) with GSAP tweens on the
/// paused seekable timeline. Modeled on the canonical `hyperframes init`
/// scaffold: lint-clean (root wrapper id/dimensions, .clip class, timeline
/// registry). `hyperframes.json` is intentionally absent — the CLI writes a
/// default on first use.
pub const TEMPLATE_INDEX_HTML: &str = r##"<!doctype html>
<html lang="vi">
  <head>
    <meta charset="UTF-8" />
    <script src="https://cdn.jsdelivr.net/npm/gsap@3.15.0/dist/gsap.min.js"></script>
    <style>
      * { margin: 0; padding: 0; box-sizing: border-box; }
      html, body { margin: 0; width: 1080px; height: 1920px; overflow: hidden;
                    background: linear-gradient(160deg, #0F172A 0%, #1e1b4b 60%, #312e81 100%); }
      .scene { position: absolute; inset: 0; display: flex; flex-direction: column;
               align-items: center; justify-content: center; text-align: center;
               font-family: Inter, sans-serif; color: #F8FAFC; }
      .brand { font-size: 200px; font-weight: 800; letter-spacing: -0.04em; color: #22D3EE; }
      .brand-sub { font-size: 62px; font-weight: 600; color: #A5B4FC; margin-top: 16px; }
      .feature { font-size: 58px; font-weight: 600; line-height: 1.7; color: #E2E8F0; }
      .feature .hl { color: #22D3EE; }
      .tagline { font-size: 74px; font-weight: 700; color: #F8FAFC; }
      .url { font-size: 44px; font-weight: 500; color: #94A3B8; margin-top: 28px; }
    </style>
  </head>
  <body>
    <div id="root" data-composition-id="vf-main" data-start="0" data-duration="10" data-width="1080" data-height="1920">
      <div id="scene-1" class="scene clip" data-start="0" data-duration="4" data-track-index="0">
        <div class="brand" id="s1-brand">vf</div>
        <div class="brand-sub" id="s1-sub">VIDEO FORGE</div>
      </div>
      <div id="scene-2" class="scene clip" data-start="4" data-duration="4" data-track-index="0">
        <div class="feature" id="s2-f1">Text → video, <span class="hl">một câu lệnh</span></div>
        <div class="feature" id="s2-f2">2 engines: <span class="hl">Remotion + HyperFrames</span></div>
        <div class="feature" id="s2-f3">Powered by <span class="hl">GLM</span></div>
      </div>
      <div id="scene-3" class="scene clip" data-start="8" data-duration="2" data-track-index="0">
        <div class="tagline" id="s3-tag">Ý tưởng → video sản phẩm</div>
        <div class="url" id="s3-url">github.com/phuongddx/vf</div>
      </div>
    </div>
    <script>
      const tl = gsap.timeline({ paused: true });
      tl.fromTo("#s1-brand", { opacity: 0, scale: 0.4 }, { opacity: 1, scale: 1, duration: 0.9, ease: "power4.out" }, 0.15);
      tl.fromTo("#s1-sub", { opacity: 0, y: 40 }, { opacity: 1, y: 0, duration: 0.7, ease: "power3.out" }, 0.65);
      tl.fromTo("#s2-f1", { opacity: 0, x: -90 }, { opacity: 1, x: 0, duration: 0.6, ease: "power3.out" }, 4.2);
      tl.fromTo("#s2-f2", { opacity: 0, x: -90 }, { opacity: 1, x: 0, duration: 0.6, ease: "power3.out" }, 4.7);
      tl.fromTo("#s2-f3", { opacity: 0, x: -90 }, { opacity: 1, x: 0, duration: 0.6, ease: "power3.out" }, 5.2);
      tl.fromTo("#s3-tag", { opacity: 0, scale: 0.7 }, { opacity: 1, scale: 1, duration: 0.8, ease: "back.out(1.7)" }, 8.2);
      tl.fromTo("#s3-url", { opacity: 0 }, { opacity: 1, duration: 0.8 }, 8.7);
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
        assert!(html.contains("data-duration=\"10\""));
        assert!(html.contains("data-track-index=\"0\""));
        assert!(html.contains("class=\"scene clip\""));
        assert!(html.contains("__timelines"));
        assert!(html.contains("gsap.min.js"));
        assert!(html.contains("fromTo"));
        assert!(html.contains("<!doctype html>"));
        assert!(!dir.join("hyperframes.json").exists()); // optional per docs
        let _ = std::fs::remove_dir_all(&dir);
    }
}
