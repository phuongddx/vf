mod cli;
mod config;
mod ir;

use clap::Parser;

fn main() {
    let _cli = cli::Cli::parse();
}
