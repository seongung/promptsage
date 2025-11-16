use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ParsedReview {
    pub improved_prompt: String,
    pub explanation: String,
    pub score: Option<i32>,
    pub tags: Vec<String>,
    pub metadata: Option<Value>,
}

pub fn parse_payload(payload: &str) -> Result<ParsedReview> {
    if payload.trim().is_empty() {
        return Err(anyhow!("empty provider response"));
    }

    // First attempt: try parsing as-is
    if let Ok(parsed) = try_parse(payload) {
        return Ok(parsed);
    }

    // Second attempt: extract from markdown code blocks (```json or ```)
    if let Some(extracted) = extract_from_markdown(payload) {
        if let Ok(parsed) = try_parse(extracted) {
            return Ok(parsed);
        }
    }

    // Third attempt: line-by-line fallback
    for line in payload.lines() {
        if let Ok(parsed) = try_parse(line) {
            return Ok(parsed);
        }
    }

    // Fourth attempt: prose extraction fallback (extract from markdown headings/labels)
    if let Ok(parsed) = extract_from_prose(payload) {
        eprintln!("WARNING: Parsed response from prose format instead of JSON contract");
        return Ok(parsed);
    }

    Err(anyhow!(
        "provider output did not match review contract JSON. Response preview: {}",
        truncate_for_error(payload, 500)
    ))
}

fn try_parse(input: &str) -> Result<ParsedReview> {
    let cleaned = input.trim();
    if cleaned.is_empty() {
        return Err(anyhow!("no JSON content found in provider output"));
    }

    // Try direct contract match
    if let Ok(contract) = serde_json::from_str::<ReviewContract>(cleaned) {
        return Ok(contract.into());
    }

    // Try parsing as generic JSON to extract embedded content
    if let Ok(value) = serde_json::from_str::<Value>(cleaned) {
        if let Some(embedded) = extract_embedded_json(&value) {
            // Try markdown extraction on embedded content first
            if let Some(markdown_extracted) = extract_from_markdown(embedded) {
                if let Ok(contract) = serde_json::from_str::<ReviewContract>(markdown_extracted) {
                    return Ok(contract.into());
                }
            }

            // Try parsing embedded content directly as contract
            if let Ok(contract) = serde_json::from_str::<ReviewContract>(embedded) {
                return Ok(contract.into());
            }
        }
    }

    Err(anyhow!(
        "provider output did not match review contract JSON"
    ))
}

fn extract_embedded_json(value: &Value) -> Option<&str> {
    // Claude: {"type":"result", "result":"<json or prose>"}
    if let Some(s) = value.get("result").and_then(|v| v.as_str()) {
        return Some(s);
    }

    // Generic text field
    if let Some(s) = value.get("text").and_then(|v| v.as_str()) {
        return Some(s);
    }

    // Nested Codex-style item.text
    if let Some(item) = value.get("item") {
        if let Some(s) = extract_embedded_json(item) {
            return Some(s);
        }
    }

    // data.text fallback
    if let Some(data) = value.get("data") {
        if let Some(s) = data.get("text").and_then(|v| v.as_str()) {
            return Some(s);
        }
    }

    None
}

/// Extract JSON from markdown code blocks (```json ... ``` or ``` ... ```)
fn extract_from_markdown(input: &str) -> Option<&str> {
    let trimmed = input.trim();

    // Look for ```json ... ``` blocks
    if let Some(start_idx) = trimmed.find("```json") {
        let after_fence = &trimmed[start_idx + 7..]; // Skip "```json"
        if let Some(end_idx) = after_fence.find("```") {
            let content = after_fence[..end_idx].trim();
            if !content.is_empty() {
                return Some(content);
            }
        }
    }

    // Look for generic ``` ... ``` blocks (but only if they contain JSON-like content)
    if let Some(start_idx) = trimmed.find("```") {
        let after_fence = &trimmed[start_idx + 3..];
        // Skip language identifier if present (e.g., ```javascript, ```python)
        let content_start = after_fence.find('\n').map(|i| i + 1).unwrap_or(0);
        let remaining = &after_fence[content_start..];

        if let Some(end_idx) = remaining.find("```") {
            let content = remaining[..end_idx].trim();
            // Only return if it looks like JSON (starts with { or [)
            if !content.is_empty() && (content.starts_with('{') || content.starts_with('[')) {
                return Some(content);
            }
        }
    }

    None
}

