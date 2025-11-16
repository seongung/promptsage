use anyhow::{Context, Result};
use tokio::process::Command;

use super::{ProviderRequest, ProviderResponse};

pub async fn run_review(request: &ProviderRequest<'_>) -> Result<ProviderResponse> {
    let mut command = Command::new(request.binary);
    command
        .arg("-p")
        .arg("--output-format")
        .arg("json")
        .arg("--model")
        .arg("haiku")
        .arg("--max-turns")
        .arg("1")
        .arg("--append-system-prompt")
        .arg(request.constitution)
        .arg(build_review_prompt(request));

    if let Some(dir) = request.working_dir {
        command.current_dir(dir);
    }

    let output = command
        .output()
        .await
        .with_context(|| "failed to invoke Claude CLI")?;
    Ok(ProviderResponse::from_output(output))
}

fn build_review_prompt(request: &ProviderRequest<'_>) -> String {
    format!(
        "You are Prompt Sage, a friendly writing coach who rewrites prompts so they sound natural, clear, and grammatically correct. Polish the prompt below so it keeps the user's intent, emphasizes key constraints, and reads like a native speaker's request. Prefer short paragraphs over rigid bullet lists unless the prompt explicitly asks for a list.

Respond with JSON (no markdown fences, just the JSON object):
{{
  \"improved_prompt\": \"Write TWO variations in one string, formatted as \\\"1) ...\\n2) ...\\\" so both prompts are easy to copy.\",
  \"explanation\": \"Two or three conversational sentences (in Korean) that describe the main grammar/clarity tweaks (no bullet lists).\",
  \"score\": 1-100,
  \"tags\": [\"short\", \"list\", \"of\", \"tone\", \"adjustments\"]
}}

Example:
{{\"improved_prompt\":\"1) Document how to migrate the billing service to the new Stripe keys, including rollback steps and verification checkpoints.\\n2) Outline the end-to-end Stripe key rotation plan with rollback triggers, validation steps, and owner assignments.\",\"explanation\":\"두 가지 버전을 제안하면서도 핵심 제약과 원하는 산출물을 분명하게 적었습니다.\",\"score\":88,\"tags\":[\"clarity\",\"tone\"]}}

Return JSON only—no extra commentary or fences.

Prompt to review:
{}",
        request.prompt
    )
}
