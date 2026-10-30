use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "vf", version, about = "Text to product video, one command.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Show configuration
    Config,
    /// Run the pipeline for an idea (stub in Plan 1)
    Make { idea: String },
}