/// Truncate text for error messages
fn truncate_for_error(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        text.to_string()
    } else {
        format!(
            "{}... [truncated {} total chars]",
            &text[..max_chars],
            text.len()
        )
    }
}

/// Extract review fields from prose/markdown format as last resort
/// Looks for patterns like "**Improved Prompt:** ...", "Explanation: ...", etc.
fn extract_from_prose(input: &str) -> Result<ParsedReview> {
    let improved_prompt = extract_field_prose(
        input,
        &[
            "**Improved Prompt:**",
            "**Improved:**",
            "Improved Prompt:",
            "Improved:",
        ],
    )
    .ok_or_else(|| anyhow!("no improved_prompt field found in prose"))?;

    let explanation = extract_field_prose(
        input,
        &[
            "**Explanation:**",
            "**Why:**",
            "Explanation:",
            "Why:",
            "Reasoning:",
        ],
    )
    .ok_or_else(|| anyhow!("no explanation field found in prose"))?;

    let score = extract_score_prose(input);
    let tags = extract_tags_prose(input);

    Ok(ParsedReview {
        improved_prompt: improved_prompt.trim().to_string(),
        explanation: explanation.trim().to_string(),
        score,
        tags,
        metadata: None,
    })
}

/// Extract a field value from prose using various label patterns
fn extract_field_prose<'a>(input: &'a str, labels: &[&str]) -> Option<&'a str> {
    for label in labels {
        if let Some(start_idx) = input.find(label) {
            let after_label = &input[start_idx + label.len()..];
            let trimmed = after_label.trim_start();

            // Find the end: next bold label, double newline, or end of string
            let end_idx = trimmed
                .find("\n**")
                .or_else(|| trimmed.find("\n\n"))
                .unwrap_or(trimmed.len());

            let value = trimmed[..end_idx].trim();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

/// Extract score from prose (looks for patterns like "**Score:** 85" or "Score: 85/100")
fn extract_score_prose(input: &str) -> Option<i32> {
    let labels = ["**Score:**", "Score:", "**Rating:**", "Rating:"];

    for label in labels {
        if let Some(start_idx) = input.find(label) {
            let after_label = &input[start_idx + label.len()..];
            let trimmed = after_label.trim_start();

            // Extract first number (up to newline or space)
            let end_idx = trimmed
                .find(|c: char| !c.is_ascii_digit() && c != '/')
                .unwrap_or(trimmed.len());

            if let Ok(score) = trimmed[..end_idx]
                .split('/')
                .next()
                .unwrap_or("")
                .parse::<i32>()
            {
                return Some(score);
            }
        }
    }
    None
}

/// Extract tags from prose (looks for patterns like "**Tags:** tag1, tag2" or comma/space separated)
fn extract_tags_prose(input: &str) -> Vec<String> {
    let labels = ["**Tags:**", "Tags:", "**Categories:**", "Categories:"];

    for label in labels {
        if let Some(start_idx) = input.find(label) {
            let after_label = &input[start_idx + label.len()..];
            let trimmed = after_label.trim_start();

            // Find end of tags (newline or end of string)
            let end_idx = trimmed.find('\n').unwrap_or(trimmed.len());
            let tags_str = trimmed[..end_idx].trim();

            if !tags_str.is_empty() {
                return tags_str
                    .split(&[',', ';'][..])
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }
    Vec::new()
}

#[derive(Debug, Deserialize)]
struct ReviewContract {
    improved_prompt: String,
    explanation: String,
    #[serde(default)]
    score: Option<i32>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    metadata: Option<Value>,
}

impl From<ReviewContract> for ParsedReview {
    fn from(value: ReviewContract) -> Self {
        ParsedReview {
            improved_prompt: value.improved_prompt.trim().to_string(),
            explanation: value.explanation.trim().to_string(),
            score: value.score,
            tags: value.tags.unwrap_or_default(),
            metadata: value.metadata,
        }
    }
}
