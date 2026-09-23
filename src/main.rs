use anyhow::Result;
use clap::Parser;

use tsiolkovsky::cli::{commands, Cli, Command};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Calculate(args) => commands::calculate(args),
        Command::Optimize(args) => commands::optimize(args),
        Command::Engines(args) => commands::engines(args),
        Command::Completions(args) => commands::completions(args),
    }
}
