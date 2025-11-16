use std::{
    borrow::Cow,
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use clap::Args;
use glob::glob;
use serde_json::Value;
use which::which;

use crate::providers::{claude_spike, codex_spike};

const DEFAULT_CONSTITUTION: &str = ".specify/memory/constitution.md";
const CLAUDE_GLOB: &str = "~/.claude/projects/**/*.jsonl";
const CODEX_GLOB: &str = "~/.codex/sessions/**/*.jsonl";

#[derive(Args, Debug)]
pub struct SpikeArgs {
    /// Number of prompts to sample per provider
    #[arg(long, default_value_t = 2)]
    pub sample_count: usize,
    /// Optional override for Claude CLI path
    #[arg(long, value_name = "PATH")]
    pub claude_bin: Option<PathBuf>,
    /// Optional override for Codex CLI path
    #[arg(long, value_name = "PATH")]
    pub codex_bin: Option<PathBuf>,
    /// Attempt to invoke the Claude CLI for the first sampled prompt
    #[arg(long, default_value_t = false)]
    pub run_claude: bool,
    /// Attempt to invoke the Codex CLI for the first sampled prompt
    #[arg(long, default_value_t = false)]
    pub run_codex: bool,
    /// Optional override for the constitution/system prompt file
    #[arg(long, value_name = "PATH")]
    pub constitution_path: Option<PathBuf>,
}

pub fn run(args: SpikeArgs) -> Result<()> {
    let constitution = load_constitution(args.constitution_path.as_deref())?;

    let claude_bin = if args.run_claude {
        Some(resolve_binary("claude", args.claude_bin.as_deref())?)
    } else {
        None
    };
    let codex_bin = if args.run_codex {
        Some(resolve_binary("codex", args.codex_bin.as_deref())?)
    } else {
        None
    };

    let claude_samples = collect_samples("claude", CLAUDE_GLOB, args.sample_count)?;
    let codex_samples = collect_samples("codex", CODEX_GLOB, args.sample_count)?;

    if claude_samples.is_empty() {
        eprintln!("warn: no Claude prompts found under {}", CLAUDE_GLOB);
    } else {
        print_samples("Claude", &claude_samples);
    }

    if codex_samples.is_empty() {
        eprintln!("warn: no Codex prompts found under {}", CODEX_GLOB);
    } else {
        print_samples("Codex", &codex_samples);
    }

    if let Some(bin) = claude_bin {
        if let Some(sample) = claude_samples.first() {
            match claude_spike::run(&bin, &sample.prompt, &constitution) {
                Ok(output) => {
                    println!(
                        "Claude CLI exited with status {}",
                        output
                            .status
                            .map(|code| code.to_string())
                            .unwrap_or_else(|| "unknown".into())
                    );
                    print_command_summary("Claude", &output.stdout, &output.stderr);
                }
                Err(err) => eprintln!("error: failed to run Claude CLI: {err:?}"),
            }
        } else {
            eprintln!("warn: skipping Claude CLI call because no prompts were sampled");
        }
    } else if args.run_claude {
        eprintln!("warn: Claude CLI not found on PATH; skipping --run-claude");
    }

    if let Some(bin) = codex_bin {
        if let Some(sample) = codex_samples.first() {
            match codex_spike::run(&bin, &sample.prompt, &constitution, sample.file.parent()) {
                Ok(output) => {
                    println!(
                        "Codex CLI exited with status {}",
                        output
                            .status
                            .map(|code| code.to_string())
                            .unwrap_or_else(|| "unknown".into())
                    );
                    print_command_summary("Codex", &output.stdout, &output.stderr);
                }
                Err(err) => eprintln!("error: failed to run Codex CLI: {err:?}"),
            }
        } else {
            eprintln!("warn: skipping Codex CLI call because no prompts were sampled");
        }
    } else if args.run_codex {
        eprintln!("warn: Codex CLI not found on PATH; skipping --run-codex");
    }

    Ok(())
}

fn load_constitution(path_override: Option<&Path>) -> Result<String> {
    let path = path_override
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONSTITUTION));
    std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read constitution file at {}", path.display()))
}

