mod cli;
mod cmd;
mod config;
mod exec;
mod ir;
mod llm;
mod state;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();
    let result = match cli.command {
        cli::Command::Config => cmd::config_cmd::run(),
        cli::Command::Make { idea } => cmd::make::run(&idea),
    };
    if let Err(e) = result {
        eprintln!("vf: {e:#}");
        std::process::exit(1);
    }
}
