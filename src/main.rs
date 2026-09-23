use anyhow::Result;
use clap::Parser;

mod cli;
mod output;

use cli::args::ColorWhen;
use cli::{commands, Cli, Command};
use output::Look;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let look = Look {
        color: match cli.color {
            ColorWhen::Auto => anstream::ColorChoice::Auto,
            ColorWhen::Always => anstream::ColorChoice::Always,
            ColorWhen::Never => anstream::ColorChoice::Never,
        },
        ascii: cli.ascii,
    };

    match cli.command {
        Command::Calculate(args) => commands::calculate(args, look),
        Command::Optimize(args) => commands::optimize(args, look),
        Command::Engines(args) => commands::engines(args, look),
        Command::Completions(args) => commands::completions(args),
    }
}
