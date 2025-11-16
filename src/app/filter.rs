use chrono::NaiveDate;

use crate::model::{PromptRecord, PromptStatus, ProviderKind};

#[derive(Debug, Clone)]
pub struct FilterState {
    provider: Option<ProviderKind>,
    project: Option<String>,
    text: Option<String>,
    status: Option<PromptStatus>,
    date_start: Option<NaiveDate>,
    date_end: Option<NaiveDate>,
    hide_reviewed: bool,
}

impl Default for FilterState {
    fn default() -> Self {
        FilterState {
            provider: None,
            project: None,
            text: None,
            status: None,
            date_start: None,
            date_end: None,
            hide_reviewed: false,
        }
    }
}

impl FilterState {
    pub fn provider(&self) -> Option<ProviderKind> {
        self.provider
    }

    pub fn project(&self) -> Option<&str> {
        self.project.as_deref()
    }

    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    pub fn status(&self) -> Option<PromptStatus> {
        self.status
    }

    pub fn date_start(&self) -> Option<NaiveDate> {
        self.date_start
    }

    pub fn date_end(&self) -> Option<NaiveDate> {
        self.date_end
    }

    pub fn hide_reviewed(&self) -> bool {
        self.hide_reviewed
    }

    pub fn set_provider(&mut self, provider: Option<ProviderKind>) {
        self.provider = provider;
    }

    pub fn set_project<S: Into<String>>(&mut self, project: Option<S>) {
        self.project = project.map(|s| s.into());
    }

    pub fn set_text(&mut self, text: Option<String>) {
        self.text = text.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
    }

    pub fn set_status(&mut self, status: Option<PromptStatus>) {
        self.status = status;
    }

    pub fn set_date_range(&mut self, start: Option<NaiveDate>, end: Option<NaiveDate>) {
        self.date_start = start;
        self.date_end = end;
    }

    pub fn set_hide_reviewed(&mut self, hide: bool) {
        self.hide_reviewed = hide;
    }

    pub fn matches(&self, record: &PromptRecord) -> bool {
        if let Some(provider) = self.provider {
            if record.provider != provider {
                return false;
            }
        }

        if let Some(project) = &self.project {
            if !record.project.eq_ignore_ascii_case(project) {
                return false;
            }
        }

        if let Some(status) = self.status {
            if record.status != status {
                return false;
            }
        }

        if self.hide_reviewed && matches!(record.status, PromptStatus::ReviewedOk) {
            return false;
        }

        if let Some(start) = self.date_start {
            if record.date < start {
                return false;
            }
        }

        if let Some(end) = self.date_end {
            if record.date > end {
                return false;
            }
        }

        if let Some(query) = &self.text {
            if !contains_case_insensitive(&record.raw_text, query)
                && !contains_case_insensitive(&record.snippet, query)
            {
                return false;
            }
        }

        true
    }
}

fn contains_case_insensitive(haystack: &str, needle: &str) -> bool {
    let needle_lower = needle.to_ascii_lowercase();
    haystack
        .to_ascii_lowercase()
        .contains(needle_lower.as_str())
}
