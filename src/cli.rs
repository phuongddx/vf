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
    /// Run the video pipeline for an idea
    Make { idea: String },
    /// Install a skill from a git URL, or list installed skills
    Skills { url: Option<String> },
}
