use std::fs;
use std::path::{Component, Path};
use std::time::SystemTime;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde_json::Value;

use crate::model::PromptSource;

const USER_ROLES: &[&str] = &["user", "human", "client"];
const NON_USER_ROLES: &[&str] = &[
    "assistant",
    "ai",
    "model",
    "system",
    "tool",
    "function",
    "agent",
];
const PROJECT_KEYS: &[&str] = &[
    "project",
    "project_name",
    "projectPath",
    "project_path",
    "workspace",
    "repo",
    "cwd",
];
const SESSION_KEYS: &[&str] = &[
    "session_id",
    "sessionId",
    "session",
    "conversation_id",
    "conversationId",
    "chat_id",
];
const TIMESTAMP_KEYS: &[&str] = &[
    "timestamp",
    "created_at",
    "updated_at",
    "time",
    "ts",
    "event_time",
];

pub fn extract_prompt_text(value: &Value) -> Option<String> {
    if !is_user_role(value) {
        return None;
    }

    let text_source = value
        .get("payload")
        .and_then(find_text_node)
        .or_else(|| find_text_node(value))?;
    sanitize_prompt(&text_source)
}

pub fn derive_project(
    value: &Value,
    path: &Path,
    source: &PromptSource,
    hint: Option<&str>,
) -> String {
    let candidate = project_candidate(value)
        .or_else(|| hint.map(|h| h.to_string()))
        .map(|text| normalize_project_label(&text))
        .or_else(|| {
            project_from_path(path, &source.root_path).map(|text| normalize_project_label(&text))
        });

    candidate.unwrap_or_else(|| "unknown".to_string())
}

pub fn derive_session(value: &Value, path: &Path) -> Option<String> {
    parse_string_keys(value, SESSION_KEYS)
        .or_else(|| {
            value
                .get("metadata")
                .and_then(|meta| parse_string_keys(meta, SESSION_KEYS))
        })
        .or_else(|| session_from_path(path))
}

pub fn derive_timestamp(value: &Value) -> Option<DateTime<Utc>> {
    for key in TIMESTAMP_KEYS {
        if let Some(node) = value.get(*key) {
            if let Some(dt) = parse_timestamp_node(node) {
                return Some(dt);
            }
        }
    }
    if let Some(meta) = value.get("metadata") {
        for key in TIMESTAMP_KEYS {
            if let Some(node) = meta.get(*key) {
                if let Some(dt) = parse_timestamp_node(node) {
                    return Some(dt);
                }
            }
        }
    }
    None
}

pub fn fallback_date(path: &Path) -> NaiveDate {
    if let Ok(metadata) = fs::metadata(path) {
        if let Ok(modified) = metadata.modified() {
            return datetime_from_system_time(modified).date_naive();
        }
    }
    Utc::now().date_naive()
}

pub fn session_from_path(path: &Path) -> Option<String> {
    path.file_stem()
        .or_else(|| path.file_name())
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

pub fn extract_cwd(value: &Value) -> Option<String> {
    value
        .get("payload")
        .and_then(|payload| payload.get("cwd"))
        .and_then(|node| node.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            value
                .get("cwd")
                .and_then(|node| node.as_str().map(|s| s.to_string()))
        })
}

pub fn project_from_path(path: &Path, root: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let mut parts: Vec<String> = rel
        .components()
        .filter_map(|comp| match comp {
            Component::Normal(os) => os.to_str().map(|s| s.to_string()),
            _ => None,
        })
        .collect();
    if parts.is_empty() {
        return None;
    }
    if let Some(last) = parts.last_mut() {
        if let Some(stripped) = last.strip_suffix(".jsonl") {
            *last = stripped.to_string();
        }
    }
    parts
        .iter()
        .find(|segment| segment.chars().any(|ch| ch.is_alphabetic()))
        .cloned()
        .or_else(|| parts.last().cloned())
}

fn datetime_from_system_time(time: SystemTime) -> DateTime<Utc> {
    DateTime::<Utc>::from(time)
}

fn parse_timestamp_node(node: &Value) -> Option<DateTime<Utc>> {
    if let Some(s) = node.as_str() {
        parse_timestamp_from_str(s)
    } else if let Some(num) = node.as_i64() {
        parse_timestamp_from_i64(num)
    } else if let Some(num) = node.as_u64() {
        parse_timestamp_from_i64(num as i64)
    } else {
        None
    }
}

