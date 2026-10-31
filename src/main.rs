mod cli;
mod cmd;
mod config;
mod engine;
mod exec;
mod ir;
mod llm;
mod skills;
mod state;
mod styles;
mod templates;

use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = cli::Cli::parse();
    let result = match cli.command {
        cli::Command::Config => cmd::config_cmd::run().await,
        cli::Command::Make { idea, template } => {
            cmd::make::run(&idea, template.as_deref()).await
        }
        cli::Command::Skills { url } => cmd::skills_cmd::run(url).await,
    };
    if let Err(e) = result {
        eprintln!("vf: {e:#}");
        std::process::exit(1);
    }
}
