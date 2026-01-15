use anyhow::Result;
use clap::Parser;

use tsi::cli::{commands, Cli, Command};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Calculate(args) => commands::calculate(args),
    }
}
