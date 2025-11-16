mod app;
mod cli;
mod config;
mod export;
mod indexing;
mod model;
mod providers;
mod review;
mod telemetry;
mod tui;
mod worker;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "prompt-sage")]
#[command(version)]
#[command(about = "Prompt Sage TUI tooling", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the Prompt Sage TUI experience
    Tui(cli::tui::TuiArgs),
    /// Run indexing routines for Claude/Codex transcripts
    Index(cli::index::IndexArgs),
    /// Run the feasibility spike that samples prompts and optionally calls Claude/Codex CLIs
    Spike(cli::spike::SpikeArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Tui(args) => cli::tui::run(args),
        Commands::Index(args) => cli::index::run(args),
        Commands::Spike(args) => cli::spike::run(args),
    }
}
