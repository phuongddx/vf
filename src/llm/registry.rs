use crate::config::{Config, Protocol};
use crate::llm::anthropic::AnthropicClient;
use crate::llm::openai::OpenAiClient;
use crate::llm::LlmClient;
use anyhow::{anyhow, Context};

pub fn build(name: &str, cfg: &Config) -> anyhow::Result<Box<dyn LlmClient>> {
    let p = cfg
        .providers
        .get(name)
        .ok_or_else(|| anyhow!("unknown provider '{name}' — check config.toml [providers]"))?;
    let key = std::env::var(&p.api_key_env).with_context(|| {
        format!(
            "set the {} environment variable (provider '{name}')",
            p.api_key_env
        )
    })?;
    Ok(match p.protocol {
        Protocol::OpenAi => Box::new(OpenAiClient::new(
            p.base_url.clone(),
            key,
            p.model.clone(),
        )),
        Protocol::Anthropic => Box::new(AnthropicClient::new(
            p.base_url.clone(),
            key,
            p.model.clone(),
        )),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Protocol, Provider};

    fn cfg_with(provider: Provider) -> Config {
        let mut cfg = Config::default();
        cfg.providers.insert("p1".into(), provider);
        cfg
    }

    #[test]
    fn unknown_provider_errors() {
        let err = build("nope", &Config::default()).err().unwrap();
        assert!(err.to_string().contains("nope"));
    }

    #[test]
    fn missing_env_key_errors_with_var_name() {
        let cfg = cfg_with(Provider {
            protocol: Protocol::OpenAi,
            base_url: "https://x".into(),
            // Plan used env::remove_var here — unsafe in edition 2024.
            // A never-set variable name achieves the same without unsafe.
            api_key_env: "VF_TEST_KEY_NEVER_SET_ANYWHERE".into(),
            model: "m".into(),
        });
        let err = build("p1", &cfg).err().unwrap();
        assert!(err.to_string().contains("VF_TEST_KEY_NEVER_SET_ANYWHERE"));
    }
}
