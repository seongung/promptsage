use std::{
    collections::BTreeMap,
    path::{Component, Path},
    sync::OnceLock,
};

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::{
    app::state::AppState,
    model::{PromptRecord, PromptStatus, ProviderKind},
};

use super::{
    detail,
    filter_bar::{self, FilterUiState},
};

const HEADER_HEIGHT: u16 = 1;
const FILTER_HEIGHT: u16 = 3;
const PROJECTS_HEIGHT: u16 = 2;
const FOOTER_HEIGHT: u16 = 1;
const DETAIL_TARGET_HEIGHT: u16 = 10;
const PROMPTS_AREA_MIN: u16 = 10;
const PROMPTS_AREA_MAX: u16 = 18; // room for ~15 prompt rows once borders/header render

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    Projects,
    Prompts,
    Original,
    Review,
}

impl ActivePane {
    pub fn next(self) -> Self {
        match self {
            ActivePane::Projects => ActivePane::Prompts,
            ActivePane::Prompts => ActivePane::Original,
            ActivePane::Original => ActivePane::Review,
            ActivePane::Review => ActivePane::Projects,
        }
    }
}

pub struct LayoutContext<'a> {
    pub app_state: &'a AppState,
    pub active_pane: ActivePane,
    pub original_scroll: u16,
    pub review_scroll: u16,
    pub status: Option<&'a str>,
    pub filter_ui: FilterUiState,
    pub selection_count: usize,
    pub diff_lines: Option<Vec<String>>,
}

