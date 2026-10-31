mod cli;
mod config;
mod ir;
mod llm;

use clap::Parser;

fn main() {
    let _cli = cli::Cli::parse();
}
