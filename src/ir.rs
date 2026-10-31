use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoSpec {
    pub meta: Meta,
    pub style: Style,
    pub scenes: Vec<Scene>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    pub title: String,
    pub duration: u32,
    pub fps: u8,
    pub ratio: Ratio,
    pub lang: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ratio {
    #[serde(rename = "9:16")]
    NineBySixteen,
    #[serde(rename = "16:9")]
    SixteenByNine,
    #[serde(rename = "1:1")]
    Square,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Style {
    pub preset: String,
    pub palette: Vec<String>,
    pub font: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub duration: u32,
    #[serde(default)]
    pub narration: Option<String>,
    #[serde(default)]
    pub elements: Vec<Element>,
    #[serde(default)]
    pub transition: Option<String>,
    #[serde(default)]
    pub custom: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Element {
    Text {
        content: String,
        motion: Option<String>,
    },
    Image {
        src: String,
        motion: Option<String>,
    },
}

impl VideoSpec {
    pub fn from_json_str(s: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(s)?)
    }

    pub fn to_json_string(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.scenes.is_empty() {
            return Err("spec must contain at least one scene".into());
        }
        let sum: u32 = self.scenes.iter().map(|s| s.duration).sum();
        if sum != self.meta.duration {
            return Err(format!(
                "meta.duration ({}) != sum of scene durations ({})",
                self.meta.duration, sum
            ));
        }
        if self.meta.fps == 0 {
            return Err("fps must be > 0".into());
        }
        Ok(())
    }

    /// Minimal valid spec for pipeline runs before LLM codegen lands (Plan 3).
    pub fn demo(title: &str) -> Self {
        Self {
            meta: Meta {
                title: title.to_string(),
                duration: 4,
                fps: 30,
                ratio: Ratio::NineBySixteen,
                lang: "vi".into(),
            },
            style: Style {
                preset: "default".into(),
                palette: vec!["#0F172A".into()],
                font: "Inter".into(),
            },
            scenes: vec![Scene {
                id: "s1".into(),
                duration: 4,
                narration: Some(title.to_string()),
                elements: vec![],
                transition: None,
                custom: false,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"{
      "meta": { "title": "Demo", "duration": 4, "fps": 30, "ratio": "9:16", "lang": "vi" },
      "style": { "preset": "mg-07", "palette": ["#0F172A"], "font": "Inter" },
      "scenes": [
        { "id": "s1", "duration": 4, "narration": "Đói giữa đêm?",
          "elements": [
            { "type": "text", "content": "Đói giữa đêm?", "motion": "pop-in" },
            { "type": "image", "src": "assets/burger.png", "motion": "float" }
          ],
          "transition": "slide" }
      ]
    }"##;

    #[test]
    fn parses_and_round_trips() {
        let spec = VideoSpec::from_json_str(SAMPLE).unwrap();
        assert_eq!(spec.scenes.len(), 1);
        assert_eq!(spec.meta.ratio, Ratio::NineBySixteen);
        let json = spec.to_json_string().unwrap();
        let back = VideoSpec::from_json_str(&json).unwrap();
        assert_eq!(back, spec);
    }

    #[test]
    fn validate_rejects_empty_scenes_and_mismatched_duration() {
        let mut spec = VideoSpec::from_json_str(SAMPLE).unwrap();
        assert!(spec.validate().is_ok());
        spec.scenes.clear();
        assert!(spec.validate().is_err());
        let mut spec2 = VideoSpec::from_json_str(SAMPLE).unwrap();
        spec2.meta.duration = 9; // scenes sum to 4
        assert!(spec2.validate().is_err());
    }

    #[test]
    fn demo_validates() {
        let spec = VideoSpec::demo("hello");
        assert!(spec.validate().is_ok());
        assert_eq!(spec.meta.title, "hello");
    }

    #[test]
    fn custom_field_defaults_false_and_round_trips() {
        // Old spec without the field still parses; defaults to false.
        let old = VideoSpec::from_json_str(SAMPLE).unwrap();
        assert!(!old.scenes[0].custom);
        let mut s = VideoSpec::from_json_str(SAMPLE).unwrap();
        s.scenes[0].custom = true;
        let json = s.to_json_string().unwrap();
        let back = VideoSpec::from_json_str(&json).unwrap();
        assert!(back.scenes[0].custom);
    }
}
