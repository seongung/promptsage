# Quickstart: Prompt Sage TUI – Prompt History Review

**Feature**: specs/001-prompt-sage-tui/spec.md  
**Plan**: specs/001-prompt-sage-tui/plan.md  
**Date**: 2025-11-15

## Prerequisites

- Rust toolchain (stable, 1.79+).
- Claude Code (`claude`) and/or Codex (`codex exec`) CLIs installed and authenticated locally (`which claude`, `which codex` should succeed).
- Prompt history available under:
  - `~/.claude/projects/**.jsonl`
  - `~/.codex/sessions/**.jsonl`
- Optional: edit `~/.config/prompt-sage/config.toml` to override default source roots or provider enablement before the first run.

## Build and Run

```bash
# From the repository root
cargo build --release

# Launch the TUI
target/release/prompt-sage tui
```

## Review Workflows (External CLIs)

The review worker invokes external CLIs with an explicit constitution/system prompt and a "Review: <prompt>" request.
Examples (exact quoting may be adjusted by the implementation):

```bash
# Claude review example
claude -p --output-format json --max-turns 1 \
  --append-system-prompt "<constitution>" \
  "Review: <prompt>"

# Codex review example
codex exec --json --sandbox read-only --ask-for-approval on-request \
  --cd <dir> "<constitution + Review: <prompt>>"
```

## Core TUI Usage

- Use the projects area to select a project.
- Use the prompts table to choose a prompt by date/session/snippet.
- Press the review key (e.g., `r`) to enqueue a review job.
- View original and improved prompts side by side; copy or export improved prompts as needed.
# Provider Setup Checklist

1. Run `claude --version` (if using Claude Code histories) and ensure the CLI can access your account in read-only mode.
2. Run `codex exec --help` (if using Codex histories) and confirm sandbox/approval flags work in your environment.
3. Verify the directories referenced in the config exist and contain JSONL logs; run `prompt-sage index --stats` to confirm.

# Troubleshooting

- `No providers configured`: update `~/.config/prompt-sage/config.toml` with at least one source, then rerun.
- `codex exec` requires approval: pass `--ask-for-approval on-request` (already default in the worker) and approve once; future runs reuse cached approvals per provider.
- Long startup times: use the `--provider` filter flags or trim old log directories to reduce indexing cost.
