use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::NaiveDate;
use glob::glob;
use serde_json::Value;

use crate::model::{PromptRecord, PromptSource};

use super::util;
use super::{IndexError, ProviderIndex};

pub struct CodexIndexer;

impl CodexIndexer {
    pub fn index(source: &PromptSource) -> Result<ProviderIndex> {
        let mut chunk = ProviderIndex::default();
        let pattern = &source.glob_pattern;
        for entry in glob(pattern).with_context(|| format!("invalid glob {}", pattern))? {
            match entry {
                Ok(path) => {
                    if path.is_file() {
                        match Self::index_file(source, &path) {
                            Ok(mut records) => {
                                chunk.files_indexed += 1;
                                chunk.records.append(&mut records);
                            }
                            Err(err) => chunk.errors.push(IndexError {
                                path: path.clone(),
                                message: err.to_string(),
                            }),
                        }
                    }
                }
                Err(err) => chunk.errors.push(IndexError {
                    path: PathBuf::from(pattern),
                    message: format!("glob error: {err}"),
                }),
            }
        }
        Ok(chunk)
    }

    fn index_file(source: &PromptSource, path: &Path) -> Result<Vec<PromptRecord>> {
        let file = File::open(path)
            .with_context(|| format!("unable to open Codex log {}", path.display()))?;
        let mut reader = BufReader::new(file);
        let fallback_date = util::fallback_date(path);
        let fallback_session = util::session_from_path(path);
        let mut records = Vec::new();
        let mut line = String::new();
        let mut line_no = 0;
        let mut cwd_hint: Option<String> = None;

        loop {
            line.clear();
            let bytes = reader
                .read_line(&mut line)
                .with_context(|| format!("unable to read {}", path.display()))?;
            if bytes == 0 {
                break;
            }
            line_no += 1;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let value: Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(err) => {
                    eprintln!(
                        "warn: invalid Codex JSON in {} line {}: {}",
                        path.display(),
                        line_no,
                        err
                    );
                    continue;
                }
            };

            if cwd_hint.is_none() {
                cwd_hint = util::extract_cwd(&value);
            }

            if let Some(record) = Self::record_from_value(
                source,
                path,
                &value,
                fallback_date,
                fallback_session.as_ref(),
                cwd_hint.as_deref(),
            ) {
                records.push(record);
            }
        }

        Ok(records)
    }

    fn record_from_value(
        source: &PromptSource,
        path: &Path,
        value: &Value,
        fallback_date: NaiveDate,
        fallback_session: Option<&String>,
        cwd_hint: Option<&str>,
    ) -> Option<PromptRecord> {
        let prompt = util::extract_prompt_text(value)?;
        let timestamp = util::derive_timestamp(value);
        let date = timestamp.map(|ts| ts.date_naive()).unwrap_or(fallback_date);
        let project = util::derive_project(value, path, source, cwd_hint);
        let session = util::derive_session(value, path)
            .or_else(|| fallback_session.map(|raw| raw.to_string()));

        Some(PromptRecord::new(
            source.provider,
            project,
            date,
            session,
            timestamp,
            prompt,
        ))
    }
}
