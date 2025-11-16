use std::{
    fs,
    io::{self, Stdout},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use anyhow::Result;
use arboard::Clipboard;
use chrono::Utc;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::{
    app::{filter::FilterState, state::AppState},
    config::{self, AutoReviewConfig},
    telemetry,
    worker::review_queue::{ReviewQueueHandle, ReviewRequest, ReviewWorkerEvent},
};

use super::{
    export as tui_export,
    filter_bar::{FilterInputMode, FilterUiState},
    layout::{self, ActivePane, LayoutContext},
};

static JOB_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn run(
    state: AppState,
    worker: Option<ReviewQueueHandle>,
    auto_review: AutoReviewConfig,
    config_path: PathBuf,
) -> Result<FilterState> {
    let mut tui = PromptSageTui::new(state, worker, auto_review, config_path)?;
    tui.run()
}

struct PromptSageTui {
    app_state: AppState,
    view: ViewState,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    cleaned_up: bool,
    worker: Option<ReviewQueueHandle>,
    auto_review: AutoReviewConfig,
    auto_batch_started: bool,
    config_path: PathBuf,
}

impl PromptSageTui {
    fn new(
        app_state: AppState,
        worker: Option<ReviewQueueHandle>,
        auto_review: AutoReviewConfig,
        config_path: PathBuf,
    ) -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(PromptSageTui {
            app_state,
            view: ViewState::default(),
            terminal,
            cleaned_up: false,
            worker,
            auto_review,
            auto_batch_started: false,
            config_path,
        })
    }

    fn run(&mut self) -> Result<FilterState> {
        loop {
            self.start_auto_batch_if_needed();
            self.poll_worker_events();
            let active_pane = self.view.active_pane;
            let original_scroll = self.view.original_scroll;
            let review_scroll = self.view.review_scroll;
            let filter_ui = self.view.filter_ui.clone();
            let selection_count = self.app_state.selected_count();
            let diff_lines = self.view.diff_lines();
            let status = self.view.status_text();
            self.terminal.draw(|frame| {
                let ctx = LayoutContext {
                    app_state: &self.app_state,
                    active_pane,
                    original_scroll,
                    review_scroll,
                    status,
                    filter_ui,
                    selection_count,
                    diff_lines,
                };
                layout::render(frame, ctx);
            })?;

            if self.handle_events()? {
                break;
            }
        }

        self.shutdown()?;
        Ok(self.app_state.filters().clone())
    }

    fn handle_events(&mut self) -> Result<bool> {
        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if self.handle_key(key.code) {
                        return Ok(true);
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
        Ok(false)
    }

    fn handle_key(&mut self, code: KeyCode) -> bool {
        if self.view.diff_is_open() {
            match code {
                KeyCode::Esc | KeyCode::Char('d') => self.view.clear_diff(),
                _ => {}
            }
            return false;
        }
        if self.handle_filter_input(code) {
            return false;
        }
        match code {
            KeyCode::Char('q') => return true,
            KeyCode::Up => {
                self.app_state.prev_selection();
            }
            KeyCode::Down => {
                self.app_state.next_selection();
            }
            KeyCode::Tab => {
                self.view.active_pane = self.view.active_pane.next();
            }
            KeyCode::Char('r') => {
                self.trigger_review();
            }
            KeyCode::Char('R') => {
                self.trigger_bulk_review();
            }
            KeyCode::Char('P') => {
                self.app_state.cycle_provider_filter();
                self.view.set_status("Provider filter cycled");
            }
            KeyCode::Char('c') => {
                self.copy_selection();
            }
            KeyCode::Char('s') => {
                self.save_selection();
            }
            KeyCode::Char('d') => {
                self.show_diff();
            }
            KeyCode::Char(' ') => {
                self.toggle_selection();
            }
            KeyCode::Char('g') => self.app_state.selection.current = 0,
            KeyCode::Char('G') => {
                if !self.app_state.visible_indices().is_empty() {
                    self.app_state.selection.current =
                        self.app_state.visible_indices().len().saturating_sub(1);
                }
            }
            KeyCode::PageUp => self.scroll_original_up(),
            KeyCode::PageDown => self.scroll_original_down(),
            _ => {}
        }
        false
    }

    fn scroll_original_up(&mut self) {
        match self.view.active_pane {
            ActivePane::Original => {
                self.view.original_scroll = self.view.original_scroll.saturating_sub(4);
            }
            ActivePane::Review => {
                self.view.review_scroll = self.view.review_scroll.saturating_sub(4);
            }
            _ => {}
        }
    }

    fn scroll_original_down(&mut self) {
        match self.view.active_pane {
            ActivePane::Original => {
                self.view.original_scroll = self.view.original_scroll.saturating_add(4);
            }
            ActivePane::Review => {
                self.view.review_scroll = self.view.review_scroll.saturating_add(4);
            }
            _ => {}
        }
    }

    fn handle_filter_input(&mut self, code: KeyCode) -> bool {
        match self.view.filter_ui.mode() {
            FilterInputMode::Search => {
                match code {
                    KeyCode::Esc => {
                        self.view.filter_ui.set_mode(FilterInputMode::Normal);
                    }
                    KeyCode::Enter => {
                        let text = if self.view.filter_ui.buffer.trim().is_empty() {
                            None
                        } else {
                            Some(self.view.filter_ui.buffer.clone())
                        };
                        self.app_state.update_text_filter(text);
                        self.view.reset_scrolls();
                        self.view.filter_ui.set_mode(FilterInputMode::Normal);
                    }
                    KeyCode::Backspace => {
                        self.view.filter_ui.buffer.pop();
                    }
                    KeyCode::Char(ch) => {
                        self.view.filter_ui.buffer.push(ch);
                    }
                    _ => {}
                }
                true
            }
            FilterInputMode::Filter => {
                match code {
                    KeyCode::Esc | KeyCode::Enter => {
                        self.view.filter_ui.set_mode(FilterInputMode::Normal);
                    }
                    KeyCode::Char('p') => {
                        self.app_state.cycle_provider_filter();
                        self.view.reset_scrolls();
                    }
                    KeyCode::Char('s') => {
                        self.app_state.cycle_status_filter();
                        self.view.reset_scrolls();
                    }
                    KeyCode::Char('h') => {
                        self.app_state.toggle_hide_reviewed();
                        self.view.reset_scrolls();
                    }
                    KeyCode::Char('c') => {
                        self.app_state.clear_filters();
                        self.view.reset_scrolls();
                    }
                    _ => {}
                }
                true
            }
            FilterInputMode::Normal => match code {
                KeyCode::Char('/') => {
                    let current = self
                        .app_state
                        .filters()
                        .text()
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    self.view.filter_ui.buffer = current;
                    self.view.filter_ui.set_mode(FilterInputMode::Search);
                    true
                }
                KeyCode::Char('f') => {
                    self.view.filter_ui.set_mode(FilterInputMode::Filter);
                    true
                }
                _ => false,
            },
        }
    }

    fn shutdown(&mut self) -> Result<()> {
        if !self.cleaned_up {
            self.cleaned_up = true;
            self.restore_terminal()?;
        }
        Ok(())
    }

    fn restore_terminal(&mut self) -> Result<()> {
        disable_raw_mode()?;
        self.terminal.show_cursor()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.flush()?;
        Ok(())
    }
    fn poll_worker_events(&mut self) {
        let mut drained = Vec::new();
        if let Some(handle) = &self.worker {
            while let Some(event) = handle.try_recv() {
                drained.push(event);
            }
        }
        for event in drained {
            self.process_worker_event(event);
        }
    }

    fn start_auto_batch_if_needed(&mut self) {
        if self.auto_batch_started || !self.auto_review.enabled {
            return;
        }
        self.auto_batch_started = true;
        let Some(worker) = &self.worker else {
            self.view
                .set_status("Auto review unavailable (worker not configured)");
            return;
        };
        let limit = self.auto_review.batch_size.max(1);
        let ids = self.app_state.newest_unreviewed_prompt_ids(limit);
        if ids.is_empty() {
            self.view.set_status("Auto review skipped (no new prompts)");
            return;
        }
        if self.app_state.auto_batch().should_skip(&ids) {
            self.view
                .set_status("Auto review skipped (no prompts since last batch)");
            return;
        }

        self.app_state.reset_auto_batch();
        {
            let batch = self.app_state.auto_batch_mut();
            batch.candidate_ids = ids.clone();
            batch.started_at = Some(Utc::now());
        }
        telemetry::emit_auto_batch_start(ids.len());

        let mut queued = 0usize;
        for id in ids {
            let Some(record) = self.app_state.record_by_id(&id) else {
                continue;
            };
            if record.raw_text.trim().is_empty() {
                continue;
            }
            let provider = record.provider;
            let prompt_text = record.raw_text.clone();
            let record_id = record.id.clone();
            let request = ReviewRequest {
                job_id: format!("job-{}", JOB_COUNTER.fetch_add(1, Ordering::SeqCst)),
                prompt_record_id: record_id.clone(),
                provider,
                prompt_text,
                working_dir: None,
            };
            if let Err(err) = worker.enqueue(request) {
                self.view
                    .set_status(format!("Auto review enqueue failed: {err}"));
                break;
            } else {
                queued += 1;
                let batch = self.app_state.auto_batch_mut();
                batch.queued_ids.insert(record_id);
            }
        }

        if queued == 0 {
            self.app_state.reset_auto_batch();
            self.view
                .set_status("Auto review skipped (unable to queue prompts)");
        } else {
            self.view
                .set_status(format!("Auto review queued {queued} prompt(s)"));
        }
    }

    fn process_worker_event(&mut self, event: ReviewWorkerEvent) {
        match event {
            ReviewWorkerEvent::Queued(job) => {
                self.app_state.enqueue_job(job);
                self.view.set_status("Review queued");
            }
            ReviewWorkerEvent::Running(job) => {
                self.app_state.update_job(job);
                self.view.set_status("Review running");
            }
            ReviewWorkerEvent::Completed { job, review } => {
                self.app_state.update_job(job.clone());
                self.app_state.apply_review(review);
                self.app_state.remove_job(&job.id);
                self.app_state
                    .auto_batch_mut()
                    .mark_prompt_completed(&job.prompt_record_id);
                self.finalize_auto_batch_if_ready();
                self.view.set_status("Review completed");
            }
            ReviewWorkerEvent::Failed(job) => {
                self.app_state.update_job(job.clone());
                let message = job
                    .error
                    .as_ref()
                    .map(|err| err.message.as_str())
                    .unwrap_or("Review failed");
                self.app_state.remove_job(&job.id);
                self.app_state
                    .auto_batch_mut()
                    .mark_prompt_failed(&job.prompt_record_id);
                self.finalize_auto_batch_if_ready();
                self.view.set_status(message);
            }
        }
    }

    fn finalize_auto_batch_if_ready(&mut self) {
        if self.app_state.auto_batch().is_ready_to_finish() {
            let completed = self.app_state.auto_batch().completed_ids.len();
            let failed = self.app_state.auto_batch().failed_ids.len();
            let skipped = self.app_state.auto_batch().skipped_ids.len();
            self.app_state.auto_batch_mut().mark_finished();
            telemetry::emit_auto_batch_complete(completed, skipped, failed);
            if let Err(err) = config::persist_auto_review_state(
                &self.config_path,
                &self.app_state.auto_batch().last_completed_ids,
                self.app_state.auto_batch().finished_at,
            ) {
                self.view
                    .set_status(format!("Auto review persisted with warning: {err}"));
                return;
            }
            self.view.set_status(format!(
                "Auto review finished (✓{completed} · ×{failed} · ~{skipped})"
            ));
        }
    }

    fn trigger_review(&mut self) {
        let Some(record) = self.app_state.current_prompt().cloned() else {
            self.view.set_status("Select a prompt first");
            return;
        };

        if record.raw_text.trim().is_empty() {
            self.view
                .set_status("Prompt is empty; skipping review request");
            return;
        }

        let Some(worker) = &self.worker else {
            self.view
                .set_status("Reviews unavailable (worker not configured)");
            return;
        };

        let request = ReviewRequest {
            job_id: format!("job-{}", JOB_COUNTER.fetch_add(1, Ordering::SeqCst)),
            prompt_record_id: record.id.clone(),
            provider: record.provider,
            prompt_text: record.raw_text.clone(),
            working_dir: None,
        };

        if let Err(err) = worker.enqueue(request) {
            self.view
                .set_status(format!("Failed to enqueue review: {err}"));
        } else {
            self.view.set_status("Enqueued review job");
        }
    }

    fn trigger_bulk_review(&mut self) {
        let records = self.app_state.selected_or_current();

        if records.is_empty() {
            self.view.set_status("No prompts selected");
            return;
        }

        let Some(worker) = &self.worker else {
            self.view
                .set_status("Reviews unavailable (worker not configured)");
            return;
        };

        let mut enqueued_count = 0;
        let mut skipped_count = 0;

        for record in records {
            // Skip empty prompts
            if record.raw_text.trim().is_empty() {
                skipped_count += 1;
                continue;
            }

            // Skip already reviewed prompts (optional - can be removed if you want to re-review)
            if matches!(record.status, crate::model::PromptStatus::ReviewedOk) {
                skipped_count += 1;
                continue;
            }

            let request = ReviewRequest {
                job_id: format!("job-{}", JOB_COUNTER.fetch_add(1, Ordering::SeqCst)),
                prompt_record_id: record.id.clone(),
                provider: record.provider,
                prompt_text: record.raw_text.clone(),
                working_dir: None,
            };

            if let Err(err) = worker.enqueue(request) {
                self.view
                    .set_status(format!("Failed to enqueue review: {err}"));
                return;
            } else {
                enqueued_count += 1;
            }
        }

        // Clear selection after queuing
        self.app_state.clear_selection();

        let message = if skipped_count > 0 {
            format!(
                "Enqueued {} review(s), skipped {} already reviewed",
                enqueued_count, skipped_count
            )
        } else {
            format!("Enqueued {} review(s)", enqueued_count)
        };
        self.view.set_status(&message);
    }

    fn copy_selection(&mut self) {
        let records = self.app_state.selected_or_current();
        if records.is_empty() {
            self.view.set_status("Select a prompt first");
            return;
        }
        match tui_export::render_selection(records.as_slice()) {
            Ok(payload) => match Clipboard::new().and_then(|mut cb| cb.set_text(payload.clone())) {
                Ok(_) => self.view.set_status("Copied export markdown to clipboard"),
                Err(err) => {
                    if let Ok(path) = self.write_copy_backup(&payload) {
                        self.view.set_status(format!(
                            "Clipboard unavailable ({err}); wrote to {}",
                            path.display()
                        ));
                    } else {
                        self.view
                            .set_status(format!("Failed to copy prompt: {err}"));
                    }
                }
            },
            Err(err) => self.view.set_status(err.to_string()),
        }
    }

    fn save_selection(&mut self) {
        let records = self.app_state.selected_or_current();
        if records.is_empty() {
            self.view.set_status("Select a prompt first");
            return;
        }
        match tui_export::save_selection(records.as_slice()) {
            Ok(path) => self
                .view
                .set_status(format!("Saved export to {}", path.display())),
            Err(err) => self.view.set_status(err.to_string()),
        }
    }

    fn show_diff(&mut self) {
        let Some(record) = self.app_state.current_prompt() else {
            self.view.set_status("Select a prompt first");
            return;
        };
        match tui_export::diff_lines(record) {
            Ok(lines) => self.view.show_diff(lines),
            Err(err) => self.view.set_status(err.to_string()),
        }
    }

    fn toggle_selection(&mut self) {
        match self.app_state.toggle_current_selection() {
            Some(true) => self.view.set_status("Added prompt to export selection"),
            Some(false) => self.view.set_status("Removed prompt from export selection"),
            None => self.view.set_status("Select a prompt first"),
        }
    }

    fn write_copy_backup(&self, payload: &str) -> Result<PathBuf> {
        let mut dir = dirs::config_dir()
            .or_else(|| {
                dirs::home_dir().map(|mut path| {
                    path.push(".config");
                    path
                })
            })
            .unwrap_or_else(|| PathBuf::from("."));
        dir.push("prompt-sage");
        fs::create_dir_all(&dir)?;
        dir.push("last-copy.txt");
        fs::write(&dir, payload)?;
        Ok(dir)
    }
}

impl Drop for PromptSageTui {
    fn drop(&mut self) {
        if self.cleaned_up {
            return;
        }
        let _ = self.restore_terminal();
    }
}

#[derive(Debug)]
struct ViewState {
    active_pane: ActivePane,
    original_scroll: u16,
    review_scroll: u16,
    status: Option<StatusMessage>,
    filter_ui: FilterUiState,
    diff_lines: Option<Vec<String>>,
}

impl Default for ViewState {
    fn default() -> Self {
        ViewState {
            active_pane: ActivePane::Prompts,
            original_scroll: 0,
            review_scroll: 0,
            status: None,
            filter_ui: FilterUiState::default(),
            diff_lines: None,
        }
    }
}

impl ViewState {
    fn set_status<S: Into<String>>(&mut self, message: S) {
        self.status = Some(StatusMessage {
            text: message.into(),
            expires_at: Instant::now() + Duration::from_secs(5),
        });
    }

    fn status_text(&mut self) -> Option<&str> {
        let expired = self
            .status
            .as_ref()
            .map(|status| Instant::now() > status.expires_at)
            .unwrap_or(false);
        if expired {
            self.status = None;
        }
        self.status.as_ref().map(|status| status.text.as_str())
    }

    fn reset_scrolls(&mut self) {
        self.original_scroll = 0;
        self.review_scroll = 0;
    }

    fn diff_lines(&self) -> Option<Vec<String>> {
        self.diff_lines.clone()
    }

    fn diff_is_open(&self) -> bool {
        self.diff_lines.is_some()
    }

    fn show_diff(&mut self, lines: Vec<String>) {
        self.diff_lines = Some(lines);
    }

    fn clear_diff(&mut self) {
        self.diff_lines = None;
    }
}

#[derive(Debug, Clone)]
struct StatusMessage {
    text: String,
    expires_at: Instant,
}
