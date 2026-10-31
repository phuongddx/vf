use crate::config::Config;

pub async fn run() -> anyhow::Result<()> {
    let cfg = Config::load()?;
    println!("config: {}", Config::path().display());
    if cfg.providers.is_empty() {
        println!("no providers configured — see docs to add [providers.<name>]");
        return Ok(());
    }
    for (name, p) in &cfg.providers {
        println!(
            "{name}  {}={}  key:{}",
            match p.protocol {
                crate::config::Protocol::OpenAi => "openai",
                crate::config::Protocol::Anthropic => "anthropic",
            },
            p.model,
            p.api_key_env
        );
    }
    Ok(())
}
