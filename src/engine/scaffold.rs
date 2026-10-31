use std::path::Path;

/// Writes a minimal bootable Remotion project skeleton:
/// package.json + src/index.tsx + remotion.config.ts.
/// The composition is a static stub — Plan 3 codegen replaces index.tsx
/// with props-driven scenes from video.spec.json.
pub fn write_scaffold(project_dir: &Path) -> anyhow::Result<()> {
    let src = project_dir.join("src");
    std::fs::create_dir_all(&src)?;

    std::fs::write(
        project_dir.join("package.json"),
        r##"{
  "name": "vf-project",
  "private": true,
  "dependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "remotion": "^4.0.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  }
}
"##,
    )?;

    std::fs::write(
        src.join("index.tsx"),
        r##"import { registerRoot, Composition } from "remotion";

// Static stub composition — Plan 3 codegen replaces this with
// props-driven scenes from video.spec.json.
const Scene = () => <div>vf</div>;

const Root = () => (
  <Composition
    id="vf-video"
    component={Scene}
    durationInFrames={120}
    fps={30}
    width={1080}
    height={1920}
  />
);

registerRoot(Root);
"##,
    )?;

    std::fs::write(
        project_dir.join("remotion.config.ts"),
        r##"import { Config } from "@remotion/cli/config";
Config.setVideoImageFormat("jpeg");
"##,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_three_files() {
        let dir = std::env::temp_dir().join(format!("vf-scaf-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_scaffold(&dir).unwrap();
        let pkg: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.join("package.json")).unwrap(),
        )
        .unwrap();
        assert!(pkg["dependencies"]["remotion"].is_string());
        let tsx = std::fs::read_to_string(dir.join("src").join("index.tsx")).unwrap();
        assert!(tsx.contains("registerRoot"));
        assert!(tsx.contains("vf-video"));
        assert!(dir.join("remotion.config.ts").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
