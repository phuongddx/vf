mod cli;
mod cmd;
mod config;
mod engine;
mod exec;
mod ir;
mod llm;
mod state;

use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = cli::Cli::parse();
    let result = match cli.command {
        cli::Command::Config => cmd::config_cmd::run().await,
        cli::Command::Make { idea } => cmd::make::run(&idea).await,
    };
    if let Err(e) = result {
        eprintln!("vf: {e:#}");
        std::process::exit(1);
    }
}
