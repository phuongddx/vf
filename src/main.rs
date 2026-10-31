mod cli;
mod cmd;
mod config;
mod engine;
mod exec;
mod i18n;
mod ir;
mod llm;
mod runtime;
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
        cli::Command::Make {
            idea,
            template,
            plan,
            max_cost,
        } => cmd::make::run(&idea, template.as_deref(), plan, max_cost).await,
        cli::Command::Skills { url } => cmd::skills_cmd::run(url).await,
        cli::Command::Doctor => cmd::doctor::run().await,
        cli::Command::Status { project } => cmd::status::run(&project).await,
    };
    if let Err(e) = result {
        eprintln!("vf: {e:#}");
        std::process::exit(1);
    }
}
