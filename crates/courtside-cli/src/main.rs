mod export;
mod validate;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use dvw_codec::Dvw;

use validate::Severity;

#[derive(Parser)]
#[command(name = "courtside", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check a scouting file for errors. Exits 1 if any error is found.
    Validate { file: PathBuf },
    /// Convert a .dvw to .json (openvolley plays layout) or re-save as .dvw
    Convert { input: PathBuf, output: PathBuf },
    /// Print match statistics
    Stats {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn read(path: &Path) -> anyhow::Result<Vec<u8>> {
    std::fs::read(path).with_context(|| format!("cannot read {}", path.display()))
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}

fn run(command: Command) -> anyhow::Result<ExitCode> {
    match command {
        Command::Validate { file } => {
            let issues = validate::validate(&file, &read(&file)?);
            for i in &issues {
                println!("{}:{i}", file.display());
            }
            let errors = issues
                .iter()
                .filter(|i| i.severity == Severity::Error)
                .count();
            eprintln!(
                "{}: {errors} error(s), {} warning(s)",
                file.display(),
                issues.len() - errors
            );
            Ok(if errors > 0 {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            })
        }
        Command::Convert { input, output } => {
            if extension(&input) != "dvw" {
                bail!("only .dvw input is supported so far");
            }
            let dvw = Dvw::parse(&read(&input)?)
                .with_context(|| format!("cannot parse {}", input.display()))?;
            let bytes = match extension(&output).as_str() {
                "json" => {
                    let mut out = serde_json::to_vec_pretty(&export::to_json(&dvw))?;
                    out.push(b'\n');
                    out
                }
                "dvw" => dvw.to_bytes()?,
                other => bail!("cannot convert to .{other}; use .json or .dvw"),
            };
            if output.exists() && std::fs::canonicalize(&output)? == std::fs::canonicalize(&input)?
            {
                bail!("output is the same file as input");
            }
            std::fs::write(&output, bytes)
                .with_context(|| format!("cannot write {}", output.display()))?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Stats { .. } => bail!("stats needs the match engine; coming in a later release"),
    }
}
