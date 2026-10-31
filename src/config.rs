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
        if let Ok(p) = std::env::var("VF_CONFIG") {
            if !p.is_empty() {
                return PathBuf::from(p);
            }
        }
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

    /// Built-in Z.ai provider profiles. File config overrides per provider key.
    pub fn builtin() -> Self {
        let mut providers = BTreeMap::new();
        providers.insert(
            "glm-coding".into(),
            Provider {
                protocol: Protocol::Anthropic,
                base_url: "https://api.z.ai/api/anthropic".into(),
                api_key_env: "ZAI_CODING_API_KEY".into(),
                model: "glm-5.2".into(),
            },
        );
        providers.insert(
            "glm".into(),
            Provider {
                protocol: Protocol::OpenAi,
                base_url: "https://api.z.ai/api/paas/v4".into(),
                api_key_env: "ZAI_API_KEY".into(),
                model: "glm-5.2".into(),
            },
        );
        Self { providers, routing: Routing::default() }
    }

    /// Merge self (file config) over built-ins; default routing to glm-coding.
    pub fn effective(self) -> Self {
        let mut providers = Self::builtin().providers;
        for (k, v) in self.providers {
            providers.insert(k, v);
        }
        let routing = Routing {
            spec: self.routing.spec.or_else(|| Some("glm-coding".into())),
            codegen: self.routing.codegen.or_else(|| Some("glm-coding".into())),
        };
        Self { providers, routing }
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

    #[test]
    fn builtin_profiles_match_spec() {
        let b = Config::builtin();
        let coding = &b.providers["glm-coding"];
        assert_eq!(coding.protocol, Protocol::Anthropic);
        assert_eq!(coding.base_url, "https://api.z.ai/api/anthropic");
        assert_eq!(coding.api_key_env, "ZAI_CODING_API_KEY");
        assert_eq!(coding.model, "glm-5.2");
        let std = &b.providers["glm"];
        assert_eq!(std.protocol, Protocol::OpenAi);
        assert_eq!(std.base_url, "https://api.z.ai/api/paas/v4");
        assert_eq!(std.api_key_env, "ZAI_API_KEY");
    }

    #[test]
    fn effective_overlays_file_over_builtin_and_defaults_routing() {
        let mut file = Config::default();
        file.providers.insert(
            "glm".into(),
            Provider {
                protocol: Protocol::OpenAi,
                base_url: "http://localhost:1/v4".into(),
                api_key_env: "LOCAL_KEY".into(),
                model: "m".into(),
            },
        );
        let eff = file.effective();
        assert_eq!(eff.providers["glm"].base_url, "http://localhost:1/v4");
        assert!(eff.providers.contains_key("glm-coding"));
        assert_eq!(eff.routing.spec.as_deref(), Some("glm-coding"));
        assert_eq!(eff.routing.codegen.as_deref(), Some("glm-coding"));
    }

    #[test]
    fn effective_keeps_file_routing() {
        let mut file = Config::default();
        file.routing.spec = Some("glm".into());
        let eff = file.effective();
        assert_eq!(eff.routing.spec.as_deref(), Some("glm"));
        assert_eq!(eff.routing.codegen.as_deref(), Some("glm-coding"));
    }
}
