use crate::ir::{Meta, Ratio, Scene, Style, VideoSpec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    Default,
    Launch,
    Explainer,
}

impl Template {
    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        match s.to_lowercase().as_str() {
            "default" => Ok(Self::Default),
            "launch" => Ok(Self::Launch),
            "explainer" => Ok(Self::Explainer),
            other => anyhow::bail!("unknown template '{other}' — expected default|launch|explainer"),
        }
    }

    /// Build a valid VideoSpec for this template.
    /// Default: 1 scene / 4s. Launch: 3 scenes / 15s, no narration.
    /// Explainer: 3 scenes / 30s, narrated.
    pub fn build_spec(&self, title: &str) -> VideoSpec {
        let (duration, scene_secs, narrate) = match self {
            Self::Default => (4u32, 4u32, true),
            Self::Launch => (15, 5, false),
            Self::Explainer => (30, 10, true),
        };
        let count = duration / scene_secs;
        let scenes: Vec<Scene> = (0..count)
            .map(|i| Scene {
                id: format!("s{}", i + 1),
                duration: scene_secs,
                narration: narrate.then(|| {
                    if count == 1 {
                        title.to_string()
                    } else {
                        format!("{title} — cảnh {}", i + 1)
                    }
                }),
                elements: vec![],
                transition: Some("slide".into()),
                custom: false,
            })
            .collect();
        VideoSpec {
            meta: Meta {
                title: title.to_string(),
                duration,
                fps: 30,
                ratio: Ratio::NineBySixteen,
                lang: "vi".into(),
            },
            style: Style {
                preset: "default".into(),
                palette: vec!["#0F172A".into()],
                font: "Inter".into(),
            },
            scenes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_maps_and_rejects() {
        assert_eq!(Template::from_str("launch").unwrap(), Template::Launch);
        assert_eq!(Template::from_str("EXPLAINER").unwrap(), Template::Explainer);
        assert_eq!(Template::from_str("default").unwrap(), Template::Default);
        assert!(Template::from_str("bogus").is_err());
    }

    #[test]
    fn specs_validate_with_expected_shapes() {
        let d = Template::Default.build_spec("t");
        assert_eq!(d.meta.duration, 4);
        assert_eq!(d.scenes.len(), 1);
        assert!(d.validate().is_ok());

        let l = Template::Launch.build_spec("t");
        assert_eq!(l.meta.duration, 15);
        assert_eq!(l.scenes.len(), 3);
        assert!(l.scenes.iter().all(|s| s.narration.is_none()));
        assert!(l.validate().is_ok());

        let e = Template::Explainer.build_spec("t");
        assert_eq!(e.meta.duration, 30);
        assert_eq!(e.scenes.len(), 3);
        assert!(e.scenes.iter().all(|s| s.narration.is_some()));
        assert!(e.validate().is_ok());
    }

    #[test]
    fn template_specs_mark_no_custom_scenes() {
        for t in [Template::Default, Template::Launch, Template::Explainer] {
            let spec = t.build_spec("t");
            assert!(spec.scenes.iter().all(|s| !s.custom));
        }
    }
}