pub fn render(frame: &mut Frame<'_>, ctx: LayoutContext<'_>) {
    let shell = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HEADER_HEIGHT),
            Constraint::Length(FILTER_HEIGHT),
            Constraint::Length(PROJECTS_HEIGHT),
            Constraint::Min(PROMPTS_AREA_MIN + DETAIL_TARGET_HEIGHT),
            Constraint::Length(FOOTER_HEIGHT),
        ])
        .split(frame.size());

    let header_area = shell[0];
    let filter_area = shell[1];
    let projects_area = shell[2];
    let content_area = shell[3];
    let footer_area = shell[4];

    let prompt_height = prompt_panel_height(content_area.height);
    let detail_height = content_area.height.saturating_sub(prompt_height);
    let content_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(prompt_height),
            Constraint::Length(detail_height),
        ])
        .split(content_area);
    let prompts_area = content_split[0];
    let detail_area = content_split[1];

    render_header(frame, header_area, ctx.app_state);
    filter_bar::render(frame, filter_area, ctx.app_state.filters(), &ctx.filter_ui);
    render_projects(
        frame,
        projects_area,
        ctx.app_state,
        ctx.active_pane == ActivePane::Projects,
    );
    render_prompts(
        frame,
        prompts_area,
        ctx.app_state,
        ctx.active_pane == ActivePane::Prompts,
    );
    render_detail(
        frame,
        detail_area,
        ctx.app_state,
        ctx.active_pane,
        ctx.original_scroll,
        ctx.review_scroll,
    );
    render_footer(frame, footer_area, ctx.status, ctx.selection_count);

    if let Some(lines) = ctx.diff_lines.as_ref() {
        render_diff_popup(frame, frame.size(), lines);
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let provider = state
        .current_prompt()
        .map(|record| record.provider.to_string())
        .unwrap_or_else(|| "n/a".into());
    let worker = &state.worker_status;
    let text = Line::from(vec![
        Span::styled(
            "Prompt Sage ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" · Provider: {}", provider)),
        Span::raw(format!(
            " · Worker q:{} r:{} ✓:{} ×:{}",
            worker.queued, worker.running, worker.completed, worker.failed
        )),
    ]);

    let block = Block::default();
    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn render_projects(frame: &mut Frame<'_>, area: Rect, state: &AppState, focused: bool) {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    state.iter_visible().for_each(|record| {
        *counts.entry(record.project.as_str()).or_insert(0) += 1;
    });

    let mut parts = Vec::new();
    for (project, count) in counts.iter().take(6) {
        parts.push(format!("{} ({})", project, count));
    }
    if counts.len() > 6 {
        parts.push("…".into());
    }
    if parts.is_empty() {
        parts.push("No prompts indexed. Run `prompt-sage index` first.".into());
    }

    let mut block = Block::default().title("Projects").borders(Borders::ALL);
    if focused {
        block = block.border_style(Style::default().fg(Color::Cyan));
    }

    frame.render_widget(
        Paragraph::new(parts.join(" · "))
            .block(block)
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_prompts(frame: &mut Frame<'_>, area: Rect, state: &AppState, focused: bool) {
    let mut block = Block::default()
        .title("Prompts (↑/↓ move · [Space] select · r review · / search · f filter)")
        .borders(Borders::ALL);
    if focused {
        block = block.border_style(Style::default().fg(Color::Cyan));
    }

    let mut rows = Vec::new();
    let visible = state.visible_indices();
    let total = visible.len();
    let available_rows = area.height.saturating_sub(3) as usize;
    let window = available_rows.max(1);
    let selected_row = state.selection.current;
    let start = if total <= window {
        0
    } else if selected_row >= total {
        total - window
    } else if selected_row < window / 2 {
        0
    } else {
        let tentative = selected_row + 1;
        tentative.saturating_sub(window)
    };
    let end = (start + window).min(total);

    for (i, idx) in visible[start..end].iter().enumerate() {
        if let Some(record) = state.prompts().get(*idx) {
            rows.push(build_prompt_row(
                record,
                state.selection.current == start + i,
                state.is_selected(&record.id),
            ));
        }
    }

    if rows.is_empty() {
        rows.push(Row::new(vec![Cell::from(
            "No prompts available. Use `index --stats` to ingest history.",
        )]));
    }

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Length(7),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Min(30),
            Constraint::Length(6),
            Constraint::Length(6),
        ],
    )
    .column_spacing(1)
    .header(
        Row::new(vec![
            Cell::from("Sel"),
            Cell::from("Prov"),
            Cell::from("Date"),
            Cell::from("Project"),
            Cell::from("Snippet"),
            Cell::from("Len"),
            Cell::from("Sage"),
        ])
        .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(block);

    frame.render_widget(table, area);
}

fn build_prompt_row(record: &PromptRecord, cursor: bool, selected: bool) -> Row<'static> {
    let status_symbol = match record.status {
        PromptStatus::ReviewedOk => "✓",
        PromptStatus::ReviewedError => "×",
        PromptStatus::Unreviewed => "·",
    };
    let project = truncate(&format_project(record), 12);

    let mut row = Row::new(vec![
        Cell::from(if selected { "★" } else { " " }),
        Cell::from(format_provider(record)),
        Cell::from(record.date.to_string()),
        Cell::from(project),
        Cell::from(truncate(&record.snippet, 40)),
        Cell::from(record.length.to_string()),
        Cell::from(status_symbol.to_string()),
    ]);

    if cursor {
        row = row.style(Style::default().add_modifier(Modifier::REVERSED));
    }

    row
}

