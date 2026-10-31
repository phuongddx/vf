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
    Make {
        idea: String,
        /// Spec template: default | launch | explainer
        #[arg(long)]
        template: Option<String>,
        /// Print the plan + cost estimate without running
        #[arg(long)]
        plan: bool,
        /// Bail if estimated LLM cost exceeds this (USD)
        #[arg(long)]
        max_cost: Option<f64>,
    },
    /// Install a skill from a git URL, or list installed skills
    Skills { url: Option<String> },
    /// Check toolchain health (git, node, npx, ffmpeg)
    Doctor,
    /// Show pipeline status for a project
    Status { project: String },
}
