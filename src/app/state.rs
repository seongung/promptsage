#![allow(dead_code)]

use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

use chrono::{DateTime, Utc};

use crate::{
    app::filter::FilterState,
    model::{
        PromptRecord, PromptReview, PromptReviewStatus, PromptStatus, ProviderKind, ReviewJob,
        ReviewJobState,
    },
};

#[derive(Debug)]
pub struct AppState {
    prompts: Vec<PromptRecord>,
    filtered_indices: Vec<usize>,
    pub filters: FilterState,
    pub selection: SelectionState,
    reviews: HashMap<String, PromptReview>,
    review_jobs: HashMap<String, ReviewJob>,
    pub worker_status: WorkerStatus,
    selected_prompts: HashSet<String>,
    auto_batch: AutoBatchState,
}

impl AppState {
    pub fn new(records: Vec<PromptRecord>) -> Self {
        Self::with_filters(records, FilterState::default())
    }

    pub fn with_filters(records: Vec<PromptRecord>, filters: FilterState) -> Self {
        let mut state = AppState {
            prompts: records,
            filtered_indices: Vec::new(),
            filters,
            selection: SelectionState::default(),
            reviews: HashMap::new(),
            review_jobs: HashMap::new(),
            worker_status: WorkerStatus::default(),
            selected_prompts: HashSet::new(),
            auto_batch: AutoBatchState::default(),
        };
        state.refilter();
        state
    }

    pub fn prompts(&self) -> &[PromptRecord] {
        &self.prompts
    }

    pub fn visible_indices(&self) -> &[usize] {
        &self.filtered_indices
    }

    pub fn iter_visible(&self) -> impl Iterator<Item = &PromptRecord> {
        self.filtered_indices
            .iter()
            .map(move |&idx| &self.prompts[idx])
    }

    pub fn current_prompt(&self) -> Option<&PromptRecord> {
        self.filtered_indices
            .get(self.selection.current)
            .map(|&idx| &self.prompts[idx])
    }

    pub fn current_prompt_mut(&mut self) -> Option<&mut PromptRecord> {
        if let Some(&idx) = self.filtered_indices.get(self.selection.current) {
            self.prompts.get_mut(idx)
        } else {
            None
        }
    }

    pub fn set_filters(&mut self, filters: FilterState) {
        self.filters = filters;
        self.selection.current = 0;
        self.refilter();
    }

    pub fn filters(&self) -> &FilterState {
        &self.filters
    }

    pub fn filters_mut(&mut self) -> &mut FilterState {
        &mut self.filters
    }

    pub fn update_text_filter(&mut self, text: Option<String>) {
        self.filters.set_text(text);
        self.selection.current = 0;
        self.refilter();
    }

    pub fn cycle_provider_filter(&mut self) {
        let next = match self.filters.provider() {
            None => Some(ProviderKind::Claude),
            Some(ProviderKind::Claude) => Some(ProviderKind::Codex),
            Some(ProviderKind::Codex) => None,
        };
        self.filters.set_provider(next);
        self.selection.current = 0;
        self.refilter();
    }

    pub fn cycle_status_filter(&mut self) {
        let next = match self.filters.status() {
            None => Some(PromptStatus::Unreviewed),
            Some(PromptStatus::Unreviewed) => Some(PromptStatus::ReviewedOk),
            Some(PromptStatus::ReviewedOk) => Some(PromptStatus::ReviewedError),
            Some(PromptStatus::ReviewedError) => None,
        };
        self.filters.set_status(next);
        self.selection.current = 0;
        self.refilter();
    }

    pub fn toggle_hide_reviewed(&mut self) {
        self.filters
            .set_hide_reviewed(!self.filters.hide_reviewed());
        self.selection.current = 0;
        self.refilter();
    }

    pub fn clear_filters(&mut self) {
        self.filters = FilterState::default();
        self.selection.current = 0;
        self.refilter();
    }

    pub fn clear_selection(&mut self) {
        self.selected_prompts.clear();
    }

    pub fn selected_count(&self) -> usize {
        self.selected_prompts.len()
    }

    pub fn has_selection(&self) -> bool {
        !self.selected_prompts.is_empty()
    }