fn parse_timestamp_from_str(value: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));
    }
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        if let Some(naive) = date.and_hms_opt(0, 0, 0) {
            return Some(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));
        }
    }
    None
}

fn parse_timestamp_from_i64(value: i64) -> Option<DateTime<Utc>> {
    if value == 0 {
        return None;
    }
    let seconds = if value > 1_000_000_000_000 {
        value / 1000
    } else {
        value
    };
    DateTime::<Utc>::from_timestamp(seconds, 0)
}

fn parse_string_keys(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(text) = value.get(*key).and_then(|v| {
            if v.is_string() {
                v.as_str().map(|s| s.to_string())
            } else if v.is_array() {
                v.as_array()
                    .and_then(|arr| arr.iter().filter_map(|item| item.as_str()).next())
                    .map(|s| s.to_string())
            } else {
                None
            }
        }) {
            if !text.trim().is_empty() {
                return Some(text.trim().to_string());
            }
        }
    }
    None
}

fn project_candidate(value: &Value) -> Option<String> {
    parse_string_keys(value, PROJECT_KEYS)
        .or_else(|| {
            value
                .get("metadata")
                .and_then(|meta| parse_string_keys(meta, PROJECT_KEYS))
        })
        .or_else(|| {
            value
                .get("payload")
                .and_then(|payload| parse_string_keys(payload, PROJECT_KEYS))
        })
}

fn normalize_project_label(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "unknown".into();
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return collapse_to_project_label(trimmed);
    }
    trimmed.to_string()
}

fn collapse_to_project_label(input: &str) -> String {
    let path = Path::new(input);
    if let Some(name) = path.file_name().and_then(|os| os.to_str()) {
        return name.to_string();
    }
    path.components()
        .filter_map(|comp| match comp {
            Component::Normal(os) => os.to_str().map(|s| s.to_string()),
            _ => None,
        })
        .last()
        .unwrap_or_else(|| input.trim_matches(&['/', '\\']).to_string())
}

fn find_text_node(value: &Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        return Some(s.to_string());
    }

    const TEXT_KEYS: &[&str] = &[
        "prompt",
        "text",
        "input",
        "user_input",
        "body",
        "raw_text",
        "message",
        "content",
    ];

    for key in TEXT_KEYS {
        if let Some(node) = value.get(*key) {
            if let Some(text) = extract_text_from_value(node) {
                return Some(text);
            }
        }
    }

    extract_text_from_value(value)
}

const TEXT_ITEM_TYPES: &[&str] = &["text", "input_text", "output_text", "user_input"];

fn extract_text_from_value(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.to_owned()),
        Value::Array(items) => {
            let mut segments = Vec::new();
            for item in items {
                // CRITICAL: Only extract items with type == "text" (matches Python implementation)
                // This filters out tool_use, system messages, and other non-text content
                if let Some(obj) = item.as_object() {
                    let item_type = obj.get("type").and_then(|v| v.as_str());

                    // Only process items with type == "text" or items without a type field
                    if let Some(t) = item_type {
                        let normalized = t.to_ascii_lowercase();
                        if !TEXT_ITEM_TYPES
                            .iter()
                            .any(|allowed| *allowed == normalized.as_str())
                        {
                            // Skip tool_use, tool_result, system, etc.
                            continue;
                        }
                    }

                    // Extract the "text" field from this item
                    if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                        segments.push(text.to_owned());
                    }
                } else if let Some(text) = item.as_str() {
                    // Handle string items directly
                    segments.push(text.to_owned());
                }
            }
            if segments.is_empty() {
                None
            } else {
                Some(segments.join("\n"))
            }
        }
        Value::Object(map) => {
            if let Some(text) = map.get("text").and_then(|v| v.as_str()) {
                return Some(text.to_owned());
            }
            if let Some(content) = map.get("content") {
                return extract_text_from_value(content);
            }
            None
        }
        _ => None,
    }
}

