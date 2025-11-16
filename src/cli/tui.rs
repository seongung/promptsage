use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use dirs;
use which::which;

use crate::{
    app::state::AppState,
    config, indexing,
    tui::app as tui_app,
    worker::review_queue::{self, ProviderBinary, ReviewWorkerConfig},
};

#[derive(Args, Debug, Default)]
pub struct TuiArgs {
    /// Optional project name to pre-select when the TUI launches
    #[arg(long)]
    pub project: Option<String>,
    /// Override path to the Claude CLI binary
    #[arg(long)]
    pub claude_bin: Option<PathBuf>,
    /// Override path to the Codex CLI binary
    #[arg(long)]
    pub codex_bin: Option<PathBuf>,
}

pub fn run(args: TuiArgs) -> Result<()> {
    let cfg = config::load().context("failed to load configuration")?;
    if cfg.sources.is_empty() {
        println!("No prompt sources configured. Update ~/.config/prompt-sage/config.toml first.");
        return Ok(());
    }

    let outcome = indexing::index_sources(&cfg.sources)?;
    if outcome.records.is_empty() {
        println!("No prompts found for the configured sources.");
        return Ok(());
    }

    let mut filters = config::filters_to_state(&cfg.filters);
    if let Some(project) = args.project.clone() {
        filters.set_project(Some(project));
    }
    let mut app_state = AppState::with_filters(outcome.records, filters);
    app_state.auto_batch_mut().last_completed_ids = cfg.auto_review.last_completed_ids.clone();
    let worker = build_worker_handle(&args)?;
    let final_filters = tui_app::run(
        app_state,
        Some(worker),
        cfg.auto_review.clone(),
        cfg.path.clone(),
    )?;
    cfg.persist_filters(&final_filters)?;
    Ok(())
}

fn build_worker_handle(args: &TuiArgs) -> Result<review_queue::ReviewQueueHandle> {
    let mut worker_config = ReviewWorkerConfig::with_constitution(load_constitution());
    worker_config.claude = detect_binary("claude", args.claude_bin.as_ref());
    worker_config.codex = detect_binary("codex", args.codex_bin.as_ref());
    review_queue::spawn_worker(worker_config)
}

fn load_constitution() -> String {
    const DEFAULT_PATH: &str = ".specify/memory/constitution.md";
    const FALLBACK: &str = "You are Prompt Sage, a coach that rewrites prompts to be clearer, safer, and auditable. Respond with JSON matching the Prompt Review contract.";
    fs::read_to_string(DEFAULT_PATH).unwrap_or_else(|_| FALLBACK.to_string())
}

fn detect_binary(name: &str, override_path: Option<&PathBuf>) -> Option<ProviderBinary> {
    if let Some(path) = override_path {
        if path.exists() {
            return Some(ProviderBinary {
                path: path.to_path_buf(),
            });
        }
    }

    if let Ok(path) = which(name) {
        return Some(ProviderBinary { path });
    }

    default_binary_paths(name)
        .into_iter()
        .find(|candidate| candidate.exists())
        .map(|path| ProviderBinary { path })
}

fn default_binary_paths(name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if name == "claude" {
        if let Some(mut home) = dirs::home_dir() {
            home.push(".claude/local/claude");
            paths.push(home);
        }
    }
    paths
}
