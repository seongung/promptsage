use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf, MAIN_SEPARATOR},
};

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    app::filter::FilterState,
    model::{PromptSource, ProviderKind},
};

const CONFIG_FILENAME: &str = "config.toml";
const CONFIG_DIR: &str = "prompt-sage";
const DEFAULT_CLAUDE_ROOT: &str = "~/.claude/projects";
const DEFAULT_CODEX_ROOT: &str = "~/.codex/sessions";

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub path: PathBuf,
    pub sources: Vec<PromptSource>,
    pub filters: FilterDefaults,
    pub safety: SafetyConfig,
    pub auto_review: AutoReviewConfig,
}
impl AppConfig {
    pub fn persist_filters(&self, filters: &FilterState) -> Result<()> {
        persist_filters_to(&self.path, filters)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct FilterDefaults {
    pub provider: Option<ProviderKind>,
    pub project: Option<String>,
    pub text: Option<String>,
    pub date_start: Option<NaiveDate>,
    pub date_end: Option<NaiveDate>,
    pub hide_reviewed: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SafetyConfig {
    pub read_only: bool,
    pub confirm_reviews: bool,
    pub confirm_exports: bool,
    pub allow_exports: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        SafetyConfig {
            read_only: true,
            confirm_reviews: true,
            confirm_exports: true,
            allow_exports: true,
        }
    }
}

pub fn load() -> Result<AppConfig> {
    let path = default_config_path();
    let raw = read_raw_config(&path)?;
    build_config(raw, path)
}

pub fn default_config_path() -> PathBuf {
    let mut base = dirs::config_dir()
        .or_else(|| {
            dirs::home_dir().map(|mut home| {
                home.push(".config");
                home
            })
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.push(CONFIG_DIR);
    base.push(CONFIG_FILENAME);
    base
}

pub fn filters_to_state(defaults: &FilterDefaults) -> FilterState {
    let mut state = FilterState::default();
    state.set_provider(defaults.provider);
    state.set_project(defaults.project.clone());
    state.set_text(defaults.text.clone());
    state.set_date_range(defaults.date_start, defaults.date_end);
    state.set_hide_reviewed(defaults.hide_reviewed);
    state
}

fn build_config(raw: RawConfig, path: PathBuf) -> Result<AppConfig> {
    let mut sources = if raw.sources.is_empty() {
        default_sources()
    } else {
        raw.sources
            .into_iter()
            .map(|entry| entry.into_source())
            .collect::<Result<Vec<_>>>()?
    };
    sources.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(AppConfig {
        path,
        sources,
        filters: FilterDefaults::from_raw(raw.filters)?,
        safety: SafetyConfig::from_raw(raw.safety),
        auto_review: AutoReviewConfig::from_raw(raw.auto_review),
    })
}

pub fn persist_filters_to(path: &Path, filters: &FilterState) -> Result<()> {
    let mut raw = read_raw_config(path)?;
    raw.filters = RawFilters::from_filter_state(filters);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(&raw)
        .with_context(|| format!("failed to serialize configuration for {}", path.display()))?;
    fs::write(path, contents)
        .with_context(|| format!("failed to write configuration at {}", path.display()))?;
    Ok(())
}

pub fn persist_auto_review_state(
    path: &Path,
    completed_ids: &[String],
    finished_at: Option<DateTime<Utc>>,
) -> Result<()> {
    let mut raw = read_raw_config(path)?;
    raw.auto_review.last_completed_ids = completed_ids.to_vec();
    raw.auto_review.last_completed_at = finished_at;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(&raw)
        .with_context(|| format!("failed to serialize configuration for {}", path.display()))?;
    fs::write(path, contents)
        .with_context(|| format!("failed to write configuration at {}", path.display()))?;
    Ok(())
}

fn read_raw_config(path: &Path) -> Result<RawConfig> {
    match fs::read_to_string(path) {
        Ok(contents) => {
            let trimmed = contents.trim();
            if trimmed.is_empty() {
                Ok(RawConfig::default())
            } else {
                toml::from_str(trimmed)
                    .with_context(|| format!("failed to parse configuration at {}", path.display()))
            }
        }
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(RawConfig::default()),
        Err(err) => {
            Err(err).with_context(|| format!("failed to read configuration at {}", path.display()))
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct RawConfig {
    #[serde(default)]
    sources: Vec<RawSource>,
    #[serde(default)]
    filters: RawFilters,
    #[serde(default)]
    safety: RawSafety,
    #[serde(default)]
    auto_review: RawAutoReview,
}

impl Default for RawConfig {
    fn default() -> Self {
        RawConfig {
            sources: Vec::new(),
            filters: RawFilters::default(),
            safety: RawSafety::default(),
            auto_review: RawAutoReview::default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct RawSource {
    provider: ProviderKind,
    #[serde(default)]
    root_path: Option<String>,
    #[serde(default)]
    glob: Option<String>,
    #[serde(default = "bool_true")]
    enabled: bool,
    #[serde(default)]
    id: Option<String>,
}

impl RawSource {
    fn into_source(self) -> Result<PromptSource> {
        let provider = self.provider;
        let root_input = self
            .root_path
            .unwrap_or_else(|| default_root(provider).to_string());
        let root_path = expand_home(&root_input);
        let glob_pattern = self
            .glob
            .map(|pattern| expand_home_pattern(&pattern))
            .unwrap_or_else(|| default_glob(&root_path));
        let id = self
            .id
            .unwrap_or_else(|| format!("{}:{}", provider.as_str(), root_path.display()));
        Ok(PromptSource {
            id,
            provider,
            root_path,
            glob_pattern,
            enabled: self.enabled,
        })
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct RawFilters {
    provider: Option<ProviderKind>,
    project: Option<String>,
    text: Option<String>,
    date_start: Option<String>,
    date_end: Option<String>,
    #[serde(default)]
    hide_reviewed: bool,
}

impl FilterDefaults {
    fn from_raw(raw: RawFilters) -> Result<Self> {
        Ok(FilterDefaults {
            provider: raw.provider,
            project: raw.project,
            text: raw.text,
            date_start: parse_date(&raw.date_start)?,
            date_end: parse_date(&raw.date_end)?,
            hide_reviewed: raw.hide_reviewed,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct RawSafety {
    #[serde(default = "bool_true")]
    read_only: bool,
    #[serde(default = "bool_true")]
    confirm_reviews: bool,
    #[serde(default = "bool_true")]
    confirm_exports: bool,
    #[serde(default = "bool_true")]
    allow_exports: bool,
}

impl Default for RawSafety {
    fn default() -> Self {
        RawSafety {
            read_only: true,
            confirm_reviews: true,
            confirm_exports: true,
            allow_exports: true,
        }
    }
}

impl SafetyConfig {
    fn from_raw(raw: RawSafety) -> Self {
        SafetyConfig {
            read_only: raw.read_only,
            confirm_reviews: raw.confirm_reviews,
            confirm_exports: raw.confirm_exports,
            allow_exports: raw.allow_exports,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct RawAutoReview {
    #[serde(default = "bool_true")]
    enabled: bool,
    #[serde(default = "default_batch_size")]
    batch_size: usize,
    #[serde(default)]
    overlay: OverlayMode,
    #[serde(default)]
    last_completed_ids: Vec<String>,
    #[serde(default)]
    last_completed_at: Option<DateTime<Utc>>,
}

impl Default for RawAutoReview {
    fn default() -> Self {
        RawAutoReview {
            enabled: bool_true(),
            batch_size: default_batch_size(),
            overlay: OverlayMode::Auto,
            last_completed_ids: Vec::new(),
            last_completed_at: None,
        }
    }
}

impl AutoReviewConfig {
    fn from_raw(raw: RawAutoReview) -> Self {
        AutoReviewConfig {
            enabled: raw.enabled,
            batch_size: raw.batch_size.max(1),
            overlay: raw.overlay,
            last_completed_ids: raw.last_completed_ids,
            last_completed_at: raw.last_completed_at,
        }
    }
}

fn parse_date(input: &Option<String>) -> Result<Option<NaiveDate>> {
    if let Some(value) = input {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        return NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
            .map(Some)
            .with_context(|| format!("invalid date '{trimmed}' (expected YYYY-MM-DD)"));
    }
    Ok(None)
}

fn expand_home(input: &str) -> PathBuf {
    if let Some(stripped) = input.strip_prefix("~/") {
        if let Some(mut home) = dirs::home_dir() {
            home.push(stripped);
            return home;
        }
    }
    PathBuf::from(input)
}

fn expand_home_pattern(input: &str) -> String {
    if let Some(stripped) = input.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            let mut base = home.to_string_lossy().into_owned();
            if !base.ends_with(MAIN_SEPARATOR) {
                base.push(MAIN_SEPARATOR);
            }
            base.push_str(stripped);
            return base;
        }
    }
    input.to_string()
}

fn default_sources() -> Vec<PromptSource> {
    let claude_root = expand_home(DEFAULT_CLAUDE_ROOT);
    let claude_glob = default_glob(&claude_root);
    let codex_root = expand_home(DEFAULT_CODEX_ROOT);
    let codex_glob = default_glob(&codex_root);
    vec![
        PromptSource {
            id: format!(
                "{}:{}",
                ProviderKind::Claude.as_str(),
                claude_root.display()
            ),
            provider: ProviderKind::Claude,
            root_path: claude_root,
            glob_pattern: claude_glob,
            enabled: true,
        },
        PromptSource {
            id: format!("{}:{}", ProviderKind::Codex.as_str(), codex_root.display()),
            provider: ProviderKind::Codex,
            root_path: codex_root,
            glob_pattern: codex_glob,
            enabled: true,
        },
    ]
}

fn default_glob(root: &Path) -> String {
    let mut buf = root.to_path_buf();
    buf.push("**");
    buf.push("*.jsonl");
    buf.to_string_lossy().into_owned()
}

fn default_root(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Claude => DEFAULT_CLAUDE_ROOT,
        ProviderKind::Codex => DEFAULT_CODEX_ROOT,
    }
}

const fn bool_true() -> bool {
    true
}

const fn default_batch_size() -> usize {
    10
}
impl RawFilters {
    fn from_filter_state(state: &FilterState) -> Self {
        RawFilters {
            provider: state.provider(),
            project: state.project().map(|s| s.to_string()),
            text: state.text().map(|s| s.to_string()),
            date_start: state.date_start().map(|d| d.to_string()),
            date_end: state.date_end().map(|d| d.to_string()),
            hide_reviewed: state.hide_reviewed(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct AutoReviewConfig {
    pub enabled: bool,
    pub batch_size: usize,
    pub overlay: OverlayMode,
    pub last_completed_ids: Vec<String>,
    pub last_completed_at: Option<DateTime<Utc>>,
}

impl Default for AutoReviewConfig {
    fn default() -> Self {
        AutoReviewConfig {
            enabled: true,
            batch_size: default_batch_size(),
            overlay: OverlayMode::Auto,
            last_completed_ids: Vec::new(),
            last_completed_at: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OverlayMode {
    Auto,
    Always,
    Never,
}

impl Default for OverlayMode {
    fn default() -> Self {
        OverlayMode::Auto
    }
}
