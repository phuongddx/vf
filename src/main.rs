mod cli;
mod config;

use clap::Parser;

fn main() {
    let _cli = cli::Cli::parse();
}
