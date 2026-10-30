use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub providers: BTreeMap<String, Provider>,
    #[serde(default)]
    pub routing: Routing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub protocol: Protocol,
    /// OpenAI: includes `/v1`. Anthropic: host root (`/v1/messages` is appended).
    pub base_url: String,
    pub api_key_env: String,
    pub model: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    OpenAi,
    Anthropic,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Routing {
    pub spec: Option<String>,
    pub codegen: Option<String>,
}

impl Config {
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("vf")
            .join("config.toml")
    }

    pub fn load() -> anyhow::Result<Self> {
        Self::load_from(&Self::path())
    }

    pub fn load_from(p: &Path) -> anyhow::Result<Self> {
        match std::fs::read_to_string(p) {
            Ok(s) => Ok(toml::from_str(&s)?),
            Err(_) => Ok(Self::default()), // missing file = fresh install
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let p = Self::path();
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&p, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trip() {
        let mut cfg = Config::default();
        cfg.providers.insert(
            "glm-coding".into(),
            Provider {
                protocol: Protocol::Anthropic,
                base_url: "https://api.example.com".into(),
                api_key_env: "GLM_API_KEY".into(),
                model: "glm-4.7".into(),
            },
        );
        let s = toml::to_string(&cfg).unwrap();
        let back: Config = toml::from_str(&s).unwrap();
        assert_eq!(back.providers["glm-coding"].api_key_env, "GLM_API_KEY");
        assert_eq!(back.providers["glm-coding"].protocol, Protocol::Anthropic);
    }

    #[test]
    fn missing_file_loads_default() {
        let cfg = Config::load_from(&std::path::PathBuf::from("/nonexistent/config.toml")).unwrap();
        assert!(cfg.providers.is_empty());
    }
}
