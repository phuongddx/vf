use std::path::Path;

/// Writes a real, working Remotion project scaffold:
/// package.json (pinned deps) + tsconfig.json + remotion.config.ts +
/// public/ + src/index.tsx (Composition `vf-main`, baked fps/frames/dims
/// from the spec) + src/Player.tsx (data-driven baseline player) +
/// src/SceneView.tsx (template scene renderer) + src/scenes/registry.ts
/// (empty custom-scene registry; codegen + rebuild_registry manage it).
///
/// Composition dims/fps/frames are baked at write time so the bundle needs
/// no runtime computation; scene CONTENT stays data-driven via the spec
/// JSON import in Player.tsx.
pub fn write_scaffold(project_dir: &Path, spec: &crate::ir::VideoSpec) -> anyhow::Result<()> {
    let src = project_dir.join("src");
    let scenes = src.join("scenes");
    std::fs::create_dir_all(&scenes)?;
    std::fs::create_dir_all(project_dir.join("public"))?;
    std::fs::write(project_dir.join("public/.gitkeep"), "")?;

    std::fs::write(
        project_dir.join("package.json"),
        r##"{
  "name": "vf-project",
  "private": true,
  "dependencies": {
    "@remotion/cli": "^4.0.539",
    "react": "^19.3.1",
    "react-dom": "^19.3.1",
    "remotion": "^4.0.539"
  },
  "devDependencies": {
    "@types/react": "^19.3.0",
    "typescript": "^7.0.2"
  }
}
"##,
    )?;

    std::fs::write(
        project_dir.join("tsconfig.json"),
        r##"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "strict": true,
    "resolveJsonModule": true,
    "skipLibCheck": true,
    "noEmit": true
  },
  "include": ["src", "remotion.config.ts", "video.spec.json"]
}
"##,
    )?;

    std::fs::write(
        src.join("index.tsx"),
        format!(
            r##"import {{ registerRoot, Composition }} from "remotion";
import {{ Player }} from "./Player";

registerRoot(() => (
  <Composition
    id="vf-main"
    component={{Player}}
    durationInFrames={{{frames}}}
    fps={{{fps}}}
    width={{{w}}}
    height={{{h}}}
  />
));
"##,
            fps = spec.meta.fps,
            frames = spec.meta.duration * spec.meta.fps as u32,
            w = dims(spec).0,
            h = dims(spec).1,
        ),
    )?;

    std::fs::write(
        src.join("Player.tsx"),
        r##"import React from "react";
import { AbsoluteFill, Sequence } from "remotion";
import rawSpec from "../video.spec.json";
import { SceneView } from "./SceneView";
import { customScenes, SceneLike } from "./scenes/registry";

const spec = rawSpec as unknown as {
  meta: { title: string; duration: number; fps: number; lang: string };
  style: { preset: string; palette: string[]; font: string };
  scenes: SceneLike[];
};

export const Player: React.FC = () => {
  const fps = spec.meta.fps;
  let frame = 0;
  return (
    <AbsoluteFill style={{ backgroundColor: spec.style.palette[0] ?? "#0F172A" }}>
      {spec.scenes.map((scene) => {
        const from = frame;
        const dur = scene.duration * fps;
        frame += dur;
        const Custom = scene.custom === true ? customScenes[scene.id] : undefined;
        return (
          <Sequence key={scene.id} from={from} durationInFrames={dur}>
            {Custom ? (
              <Custom scene={scene} palette={spec.style.palette} font={spec.style.font} />
            ) : (
              <SceneView scene={scene} palette={spec.style.palette} font={spec.style.font} />
            )}
          </Sequence>
        );
      })}
    </AbsoluteFill>
  );
};
"##,
    )?;

    std::fs::write(
        src.join("SceneView.tsx"),
        r##"import React from "react";
import { AbsoluteFill, Img, interpolate, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import type { SceneProps } from "./scenes/registry";

export const SceneView: React.FC<SceneProps> = ({ scene, palette, font }) => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const fg = palette[1] ?? "#FFFFFF";
  const bg = palette[0] ?? "#0F172A";
  const opacity = interpolate(
    frame,
    [0, 10, Math.max(11, durationInFrames - 10), durationInFrames],
    [0, 1, 1, 0],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
  );
  const elements = scene.elements ?? [];
  return (
    <AbsoluteFill style={{ backgroundColor: bg, fontFamily: font, opacity }}>
      {scene.narration ? (
        <AbsoluteFill style={{ justifyContent: "flex-end", alignItems: "center", paddingBottom: 140 }}>
          <div style={{ color: fg, fontSize: 64, fontWeight: 700, textAlign: "center", maxWidth: "85%" }}>
            {scene.narration}
          </div>
        </AbsoluteFill>
      ) : null}
      {elements.map((el, i) =>
        el.type === "image" && el.src ? (
          <Img
            key={i}
            src={el.src.startsWith("assets/") ? staticFile(el.src) : el.src}
            style={{ position: "absolute", width: "80%", left: "10%", top: `${20 + i * 25}%` }}
          />
        ) : (
          <div
            key={i}
            style={{
              position: "absolute",
              color: fg,
              fontSize: 72,
              fontWeight: 700,
              textAlign: "center",
              top: `${30 + i * 20}%`,
              width: "100%",
            }}
          >
            {el.content ?? ""}
          </div>
        )
      )}
    </AbsoluteFill>
  );
};
"##,
    )?;

    std::fs::write(
        scenes.join("registry.ts"),
        r##"import type { FC } from "react";

export interface SceneLike {
  id: string;
  duration: number;
  custom?: boolean;
  narration?: string | null;
  elements?: Array<{ type: string; content?: string; src?: string; motion?: string | null }>;
}

export interface SceneProps {
  scene: SceneLike;
  palette: string[];
  font: string;
}

// vf regenerates this file by scanning src/scenes/Custom_*.tsx — do not edit.
export const customScenes: Record<string, FC<SceneProps>> = {};
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

fn dims(spec: &crate::ir::VideoSpec) -> (u32, u32) {
    match spec.meta.ratio {
        crate::ir::Ratio::SixteenByNine => (1920, 1080),
        crate::ir::Ratio::Square => (1080, 1080),
        crate::ir::Ratio::NineBySixteen => (1080, 1920),
    }
}

/// Regenerate src/scenes/registry.ts from existing Custom_s*.tsx files.
/// Returns the number of custom components registered.
pub fn rebuild_registry(project_dir: &Path) -> anyhow::Result<usize> {
    let scenes_dir = project_dir.join("src").join("scenes");
    let mut ids: Vec<String> = Vec::new();
    if scenes_dir.exists() {
        for entry in std::fs::read_dir(&scenes_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(id) = name
                .strip_prefix("Custom_")
                .and_then(|s| s.strip_suffix(".tsx"))
            {
                ids.push(id.to_string());
            }
        }
    }
    ids.sort();
    let mut out = String::from(
        "import type { FC } from \"react\";\n\nexport interface SceneLike {\n  id: string;\n  duration: number;\n  custom?: boolean;\n  narration?: string | null;\n  elements?: Array<{ type: string; content?: string; src?: string; motion?: string | null }>;\n}\n\nexport interface SceneProps {\n  scene: SceneLike;\n  palette: string[];\n  font: string;\n}\n\n// vf regenerates this file by scanning src/scenes/Custom_*.tsx — do not edit.\n",
    );
    for id in &ids {
        out.push_str(&format!("import Custom_{id} from \"./Custom_{id}\";\n"));
    }
    out.push_str("\nexport const customScenes: Record<string, FC<SceneProps>> = {\n");
    for id in &ids {
        out.push_str(&format!("  \"{id}\": Custom_{id},\n"));
    }
    out.push_str("};\n");
    std::fs::create_dir_all(&scenes_dir)?;
    std::fs::write(scenes_dir.join("registry.ts"), out)?;
    Ok(ids.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_full_remotion_project() {
        let dir = std::env::temp_dir().join(format!("vf-scaf-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let spec = crate::ir::VideoSpec::demo("demo");
        write_scaffold(&dir, &spec).unwrap();

        let pkg: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.join("package.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(pkg["dependencies"]["remotion"], "^4.0.539");
        assert_eq!(pkg["dependencies"]["@remotion/cli"], "^4.0.539");
        assert_eq!(pkg["dependencies"]["react"], "^19.3.1");
        assert_eq!(pkg["devDependencies"]["typescript"], "^7.0.2");

        let tsconfig: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.join("tsconfig.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(tsconfig["compilerOptions"]["resolveJsonModule"], true);

        let index = std::fs::read_to_string(dir.join("src/index.tsx")).unwrap();
        assert!(index.contains("vf-main"));
        assert!(index.contains("durationInFrames={120}")); // demo: 4s * 30fps
        assert!(index.contains("width={1080}"));
        assert!(index.contains("height={1920}"));

        assert!(dir.join("src/Player.tsx").exists());
        let player = std::fs::read_to_string(dir.join("src/Player.tsx")).unwrap();
        assert!(player.contains(r#"from "../video.spec.json""#));
        let sv = std::fs::read_to_string(dir.join("src/SceneView.tsx")).unwrap();
        assert!(sv.contains("useCurrentFrame"));
        let reg = std::fs::read_to_string(dir.join("src/scenes/registry.ts")).unwrap();
        assert!(reg.contains("customScenes"));
        assert!(dir.join("public").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rebuild_registry_scans_custom_files() {
        let dir = std::env::temp_dir().join(format!("vf-reg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let spec = crate::ir::VideoSpec::demo("d");
        write_scaffold(&dir, &spec).unwrap();
        std::fs::write(dir.join("src/scenes/Custom_s1.tsx"), "export default () => null;").unwrap();
        std::fs::write(dir.join("src/scenes/Custom_s7.tsx"), "export default () => null;").unwrap();
        let n = rebuild_registry(&dir).unwrap();
        assert_eq!(n, 2);
        let reg = std::fs::read_to_string(dir.join("src/scenes/registry.ts")).unwrap();
        assert!(reg.contains("Custom_s1"));
        assert!(reg.contains("Custom_s7"));
        assert!(reg.contains("\"s7\": Custom_s7,"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
