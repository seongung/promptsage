use chrono::NaiveDate;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::filter::FilterState;
use crate::model::{PromptStatus, ProviderKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterInputMode {
    Normal,
    Search,
    Filter,
}

#[derive(Debug, Clone)]
pub struct FilterUiState {
    pub mode: FilterInputMode,
    pub buffer: String,
}

impl Default for FilterUiState {
    fn default() -> Self {
        FilterUiState {
            mode: FilterInputMode::Normal,
            buffer: String::new(),
        }
    }
}

impl FilterUiState {
    pub fn set_mode(&mut self, mode: FilterInputMode) {
        if !matches!(mode, FilterInputMode::Search) {
            self.buffer.clear();
        }
        self.mode = mode;
    }

    pub fn mode(&self) -> FilterInputMode {
        self.mode
    }
}

pub fn render(frame: &mut Frame<'_>, area: Rect, filters: &FilterState, ui: &FilterUiState) {
    let mut lines = Vec::new();
    let summary = format!(
        "Provider: {} · Status: {} · Dates: {} · Text: {} · Hide reviewed: {}",
        provider_display(filters.provider()),
        status_display(filters.status()),
        date_display(filters.date_start(), filters.date_end()),
        filters
            .text()
            .map(|t| format!("\"{}\"", truncate(t, 30)))
            .unwrap_or_else(|| "—".into()),
        if filters.hide_reviewed() { "yes" } else { "no" }
    );

    match ui.mode {
        FilterInputMode::Search => {
            lines.push(Line::from(vec![
                Span::styled("Search (/ to cancel): ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}_", ui.buffer)),
            ]));
        }
        _ => {
            lines.push(Line::from(summary));
            lines.push(Line::from(Span::styled(
                "[/] search  [f] filters (p provider · s status · h hide · c clear)",
                Style::default().fg(Color::Gray),
            )));
        }
    }

    let block = Block::default().title("Filters").borders(Borders::ALL);
    frame.render_widget(Paragraph::new(lines).block(block), area);

    if matches!(ui.mode, FilterInputMode::Filter) {
        render_modal(frame, area, filters);
    }
}

fn render_modal(frame: &mut Frame<'_>, area: Rect, filters: &FilterState) {
    let modal = centered_rect(60, 50, area);
    let mut lines = Vec::new();
    lines.push(Line::from("Filter controls"));
    lines.push(Line::from(format!(
        "Provider: {}",
        provider_display(filters.provider())
    )));
    lines.push(Line::from(format!(
        "Status: {}",
        status_display(filters.status())
    )));
    lines.push(Line::from(format!(
        "Date range: {}",
        date_display(filters.date_start(), filters.date_end())
    )));
    lines.push(Line::from(format!(
        "Hide reviewed: {}",
        if filters.hide_reviewed() { "yes" } else { "no" }
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(
        "Keys: [p] provider · [s] status · [h] hide · [c] clear · [Esc]/[Enter] close",
    ));

    let block = Block::default()
        .title("[f] Filter Controls")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));
    frame.render_widget(Paragraph::new(lines).block(block), modal);
}

fn provider_display(provider: Option<ProviderKind>) -> String {
    provider
        .map(|p| p.to_string())
        .unwrap_or_else(|| "all".into())
}

fn status_display(status: Option<PromptStatus>) -> String {
    match status {
        None => "all".into(),
        Some(PromptStatus::Unreviewed) => "unreviewed".into(),
        Some(PromptStatus::ReviewedOk) => "reviewed-ok".into(),
        Some(PromptStatus::ReviewedError) => "reviewed-error".into(),
    }
}

fn date_display(start: Option<NaiveDate>, end: Option<NaiveDate>) -> String {
    match (start, end) {
        (Some(s), Some(e)) if s == e => s.to_string(),
        (Some(s), Some(e)) => format!("{} → {}", s, e),
        (Some(s), None) => format!("≥ {}", s),
        (None, Some(e)) => format!("≤ {}", e),
        _ => "any".into(),
    }
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let mut truncated: String = value.chars().take(max.saturating_sub(1)).collect();
    truncated.push('…');
    truncated
}

fn centered_rect(width_percent: u16, height_percent: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - height_percent) / 2),
                Constraint::Percentage(height_percent),
                Constraint::Percentage((100 - height_percent) / 2),
            ]
            .as_ref(),
        )
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - width_percent) / 2),
                Constraint::Percentage(width_percent),
                Constraint::Percentage((100 - width_percent) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
