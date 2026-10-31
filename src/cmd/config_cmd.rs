use crate::config::Config;

pub async fn run() -> anyhow::Result<()> {
    let lang = crate::i18n::Lang::from_env();
    let cfg = Config::load()?.effective();
    println!("config: {}", Config::path().display());
    let spec_target = cfg.routing.spec.clone().unwrap_or_default();
    for (name, p) in &cfg.providers {
        let proto = match p.protocol {
            crate::config::Protocol::OpenAi => "openai",
            crate::config::Protocol::Anthropic => "anthropic",
        };
        let state = if std::env::var(&p.api_key_env).map(|v| !v.is_empty()).unwrap_or(false) {
            "set"
        } else {
            "missing"
        };
        let routing_note = if *name == spec_target { "  [routing.spec]" } else { "" };
        println!(
            "{name}  {proto}  {}  key: {state}: {}{routing_note}",
            p.base_url, p.api_key_env
        );
    }
    let key_env = cfg
        .providers
        .get(&spec_target)
        .map(|p| p.api_key_env.clone())
        .unwrap_or_default();
    println!("{}", crate::i18n::config_hint(lang, &key_env));
    Ok(())
}
