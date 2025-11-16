# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Prompt Sage TUI** is a terminal-based prompt history review application that indexes prompt logs from Claude Code (`~/.claude/projects/**.jsonl`) and Codex (`~/.codex/sessions/**.jsonl`), enabling developers to review, improve, and export historical prompts using AI-powered analysis.

Key capabilities:
- Indexes and groups prompt history by provider, project, and date
- Displays prompts in a single-screen Ratatui TUI with side-by-side comparison
- Generates improved prompts using Claude or Codex with a "Prompt Review Coach" instruction
- Operates in read-only mode (no code execution or file modifications)
- Supports copy/export of improved prompts for reuse

## Build, Test, and Run Commands

### Development
- `cargo build` - Build the project
- `cargo build --release` - Build optimized release binary
- `cargo run -- tui` - Run the TUI interface
- `cargo run -- index` - Run indexing for Claude/Codex transcripts
- `cargo run -- spike` - Run the feasibility spike (samples prompts, optionally calls CLIs)
- `cargo check` - Fast compilation check without producing binaries

### Testing
- `cargo test` - Run all tests
- `cargo test <test_name>` - Run specific test by name
- `cargo test -- --nocapture` - Run tests with output visible
- `cargo test --no-fail-fast` - Run all tests regardless of failures

### Code Quality
- `cargo clippy` - Run linter
- `cargo fmt` - Format code
- `cargo fmt -- --check` - Check formatting without modifying files

## Architecture Overview

### Core Data Flow

1. **Indexing Layer** (`src/indexing/`)
   - `claude.rs` and `codex.rs` implement provider-specific JSONL parsers
   - Extracts user-authored prompts from session logs (filters out system/agent messages per FR-011)
   - Returns `IndexOutcome` with `PromptRecord` entries, summary stats, and parsing errors
   - Gracefully skips malformed JSONL lines while surfacing warnings

2. **Application State** (`src/app/`)
   - `state.rs` manages the runtime state of selected prompts, reviews, and UI navigation
   - `filter.rs` implements provider/project/date/text filtering logic
   - State is immutable where possible; updates flow unidirectionally through the TUI event loop

3. **TUI Layer** (`src/tui/`)
   - Built with Ratatui + crossterm for terminal rendering
   - `app.rs` contains the main event loop and rendering orchestration
   - `layout.rs` defines the single-screen multi-pane layout (header, project list, prompt list, original/sage detail panes, footer)
   - `detail.rs` renders side-by-side original vs improved prompt comparison
   - `filter_bar.rs` handles interactive filtering UI
   - `export.rs` manages copy-to-clipboard and file export flows

4. **Review Pipeline** (`src/review/` and `src/worker/`)
   - `review_queue.rs` manages asynchronous review job lifecycle (queued → running → completed/failed)
   - `parser.rs` extracts structured review results from JSON provider responses
   - Review jobs send only the prompt text to the AI (no file paths, project identifiers, or surrounding transcript per FR-004)

5. **Provider Integration** (`src/providers/`)
   - `claude.rs` and `codex.rs` invoke respective CLIs with JSON output (`claude -p --output-format json` or `codex exec --json`)
   - `ProviderRequest` encapsulates the constitution (system prompt), user prompt, and working directory
   - `ProviderResponse` captures stdout/stderr/exit status for parsing

6. **Configuration** (`src/config/`)
   - Reads from `~/.config/prompt-sage/config.toml` (or platform-equivalent)
   - Supports custom prompt sources, filter defaults, and safety settings
   - Safety defaults: `read_only=true`, `confirm_reviews=true`, `confirm_exports=true`

### Key Entities

- **PromptRecord**: Single indexed prompt with provider, project, date, session_id, snippet, review status
- **PromptReview**: AI-generated improved prompt + explanation + optional score/tags
- **PromptSource**: Configuration for a log source (provider, root_path, glob_pattern, enabled)
- **ReviewJob**: Background job representation (id, state, timestamps, error info)

## Important Constraints

### Safety & Read-Only Operation (FR-005, FR-010)
- The application MUST NOT execute project code, run shell commands (except provider CLIs), or modify project files
- All indexing and review operations are read-only
- Configuration enforces safety defaults; any changes must preserve these guarantees

### Prompt Privacy (FR-004)
- Only send the selected prompt text to AI providers
- Never include file paths, project identifiers, surrounding transcript context, or session metadata in review requests

### User-Authored Prompts Only (FR-011)
- Filter out system messages, tool calls, agent scaffolding, and internal prompts during indexing
- Only surface prompts directly authored by users for review/export

### Graceful Degradation (FR-007, FR-008)
- Handle missing/malformed JSONL gracefully (skip invalid lines, log warnings, continue)
- Support operation when only one provider's logs are present
- Surface clear error messages for unavailable or slow backends without freezing the TUI

## Development Guidelines

### Module Organization
- Small, focused modules grouped by domain (e.g., `indexing/`, `providers/`, `tui/`)
- Prefer explicit error propagation with `anyhow::Result` over panics
- Use `#[allow(dead_code)]` sparingly; remove unused code when possible

### Async and Background Work
- Tokio runtime used for async provider calls and background review jobs
- TUI event loop runs on main thread; spawns async tasks for reviews to keep UI responsive
- Use `tokio::spawn` for concurrent review jobs; track state via `ReviewJob` entities

### Testing Strategy
- Unit tests for parsers (`indexing::claude`, `indexing::codex`, `review::parser`)
- Integration tests for config loading and filter logic
- Manual smoke tests for TUI flows (see User Stories in spec.md)

### Spike Code Removal
- `src/providers/claude_spike.rs`, `src/providers/codex_spike.rs`, and `src/cli/spike.rs` are temporary feasibility code
- These should be evaluated for removal or refactoring once core features stabilize (see AGENTS.md - 002-remove-spike)

## Configuration Example

Default configuration path: `~/.config/prompt-sage/config.toml`

```toml
[[sources]]
provider = "claude"
root_path = "~/.claude/projects"
enabled = true

[[sources]]
provider = "codex"
root_path = "~/.codex/sessions"
enabled = true

[filters]
# Optional: set default filters
# provider = "claude"
# project = "my-service"
# hide_reviewed = false

[safety]
read_only = true
confirm_reviews = true
confirm_exports = true
allow_exports = true
```

## Performance Targets (SC-002)

- Median review turnaround: <3s on M2 MacBook with stable connectivity
- 90% of reviews complete within 5s
- Indexing should parse ≥95% of test corpus logs without blocking errors

## Typical Workflows

### Adding a New Provider
1. Create `src/indexing/{provider}.rs` implementing the JSONL parsing logic
2. Add provider variant to `ProviderKind` enum in `src/model/mod.rs`
3. Update `src/providers/{provider}.rs` to handle CLI invocation
4. Wire provider into `index_sources()` in `src/indexing/mod.rs`
5. Update config defaults in `src/config/mod.rs`

### Extending the Review Contract
1. Update `ReviewContract` struct in `src/review/parser.rs`
2. Modify `parse_payload()` to handle new fields
3. Update `PromptReview` in `src/model/mod.rs` if persisting new data
4. Adjust TUI rendering in `src/tui/detail.rs` to display new fields

### Adding a New Filter Type
1. Add field to `FilterState` in `src/app/filter.rs`
2. Implement filter logic in `FilterState::matches()`
3. Update `FilterDefaults` and config serialization in `src/config/mod.rs`
4. Wire UI controls in `src/tui/filter_bar.rs`
