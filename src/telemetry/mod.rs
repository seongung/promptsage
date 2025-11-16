use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use serde_json::{json, Value};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum EventKind {
    Index,
    Worker,
    Privacy,
}

impl EventKind {
    fn as_str(&self) -> &'static str {
        match self {
            EventKind::Index => "index",
            EventKind::Worker => "worker",
            EventKind::Privacy => "privacy",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TelemetryLevel {
    Info,
    Warn,
    Error,
}

#[derive(Serialize)]
struct EventRecord<'a> {
    timestamp: String,
    category: &'static str,
    level: TelemetryLevel,
    action: &'a str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

pub fn emit_index(action: &str, message: &str, records: usize, files: usize) {
    emit(
        EventKind::Index,
        TelemetryLevel::Info,
        action,
        message,
        Some(json!({
            "records": records,
            "files": files,
        })),
    );
}

#[allow(dead_code)]
pub fn emit_worker(action: &str, level: TelemetryLevel, message: &str, details: Option<Value>) {
    emit(EventKind::Worker, level, action, message, details);
}

#[allow(dead_code)]
pub fn emit_privacy_alert(action: &str, prompt_text: &str) {
    emit(
        EventKind::Privacy,
        TelemetryLevel::Warn,
        action,
        "privacy guard activated",
        Some(json!({
            "sample": scrub_prompt(prompt_text),
        })),
    );
}

#[allow(dead_code)]
pub fn scrub_prompt(prompt: &str) -> String {
    const LIMIT: usize = 80;
    let sanitized = prompt.trim().replace('\n', " ");
    if sanitized.chars().count() <= LIMIT {
        return sanitized;
    }
    let mut truncated: String = sanitized.chars().take(LIMIT).collect();
    truncated.push('…');
    truncated
}

fn emit(
    kind: EventKind,
    level: TelemetryLevel,
    action: &str,
    message: &str,
    details: Option<Value>,
) {
    let record = EventRecord {
        timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        category: kind.as_str(),
        level,
        action,
        message,
        details,
    };

    match serde_json::to_string(&record) {
        Ok(payload) => eprintln!("{payload}"),
        Err(_) => eprintln!("[telemetry:{}] {} - {}", kind.as_str(), action, message),
    }
}

pub fn emit_auto_batch_start(total_prompts: usize) {
    emit_worker(
        "auto_batch_start",
        TelemetryLevel::Info,
        "auto review batch started",
        Some(json!({ "total_prompts": total_prompts })),
    );
}

pub fn emit_auto_batch_complete(completed: usize, skipped: usize, failed: usize) {
    emit_worker(
        "auto_batch_complete",
        TelemetryLevel::Info,
        "auto review batch completed",
        Some(json!({
            "completed": completed,
            "skipped": skipped,
            "failed": failed
        })),
    );
}

pub fn emit_auto_batch_cancelled(remaining: usize) {
    emit_worker(
        "auto_batch_cancelled",
        TelemetryLevel::Warn,
        "auto review batch cancelled by user",
        Some(json!({ "pending_prompts": remaining })),
    );
}