fn render_detail(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    active: ActivePane,
    original_scroll: u16,
    review_scroll: u16,
) {
    let sections = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    let current = state.current_prompt();

    let mut original_block = Block::default()
        .title("[Original]")
        .borders(Borders::ALL)
        .padding(Padding::new(1, 1, 1, 1));
    let mut review_block = Block::default()
        .title("[Sage]")
        .borders(Borders::ALL)
        .padding(Padding::new(1, 1, 1, 1));
    if matches!(active, ActivePane::Original) {
        original_block = original_block.border_style(Style::default().fg(Color::Cyan));
    } else if matches!(active, ActivePane::Review) {
        review_block = review_block.border_style(Style::default().fg(Color::Cyan));
    }

    let original = Paragraph::new(detail::original_lines(current))
        .block(original_block)
        .scroll((original_scroll, 0))
        .wrap(Wrap { trim: false });
    let review = Paragraph::new(detail::review_lines(current))
        .block(review_block)
        .scroll((review_scroll, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(original, sections[0]);
    frame.render_widget(review, sections[1]);
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, status: Option<&str>, selection_count: usize) {
    let mut spans = vec![
        Span::raw("[Tab] switch  "),
        Span::raw("[Space] select  "),
        Span::raw("[r] review  "),
        Span::raw("[R] bulk review  "),
        Span::raw("[P] provider  "),
        Span::raw("[c] copy  "),
        Span::raw("[s] save  "),
        Span::raw("[q] quit  "),
        Span::raw(format!("Selections: {}", selection_count)),
    ];

    if let Some(message) = status {
        spans.push(Span::raw("  | "));
        spans.push(Span::styled(
            message,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line).block(Block::default()), area);
}

fn truncate(input: &str, limit: usize) -> String {
    if input.chars().count() <= limit {
        return input.to_string();
    }
    let mut truncated: String = input.chars().take(limit.saturating_sub(1)).collect();
    truncated.push('…');
    truncated
}

fn render_diff_popup(frame: &mut Frame<'_>, area: Rect, lines: &[String]) {
    let rect = centered_rect(70, 70, area);
    let paragraphs = lines
        .iter()
        .map(|line| Line::from(line.clone()))
        .collect::<Vec<_>>();
    let block = Block::default()
        .title("[d]/[esc] close diff")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    frame.render_widget(Paragraph::new(paragraphs).block(block), rect);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - height) / 2),
                Constraint::Percentage(height),
                Constraint::Percentage((100 - height) / 2),
            ]
            .as_ref(),
        )
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - width) / 2),
                Constraint::Percentage(width),
                Constraint::Percentage((100 - width) / 2),
            ]
            .as_ref(),
        )
        .split(vertical[1])[1]
}

fn prompt_panel_height(content_height: u16) -> u16 {
    if content_height == 0 {
        return 0;
    }
    let max_for_prompts = if content_height > 1 {
        content_height - 1
    } else {
        1
    };

    let mut min_area = PROMPTS_AREA_MIN.min(max_for_prompts);
    if max_for_prompts >= 3 {
        min_area = min_area.max(3);
    }
    let max_area = PROMPTS_AREA_MAX.min(max_for_prompts).max(min_area);

    if content_height <= PROMPTS_AREA_MIN + DETAIL_TARGET_HEIGHT {
        return min_area;
    }

    let desired = content_height.saturating_sub(DETAIL_TARGET_HEIGHT);
    desired.clamp(min_area, max_area)
}

fn format_provider(record: &PromptRecord) -> String {
    match record.provider {
        ProviderKind::Claude => "Claude".into(),
        ProviderKind::Codex => "Codex".into(),
    }
}

fn format_project(record: &PromptRecord) -> String {
    clean_project_name(record.project.as_str())
}

fn clean_project_name(raw: &str) -> String {
    if raw.is_empty() {
        return "-".into();
    }

    if raw.contains('/') || raw.contains('\\') {
        return path_tail(raw).unwrap_or_else(|| raw.to_string());
    }

    if raw.starts_with('-') {
        let segments: Vec<&str> = raw.split('-').filter(|s| !s.is_empty()).collect();
        if segments.is_empty() {
            return "-".into();
        }

        let start_idx = home_segments()
            .map(|home| {
                let mut matched = 0;
                while matched < home.len() && matched < segments.len() {
                    if segments[matched] == home[matched].as_str() {
                        matched += 1;
                    } else {
                        break;
                    }
                }
                matched
            })
            .unwrap_or(0);

        let remainder = if start_idx < segments.len() {
            &segments[start_idx..]
        } else {
            &segments[..]
        };

        if remainder.is_empty() {
            segments.join("/")
        } else {
            remainder.join("/")
        }
    } else {
        raw.to_string()
    }
}

fn home_segments() -> Option<&'static Vec<String>> {
    static HOME_SEGMENTS: OnceLock<Option<Vec<String>>> = OnceLock::new();
    HOME_SEGMENTS
        .get_or_init(|| {
            dirs::home_dir().map(|path| {
                path.components()
                    .filter_map(|comp| match comp {
                        Component::Normal(os) => os.to_str().map(|s| s.to_string()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })
        })
        .as_ref()
}

fn path_tail(input: &str) -> Option<String> {
    let path = Path::new(input);
    if let Some(name) = path.file_name().and_then(|os| os.to_str()) {
        return Some(name.to_string());
    }
    path.components()
        .filter_map(|comp| match comp {
            Component::Normal(os) => os.to_str().map(|s| s.to_string()),
            _ => None,
        })
        .last()
}
