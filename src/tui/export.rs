use std::path::PathBuf;

use anyhow::Result;

use crate::{export, model::PromptRecord};

pub fn render_selection(records: &[&PromptRecord]) -> Result<String> {
    export::render_markdown(records)
}

pub fn save_selection(records: &[&PromptRecord]) -> Result<PathBuf> {
    export::write_default_export(records)
}

pub fn diff_lines(record: &PromptRecord) -> Result<Vec<String>> {
    export::diff_lines(record)
}
