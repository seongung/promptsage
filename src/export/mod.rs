use std::{fs, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;

use crate::model::{PromptRecord, PromptReviewStatus};

pub fn render_markdown(records: &[&PromptRecord]) -> Result<String> {
    let mut sections = Vec::new();
    for record in records {
        let review = match record
            .latest_review
            .as_ref()
            .filter(|r| r.status == PromptReviewStatus::Completed)
        {
            Some(review) => review,
            None => continue,
        };
        let mut section = String::new();
        section.push_str(&format!(
            "## {} · {} · {}\n\n",
            record.project, record.date, record.provider
        ));
        section.push_str("### Original\n```\n");
        section.push_str(record.raw_text.trim_end());
        section.push_str("\n```\n\n");
        section.push_str("### Improved\n```\n");
        section.push_str(review.improved_prompt.trim_end());
        section.push_str("\n```\n\n");
        if !review.explanation.trim().is_empty() {
            section.push_str("### Explanation\n");
            section.push_str(review.explanation.trim());
            section.push_str("\n\n");
        }
        sections.push(section);
    }

    if sections.is_empty() {
        return Err(anyhow!(
            "No completed reviews available to export. Run a review first."
        ));
    }

    Ok(sections.join("\n"))
}

pub fn write_default_export(records: &[&PromptRecord]) -> Result<PathBuf> {
    let content = render_markdown(records)?;
    let path = default_export_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("unable to create export dir {}", parent.display()))?;
    }
    fs::write(&path, content)
        .with_context(|| format!("failed to write export file {}", path.display()))?;
    Ok(path)
}

pub fn default_export_path() -> Result<PathBuf> {
    let mut dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("prompt-sage");
    dir.push("exports");
    fs::create_dir_all(&dir)
        .with_context(|| format!("unable to prepare export directory {}", dir.display()))?;
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    dir.push(format!("prompt-sage-export-{timestamp}.md"));
    Ok(dir)
}

pub fn diff_lines(record: &PromptRecord) -> Result<Vec<String>> {
    let review = record
        .latest_review
        .as_ref()
        .filter(|r| r.status == PromptReviewStatus::Completed)
        .ok_or_else(|| anyhow!("No completed review available for diff"))?;
    let mut lines = Vec::new();
    lines.push(format!("Diff: {} ({})", record.project, record.date));
    lines.push(String::new());
    lines.push("Original:".into());
    for line in record.raw_text.lines() {
        lines.push(format!("- {line}"));
    }
    lines.push(String::new());
    lines.push("Improved:".into());
    for line in review.improved_prompt.lines() {
        lines.push(format!("+ {line}"));
    }
    if !review.explanation.trim().is_empty() {
        lines.push(String::new());
        lines.push("Explanation:".into());
        for line in review.explanation.lines() {
            lines.push(format!("> {line}"));
        }
    }
    Ok(lines)
}
