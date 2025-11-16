use anyhow::{Context, Result};
use clap::Args;

use crate::config;
use crate::indexing::{self, IndexSummary};
use crate::telemetry;

#[derive(Args, Debug)]
pub struct IndexArgs {
    /// Print index statistics after scanning transcripts
    #[arg(long)]
    pub stats: bool,
}

pub fn run(args: IndexArgs) -> Result<()> {
    let cfg = config::load().context("failed to load configuration")?;
    if cfg.sources.is_empty() {
        println!("No prompt sources configured. Edit ~/.config/prompt-sage/config.toml first.");
        return Ok(());
    }

    let outcome = indexing::index_sources(&cfg.sources)?;

    if args.stats {
        print_summary(&outcome.summary);
    } else {
        println!(
            "Indexed {} prompts across {} files.",
            outcome.summary.total_records, outcome.summary.total_files
        );
    }

    if !outcome.errors.is_empty() {
        eprintln!(
            "Completed with {} warnings. Showing the first {}:",
            outcome.errors.len(),
            outcome.errors.len().min(5)
        );
        for err in outcome.errors.iter().take(5) {
            eprintln!("  - {} :: {}", err.path.display(), err.message);
        }
    }

    telemetry::emit_index(
        "index_complete",
        "index command finished",
        outcome.summary.total_records,
        outcome.summary.total_files,
    );

    Ok(())
}

fn print_summary(summary: &IndexSummary) {
    println!(
        "Total prompts: {} ({} files)",
        summary.total_records, summary.total_files
    );
    for (provider, stats) in &summary.per_provider {
        println!(
            "  {}: {} prompts across {} files",
            provider, stats.prompt_count, stats.files_indexed
        );
        for (project, count) in &stats.projects {
            println!("    - {project}: {count}");
        }
    }
}
