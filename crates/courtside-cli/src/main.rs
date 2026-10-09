use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "courtside", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check a scouting file for errors
    Validate { file: PathBuf },
    /// Convert between .dvw, .vsm and .json
    Convert { input: PathBuf, output: PathBuf },
    /// Print match statistics
    Stats {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Validate { .. } | Command::Convert { .. } | Command::Stats { .. } => {
            anyhow::bail!("not implemented yet")
        }
    }
}
