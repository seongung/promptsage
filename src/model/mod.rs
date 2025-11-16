use std::{
    collections::hash_map::DefaultHasher,
    fmt,
    hash::{Hash, Hasher},
    path::PathBuf,
    str::FromStr,
};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Supported transcript providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Claude,
    Codex,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::Claude => "claude",
            ProviderKind::Codex => "codex",
        }
    }
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ProviderKind {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "claude" => Ok(ProviderKind::Claude),
            "codex" => Ok(ProviderKind::Codex),
            _ => Err("unsupported provider"),
        }
    }
}

/// Logical description of a transcript source (Claude or Codex).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSource {
    pub id: String,
    pub provider: ProviderKind,
    pub root_path: PathBuf,
    pub glob_pattern: String,
    pub enabled: bool,
}

/// Aggregated grouping of prompts for navigation.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSession {
    pub id: String,
    pub provider: ProviderKind,
    pub project: String,
    pub date: NaiveDate,
    pub session_id: Option<String>,
}

/// Review status for a prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptStatus {
    Unreviewed,
    ReviewedOk,
    ReviewedError,
}

impl Default for PromptStatus {
    fn default() -> Self {
        PromptStatus::Unreviewed
    }
}

/// Outcome status of a prompt review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptReviewStatus {
    Pending,
    Completed,
    Failed,
}

/// Lifecycle states for background review jobs.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewJobState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl ReviewJobState {
    #[allow(dead_code)]
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            ReviewJobState::Completed | ReviewJobState::Failed | ReviewJobState::Cancelled
        )
    }
}

/// Error information captured when a review job fails.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewJobError {
    pub message: String,
    pub details: Option<String>,
}

/// Indexed prompt entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptRecord {
    pub id: String,
    pub provider: ProviderKind,
    pub project: String,
    pub date: NaiveDate,
    pub session_id: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
    pub snippet: String,
    pub length: usize,
    pub status: PromptStatus,
    pub raw_text: String,
    pub latest_review: Option<PromptReview>,
}

impl PromptRecord {
    pub fn new(
        provider: ProviderKind,
        project: String,
        date: NaiveDate,
        session_id: Option<String>,
        timestamp: Option<DateTime<Utc>>,
        raw_text: String,
    ) -> Self {
        let snippet = Self::make_snippet(&raw_text);
        let length = raw_text.chars().count();
        let fallback_time = {
            let mut hasher = DefaultHasher::new();
            raw_text.hash(&mut hasher);
            hasher.finish()
        };
        let session_part = session_id.clone().unwrap_or_else(|| "na".to_string());
        let time_part = timestamp
            .map(|ts| ts.timestamp_millis().to_string())
            .unwrap_or_else(|| format!("{fallback_time:x}"));
        let id = format!(
            "{}:{}:{}:{}",
            provider.as_str(),
            project.as_str(),
            session_part,
            time_part
        );

        PromptRecord {
            id,
            provider,
            project,
            date,
            session_id,
            timestamp,
            snippet,
            length,
            status: PromptStatus::Unreviewed,
            raw_text,
            latest_review: None,
        }
    }

    #[allow(dead_code)]
    pub fn is_reviewable(&self) -> bool {
        !self.raw_text.trim().is_empty()
    }

    pub fn make_snippet(text: &str) -> String {
        const MAX: usize = 200;
        let mut snippet: String = text.chars().take(MAX).collect();
        if text.chars().count() > MAX {
            snippet.push('…');
        }
        snippet.replace('\n', " ")
    }
}

/// Stored prompt review result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptReview {
    pub id: String,
    pub prompt_record_id: String,
    pub provider: ProviderKind,
    pub improved_prompt: String,
    pub explanation: String,
    pub score: Option<i32>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub status: PromptReviewStatus,
    pub metadata: Option<Value>,
}

/// Representation of a queued review job.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewJob {
    pub id: String,
    pub prompt_record_id: String,
    pub provider: ProviderKind,
    pub working_dir: Option<PathBuf>,
    pub state: ReviewJobState,
    pub requested_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error: Option<ReviewJobError>,
}
