use std::{
    path::PathBuf,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use tokio::{sync::mpsc as tokio_mpsc, time};

use crate::{
    model::{
        PromptReview, PromptReviewStatus, ProviderKind, ReviewJob, ReviewJobError, ReviewJobState,
    },
    providers::{self, ProviderRequest},
    review::parser::{self, ParsedReview},
};

const DEFAULT_TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Clone)]
pub struct ProviderBinary {
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ReviewWorkerConfig {
    pub constitution: String,
    pub claude: Option<ProviderBinary>,
    pub codex: Option<ProviderBinary>,
    pub timeout: Duration,
}

impl ReviewWorkerConfig {
    pub fn with_constitution(constitution: String) -> Self {
        ReviewWorkerConfig {
            constitution,
            claude: None,
            codex: None,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReviewRequest {
    pub job_id: String,
    pub prompt_record_id: String,
    pub provider: ProviderKind,
    pub prompt_text: String,
    pub working_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum ReviewWorkerEvent {
    Queued(ReviewJob),
    Running(ReviewJob),
    Completed {
        job: ReviewJob,
        review: PromptReview,
    },
    Failed(ReviewJob),
}

pub struct ReviewQueueHandle {
    sender: tokio_mpsc::Sender<ReviewRequest>,
    receiver: mpsc::Receiver<ReviewWorkerEvent>,
}

impl ReviewQueueHandle {
    pub fn enqueue(&self, request: ReviewRequest) -> Result<()> {
        self.sender
            .blocking_send(request)
            .context("review queue closed")?;
        Ok(())
    }

    pub fn try_recv(&self) -> Option<ReviewWorkerEvent> {
        self.receiver.try_recv().ok()
    }
}

pub fn spawn_worker(config: ReviewWorkerConfig) -> Result<ReviewQueueHandle> {
    let (job_tx, job_rx) = tokio_mpsc::channel::<ReviewRequest>(32);
    let (event_tx, event_rx) = mpsc::channel::<ReviewWorkerEvent>();
    let config = Arc::new(config);

    thread::Builder::new().name("review-worker".into()).spawn({
        let config = Arc::clone(&config);
        move || {
            let runtime = tokio::runtime::Runtime::new()
                .expect("Failed to create Tokio runtime for review worker");
            runtime.block_on(worker_loop(job_rx, event_tx, config));
        }
    })?;

    Ok(ReviewQueueHandle {
        sender: job_tx,
        receiver: event_rx,
    })
}

async fn worker_loop(
    mut job_rx: tokio_mpsc::Receiver<ReviewRequest>,
    event_tx: mpsc::Sender<ReviewWorkerEvent>,
    config: Arc<ReviewWorkerConfig>,
) {
    while let Some(request) = job_rx.recv().await {
        let mut job = ReviewJob {
            id: request.job_id.clone(),
            prompt_record_id: request.prompt_record_id.clone(),
            provider: request.provider,
            working_dir: request.working_dir.clone(),
            state: ReviewJobState::Queued,
            requested_at: Utc::now(),
            started_at: None,
            finished_at: None,
            error: None,
        };
        let _ = event_tx.send(ReviewWorkerEvent::Queued(job.clone()));

        job.state = ReviewJobState::Running;
        job.started_at = Some(Utc::now());
        let _ = event_tx.send(ReviewWorkerEvent::Running(job.clone()));

        let result = process_request(&request, Arc::clone(&config)).await;

        match result {
            Ok(parsed) => {
                job.state = ReviewJobState::Completed;
                job.finished_at = Some(Utc::now());
                let review = build_prompt_review(&request, parsed);
                let _ = event_tx.send(ReviewWorkerEvent::Completed { job, review });
            }
            Err(err) => {
                job.state = ReviewJobState::Failed;
                job.finished_at = Some(Utc::now());
                job.error = Some(ReviewJobError {
                    message: err.to_string(),
                    details: None,
                });
                let _ = event_tx.send(ReviewWorkerEvent::Failed(job));
            }
        }
    }
}

async fn process_request(
    request: &ReviewRequest,
    config: Arc<ReviewWorkerConfig>,
) -> Result<ParsedReview> {
    let binary = match request.provider {
        ProviderKind::Claude => config
            .claude
            .as_ref()
            .ok_or_else(|| anyhow!("Claude CLI is not configured"))?,
        ProviderKind::Codex => config
            .codex
            .as_ref()
            .ok_or_else(|| anyhow!("Codex CLI is not configured"))?,
    };

    let provider_request = ProviderRequest {
        binary: binary.path.as_path(),
        constitution: &config.constitution,
        prompt: request.prompt_text.as_str(),
        working_dir: request.working_dir.as_deref(),
    };

    let response = time::timeout(config.timeout, async {
        match request.provider {
            ProviderKind::Claude => providers::claude::run_review(&provider_request).await,
            ProviderKind::Codex => providers::codex::run_review(&provider_request).await,
        }
    })
    .await
    .map_err(|_| anyhow!("provider timed out after {:?}", config.timeout))??;

    if !response.status.success() {
        return Err(anyhow!(
            "provider exited with status {:?}: {}",
            response.status.code(),
            response.stderr
        ));
    }

    // Note: Debug logging disabled to prevent TUI corruption
    // Uncomment below for debugging provider responses:
    // eprintln!("=== PROVIDER DEBUG ({}) ===", request.provider);
    // eprintln!("STDOUT ({} bytes):", response.stdout.len());
    // eprintln!("{}", response.stdout);
    // eprintln!("=== END PROVIDER DEBUG ===\n");

    parser::parse_payload(&response.stdout)
}

fn build_prompt_review(request: &ReviewRequest, parsed: ParsedReview) -> PromptReview {
    let review_id = format!("{}:{}", request.prompt_record_id, request.job_id);
    PromptReview {
        id: review_id,
        prompt_record_id: request.prompt_record_id.clone(),
        provider: request.provider,
        improved_prompt: parsed.improved_prompt,
        explanation: parsed.explanation,
        score: parsed.score,
        tags: parsed.tags,
        created_at: Utc::now(),
        status: PromptReviewStatus::Completed,
        metadata: parsed.metadata,
    }
}