    pub fn toggle_current_selection(&mut self) -> Option<bool> {
        let prompt = self.current_prompt()?.id.clone();
        if self.selected_prompts.insert(prompt.clone()) {
            Some(true)
        } else {
            self.selected_prompts.remove(&prompt);
            Some(false)
        }
    }

    pub fn is_selected(&self, prompt_id: &str) -> bool {
        self.selected_prompts.contains(prompt_id)
    }

    pub fn selected_records(&self) -> Vec<&PromptRecord> {
        self.filtered_indices
            .iter()
            .filter_map(|&idx| {
                let record = &self.prompts[idx];
                if self.selected_prompts.contains(&record.id) {
                    Some(record)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn selected_or_current(&self) -> Vec<&PromptRecord> {
        let mut records = self.selected_records();
        if records.is_empty() {
            if let Some(current) = self.current_prompt() {
                records.push(current);
            }
        }
        records
    }

    pub fn record_by_id(&self, id: &str) -> Option<&PromptRecord> {
        self.prompts.iter().find(|record| record.id == id)
    }

    pub fn newest_unreviewed_prompt_ids(&self, limit: usize) -> Vec<String> {
        let mut candidates: Vec<&PromptRecord> = self
            .prompts
            .iter()
            .filter(|record| matches!(record.status, PromptStatus::Unreviewed))
            .collect();
        candidates.sort_by(|a, b| compare_prompt_freshness(b, a));
        candidates
            .into_iter()
            .take(limit)
            .map(|record| record.id.clone())
            .collect()
    }

    pub fn update_selection(&mut self, mut selection: SelectionState) {
        selection.clamp(self.filtered_indices.len());
        self.selection = selection;
    }

    pub fn next_selection(&mut self) {
        self.selection.move_next(self.filtered_indices.len());
    }

    pub fn prev_selection(&mut self) {
        self.selection.move_previous(self.filtered_indices.len());
    }

    pub fn enqueue_job(&mut self, job: ReviewJob) {
        self.review_jobs.insert(job.id.clone(), job);
        self.refresh_worker_status();
    }

    pub fn update_job(&mut self, job: ReviewJob) {
        self.review_jobs.insert(job.id.clone(), job);
        self.refresh_worker_status();
    }

    pub fn remove_job(&mut self, job_id: &str) -> Option<ReviewJob> {
        let result = self.review_jobs.remove(job_id);
        if result.is_some() {
            self.refresh_worker_status();
        }
        result
    }

    pub fn auto_batch(&self) -> &AutoBatchState {
        &self.auto_batch
    }

    pub fn auto_batch_mut(&mut self) -> &mut AutoBatchState {
        &mut self.auto_batch
    }

    pub fn reset_auto_batch(&mut self) {
        self.auto_batch.reset_runtime();
    }

    pub fn apply_review(&mut self, review: PromptReview) {
        let prompt_id = review.prompt_record_id.clone();
        self.reviews.insert(prompt_id.clone(), review.clone());

        if let Some(record) = self.prompts.iter_mut().find(|r| r.id == prompt_id) {
            record.latest_review = Some(review.clone());
            record.status = match review.status {
                PromptReviewStatus::Completed => PromptStatus::ReviewedOk,
                PromptReviewStatus::Failed => PromptStatus::ReviewedError,
                PromptReviewStatus::Pending => PromptStatus::Unreviewed,
            };
        }
    }

    pub fn review_for(&self, prompt_id: &str) -> Option<&PromptReview> {
        self.reviews.get(prompt_id)
    }

    pub fn jobs(&self) -> &HashMap<String, ReviewJob> {
        &self.review_jobs
    }

    fn refilter(&mut self) {
        self.filtered_indices = self
            .prompts
            .iter()
            .enumerate()
            .filter_map(|(idx, record)| {
                if self.filters.matches(record) {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();

        self.selection.clamp(self.filtered_indices.len());

        if !self.selected_prompts.is_empty() {
            let visible: HashSet<String> = self
                .filtered_indices
                .iter()
                .filter_map(|&idx| self.prompts.get(idx))
                .map(|record| record.id.clone())
                .collect();
            self.selected_prompts.retain(|id| visible.contains(id));
        }
    }

    fn refresh_worker_status(&mut self) {
        self.worker_status = WorkerStatus::from_jobs(&self.review_jobs);
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SelectionState {
    pub current: usize,
}

impl SelectionState {
    pub fn clamp(&mut self, total: usize) {
        if total == 0 {
            self.current = 0;
        } else if self.current >= total {
            self.current = total - 1;
        }
    }

    pub fn move_next(&mut self, total: usize) {
        if total == 0 {
            self.current = 0;
        } else if self.current + 1 < total {
            self.current += 1;
        }
    }

    pub fn move_previous(&mut self, total: usize) {
        if self.current > 0 && total > 0 {
            self.current -= 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkerStatus {
    pub queued: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub last_update: chrono::DateTime<Utc>,
}

impl WorkerStatus {
    fn now() -> Self {
        WorkerStatus {
            queued: 0,
            running: 0,
            completed: 0,
            failed: 0,
            last_update: Utc::now(),
        }
    }

    pub fn from_jobs(jobs: &HashMap<String, ReviewJob>) -> Self {
        let mut status = WorkerStatus::now();
        for job in jobs.values() {
            match job.state {
                ReviewJobState::Queued => status.queued += 1,
                ReviewJobState::Running => status.running += 1,
                ReviewJobState::Completed => status.completed += 1,
                ReviewJobState::Failed | ReviewJobState::Cancelled => status.failed += 1,
            }
        }
        status
    }
}

impl Default for WorkerStatus {
    fn default() -> Self {
        WorkerStatus::now()
    }
}

#[derive(Debug, Clone)]
pub struct AutoBatchState {
    pub candidate_ids: Vec<String>,
    pub queued_ids: HashSet<String>,
    pub completed_ids: HashSet<String>,
    pub failed_ids: HashSet<String>,
    pub skipped_ids: HashSet<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub cancelled: bool,
    pub last_completed_ids: Vec<String>,
}

impl Default for AutoBatchState {
    fn default() -> Self {
        AutoBatchState {
            candidate_ids: Vec::new(),
            queued_ids: HashSet::new(),
            completed_ids: HashSet::new(),
            failed_ids: HashSet::new(),
            skipped_ids: HashSet::new(),
            started_at: None,
            finished_at: None,
            cancelled: false,
            last_completed_ids: Vec::new(),
        }
    }
}

impl AutoBatchState {
    pub fn reset_runtime(&mut self) {
        self.candidate_ids.clear();
        self.queued_ids.clear();
        self.completed_ids.clear();
        self.failed_ids.clear();
        self.skipped_ids.clear();
        self.started_at = None;
        self.finished_at = None;
        self.cancelled = false;
    }

    pub fn should_skip(&self, candidates: &[String]) -> bool {
        !candidates.is_empty()
            && !self.last_completed_ids.is_empty()
            && self.last_completed_ids == candidates
    }

    pub fn mark_prompt_completed(&mut self, id: &str) {
        if self.candidate_ids.iter().any(|candidate| candidate == id) {
            self.completed_ids.insert(id.to_string());
        }
    }

    pub fn mark_prompt_failed(&mut self, id: &str) {
        if self.candidate_ids.iter().any(|candidate| candidate == id) {
            self.failed_ids.insert(id.to_string());
        }
    }

    pub fn mark_prompt_skipped(&mut self, id: &str) {
        if self.candidate_ids.iter().any(|candidate| candidate == id) {
            self.skipped_ids.insert(id.to_string());
        }
    }

    pub fn is_ready_to_finish(&self) -> bool {
        if self.candidate_ids.is_empty() || self.finished_at.is_some() {
            return false;
        }
        let processed = self.completed_ids.len() + self.failed_ids.len() + self.skipped_ids.len();
        processed >= self.queued_ids.len() && !self.queued_ids.is_empty()
    }

    pub fn mark_finished(&mut self) {
        self.finished_at = Some(Utc::now());
        self.last_completed_ids = self.candidate_ids.clone();
    }
}

fn compare_prompt_freshness(a: &PromptRecord, b: &PromptRecord) -> Ordering {
    match (a.timestamp, b.timestamp) {
        (Some(a_ts), Some(b_ts)) => a_ts.cmp(&b_ts),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => a.date.cmp(&b.date),
    }
}
