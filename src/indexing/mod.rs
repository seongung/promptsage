pub mod claude;
pub mod codex;
pub mod util;

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Result;

use crate::model::{PromptRecord, PromptSource, ProviderKind};

use self::claude::ClaudeIndexer;
use self::codex::CodexIndexer;

#[derive(Debug, Default)]
pub struct IndexOutcome {
    pub records: Vec<PromptRecord>,
    pub summary: IndexSummary,
    pub errors: Vec<IndexError>,
}

#[derive(Debug, Default)]
pub struct IndexSummary {
    pub total_records: usize,
    pub total_files: usize,
    pub per_provider: BTreeMap<ProviderKind, ProviderSummary>,
}

#[derive(Debug, Default)]
pub struct ProviderSummary {
    pub prompt_count: usize,
    pub files_indexed: usize,
    pub projects: BTreeMap<String, usize>,
}

#[derive(Debug, Clone)]
pub struct IndexError {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct ProviderIndex {
    pub records: Vec<PromptRecord>,
    pub errors: Vec<IndexError>,
    pub files_indexed: usize,
}

pub fn index_sources(sources: &[PromptSource]) -> Result<IndexOutcome> {
    let mut outcome = IndexOutcome::default();

    for source in sources {
        if !source.enabled {
            continue;
        }

        let chunk = match source.provider {
            ProviderKind::Claude => ClaudeIndexer::index(source)?,
            ProviderKind::Codex => CodexIndexer::index(source)?,
        };

        outcome.summary.total_files += chunk.files_indexed;
        outcome.summary.total_records += chunk.records.len();
        {
            let entry = outcome
                .summary
                .per_provider
                .entry(source.provider)
                .or_insert_with(ProviderSummary::default);
            entry.files_indexed += chunk.files_indexed;
        }
        merge_summary(&mut outcome.summary, &chunk.records);
        outcome.errors.extend(chunk.errors);
        outcome.records.extend(chunk.records);
    }

    outcome.records.sort_by(|a, b| {
        b.timestamp
            .cmp(&a.timestamp)
            .then_with(|| b.date.cmp(&a.date))
            .then_with(|| a.project.cmp(&b.project))
    });

    Ok(outcome)
}

fn merge_summary(summary: &mut IndexSummary, records: &[PromptRecord]) {
    for record in records {
        let entry = summary
            .per_provider
            .entry(record.provider)
            .or_insert_with(ProviderSummary::default);
        entry.prompt_count += 1;
        *entry.projects.entry(record.project.clone()).or_insert(0) += 1;
    }
}