fn collect_samples(provider: &str, pattern: &str, limit: usize) -> Result<Vec<PromptSample>> {
    let expanded = expand_home(pattern)?;
    let mut samples = Vec::new();
    let mut utf8_warned = HashSet::new();

    for entry in glob(&expanded).with_context(|| format!("invalid glob pattern {expanded}"))? {
        let path = match entry {
            Ok(p) => p,
            Err(e) => {
                eprintln!("warn: skipping glob entry error: {e}");
                continue;
            }
        };
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(err) => {
                eprintln!("warn: unable to open {}: {err}", path.display());
                continue;
            }
        };
        let mut reader = BufReader::new(file);
        let mut buffer = Vec::new();
        loop {
            buffer.clear();
            let read = reader
                .read_until(b'\n', &mut buffer)
                .with_context(|| format!("unable to read line from {}", path.display()))?;
            if read == 0 {
                break;
            }

            let cow = String::from_utf8_lossy(&buffer);
            if matches!(cow, Cow::Owned(_)) && utf8_warned.insert(path.clone()) {
                eprintln!(
                    "warn: non-UTF8 data encountered in {}; lossy conversion applied",
                    path.display()
                );
            }

            let line = cow.trim();
            if line.is_empty() {
                continue;
            }
            let value: Value = match serde_json::from_str(line) {
                Ok(val) => val,
                Err(err) => {
                    eprintln!(
                        "warn: invalid JSON in {}: {} (line truncated)",
                        path.display(),
                        err
                    );
                    continue;
                }
            };
            if let Some(prompt) = extract_prompt(provider, &value) {
                samples.push(PromptSample {
                    file: path.clone(),
                    prompt,
                });
                if samples.len() >= limit {
                    return Ok(samples);
                }
            }
        }
        if samples.len() >= limit {
            break;
        }
    }

    Ok(samples)
}

fn expand_home(pattern: &str) -> Result<String> {
    if let Some(stripped) = pattern.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            let mut buf = home;
            buf.push(stripped);
            return Ok(buf.to_string_lossy().into_owned());
        } else {
            return Err(anyhow!(
                "cannot resolve ~ in pattern {pattern}: home directory not found"
            ));
        }
    }
    Ok(pattern.to_string())
}

fn extract_prompt(_provider: &str, value: &Value) -> Option<String> {
    if let Some(role) = value.get("role").and_then(|v| v.as_str()) {
        let normalized = role.to_ascii_lowercase();
        if normalized != "user" && normalized != "human" && normalized != "client" {
            return None;
        }
    }

    const CANDIDATE_KEYS: &[&str] = &[
        "prompt",
        "text",
        "input",
        "user_input",
        "body",
        "raw_text",
        "message",
        "content",
    ];

    for key in CANDIDATE_KEYS {
        if let Some(node) = value.get(*key) {
            if let Some(text) = extract_text_from_value(node) {
                if let Some(clean) = sanitize_prompt(&text) {
                    return Some(clean);
                }
            }
        }
    }

    if let Some(text) = value.as_str().and_then(|s| sanitize_prompt(s)) {
        return Some(text);
    }

    None
}

fn print_samples(label: &str, samples: &[PromptSample]) {
    println!("=== {label} samples ===");
    for (idx, sample) in samples.iter().enumerate() {
        let snippet: String = sample.prompt.chars().take(140).collect();
        println!(
            "#{idx}: {} :: {}",
            sample.file.display(),
            snippet.replace('\n', " ")
        );
    }
}

fn print_command_summary(provider: &str, stdout: &str, stderr: &str) {
    println!("--- {provider} stdout ---");
    if let Some(summary) = summarize_json(stdout) {
        println!("{summary}");
    } else {
        let truncated_stdout: String = truncate_text(stdout, 200);
        if truncated_stdout.is_empty() {
            println!("<empty>");
        } else {
            println!("{truncated_stdout}");
        }
    }
    if !stderr.trim().is_empty() {
        println!("--- {provider} stderr ---");
        println!("{}", truncate_text(stderr, 200));
    }
}

struct PromptSample {
    file: PathBuf,
    prompt: String,
}

fn resolve_binary(default_name: &str, override_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = override_path {
        if path.exists() {
            return Ok(path.to_path_buf());
        } else {
            return Err(anyhow!("override path {} does not exist", path.display()));
        }
    }

    which(default_name).map_err(|_| anyhow!("{default_name} CLI not found on PATH"))
}

fn extract_text_from_value(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.to_owned()),
        Value::Array(items) => {
            let mut parts = Vec::new();
            for item in items {
                if let Some(text) = extract_text_from_value(item) {
                    parts.push(text);
                } else if let Some(text) = item
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_owned())
                {
                    parts.push(text);
                }
            }
            if parts.is_empty() {
                None
            } else {
                Some(parts.join("\n"))
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
    if looks_like_command(trimmed) {
        return None;
    }
    Some(trimmed.to_owned())
}

fn looks_like_command(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("<command-") || lower.starts_with("/command/")
}

fn summarize_json(payload: &str) -> Option<String> {
    let trimmed = payload.trim();
    if trimmed.is_empty() {
        return Some("<empty>".into());
    }

    for line in trimmed.lines() {
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            if let Some(improved) = value.get("improved_prompt").and_then(|v| v.as_str()) {
                let explanation = value
                    .get("explanation")
                    .and_then(|v| v.as_str())
                    .unwrap_or("<missing>");
                return Some(format!(
                    "improved_prompt: {}\nexplanation: {}",
                    truncate_text(improved, 120),
                    truncate_text(explanation, 120)
                ));
            }
        }
    }

    None
}

fn truncate_text(text: &str, limit: usize) -> String {
    let mut truncated = text.chars().take(limit).collect::<String>();
    if text.chars().count() > limit {
        truncated.push_str("…");
    }
    truncated.replace('\n', " ")
}