fn sanitize_prompt(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Filter out various non-user content (matches Python reference implementation)
    // Skip tool results
    if trimmed.starts_with("tool_use_id") {
        return None;
    }

    // Skip interruption messages
    if trimmed.contains("[Request interrupted") {
        return None;
    }

    // Skip session continuation messages
    if trimmed
        .to_ascii_lowercase()
        .contains("session is being continued")
    {
        return None;
    }

    // Skip command output messages (e.g., "foo is running…")
    if trimmed.contains("is running") && trimmed.contains('…') {
        return None;
    }

    // Skip if it looks like a command message
    if looks_like_command(trimmed) {
        return None;
    }

    // Skip system reminders and environment context
    if trimmed.starts_with("<system-") || trimmed.starts_with("<environment_") {
        return None;
    }

    // Skip if the entire content is just XML-like tags
    if trimmed.starts_with('<') && trimmed.ends_with('>') {
        return None;
    }

    Some(trimmed.to_string())
}

fn looks_like_command(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("<command-") || lower.starts_with("/command/")
}

fn is_user_role(value: &Value) -> bool {
    // Claude JSONL format: Must check BOTH top-level "type" AND nested "message.role"
    // This matches the Python reference implementation exactly
    if let Some(top_type) = value.get("type").and_then(|v| v.as_str()) {
        let type_lower = top_type.to_ascii_lowercase();

        // Explicitly reject non-user types first
        if type_lower == "assistant"
            || type_lower == "ai"
            || type_lower == "system"
            || type_lower.contains("tool")
            || type_lower.contains("function")
        {
            return false;
        }

        // If type is "user", ALSO verify message.role is "user" (both must match!)
        if type_lower == "user" {
            if let Some(message) = value.get("message") {
                if let Some(role) = message.get("role").and_then(|v| v.as_str()) {
                    let role_lower = role.to_ascii_lowercase();
                    // Both type AND role must be "user"
                    return USER_ROLES.iter().any(|&user| role_lower.contains(user));
                }
            }
            // If type is "user" but no valid message.role, reject
            return false;
        }
    }

    // Check nested message.role (Claude alternative structure)
    if let Some(message) = value.get("message") {
        if let Some(role) = message.get("role").and_then(|v| v.as_str()) {
            let role_lower = role.to_ascii_lowercase();

            // Reject non-user roles
            if NON_USER_ROLES
                .iter()
                .any(|&non_user| role_lower.contains(non_user))
            {
                return false;
            }

            // Accept user roles
            if USER_ROLES.iter().any(|&user| role_lower.contains(user)) {
                return true;
            }

            return false; // Unknown nested role - reject
        }
    }

    // Check direct role field (other formats)
    if let Some(role) = value.get("role").and_then(|v| v.as_str()) {
        let role_lower = role.to_ascii_lowercase();

        // Explicitly reject non-user roles
        if NON_USER_ROLES
            .iter()
            .any(|&non_user| role_lower.contains(non_user))
        {
            return false;
        }

        // Accept known user roles
        if USER_ROLES.iter().any(|&user| role_lower.contains(user)) {
            return true;
        }

        return false; // Unknown direct role - reject
    }

    // Codex JSONL format stores roles inside payload.role
    if let Some(payload) = value.get("payload") {
        if let Some(role) = payload.get("role").and_then(|v| v.as_str()) {
            let role_lower = role.to_ascii_lowercase();
            if NON_USER_ROLES
                .iter()
                .any(|&non_user| role_lower.contains(non_user))
            {
                return false;
            }
            if USER_ROLES.iter().any(|&user| role_lower.contains(user)) {
                return true;
            }
            return false;
        }
    }

    // No role indicators found - reject (be restrictive)
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_codex_payload_prompts() {
        let value = json!({
            "timestamp": "2025-11-15T11:30:47.571Z",
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "user",
                "content": [
                    {"type": "input_text", "text": "Summarize AGENTS.md for me."}
                ]
            }
        });
        let prompt = extract_prompt_text(&value);
        assert_eq!(prompt.as_deref(), Some("Summarize AGENTS.md for me."));
    }

    #[test]
    fn skips_codex_assistant_payloads() {
        let value = json!({
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "assistant",
                "content": [
                    {"type": "output_text", "text": "Sure!"}
                ]
            }
        });
        assert!(extract_prompt_text(&value).is_none());
    }
}
