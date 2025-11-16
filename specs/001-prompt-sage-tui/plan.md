# Implementation Plan: Prompt Sage TUI – Prompt History Review

**Branch**: `001-prompt-sage-tui` | **Date**: 2025-11-15 | **Spec**: `specs/001-prompt-sage-tui/spec.md`
**Input**: Feature specification from `specs/001-prompt-sage-tui/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Prompt Sage TUI is a Rust TUI application that indexes Claude Code and Codex prompt history from local JSONL logs,
groups them by project and date, and lets users request improved prompts plus explanations in a safe, read-only way.
The technical approach is a single binary using Ratatui for layout, Crossterm for terminal control, Tokio for async
work, and a background worker model for calling external AI CLIs (Claude and Codex) with explicit, auditable prompts.

### Milestones Overview

0. **Feasibility spike – fetch prompts & run LLM CLIs**
   - Tasks:
     - Implement a minimal Rust CLI subcommand that reads a small sample of prompts from local Claude and Codex JSONL
       logs and prints snippets to stdout.
     - Invoke `claude` and `codex exec` using the planned review commands for a sample prompt and log raw JSON output.
   - Risks:
     - CLIs may not be installed or available on `PATH` in all environments.
     - Provider output JSON may differ from the planned contract shape.
   - Success criteria:
     - At least one prompt per provider can be loaded from logs and successfully reviewed via the CLI with a valid JSON
       payload that can be parsed into the internal review model or clearly mapped to it.
   - Demo points:
     - Run `prompt-sage spike` (or similar) to show sample prompt snippets and one successful Claude + one successful
       Codex review call with parsed results or clearly logged mismatches.

1. **Transcript indexers (Claude, Codex)**
   - Tasks:
     - Implement streaming JSONL indexers for `~/.claude/projects/**.jsonl` and `~/.codex/sessions/**.jsonl` using `glob`
       and `serde_json`.
     - Define a shared `PromptRecord` type (provider, project, date, session id, snippet, length, review status).
     - Build incremental indexing and cache to avoid re-reading unchanged files.
   - Risks:
     - Variation in JSONL schema between versions of Claude Code and Codex.
     - Large histories causing slow startup or high memory usage.
   - Success criteria:
     - ≥95% of prompts in a test corpus are parsed into `PromptRecord`s without crashing; invalid lines are logged and
       skipped.
   - Demo points:
     - Run `prompt-sage index --stats` and show counts per provider/project/date.

2. **TUI shell + tables**
   - Tasks:
     - Implement Ratatui-based single-screen layout matching the spec’s UI reference (header, projects row, prompts
       table, original/improved panes, footer keybindings).
     - Wire keyboard navigation with `crossterm` (pane focus, project selection, prompt selection, scrolling).
   - Risks:
     - Limited terminal size and differing fonts impacting readability.
   - Success criteria:
     - Users can navigate projects and prompts using only the keyboard and view original prompts in the detail pane.
   - Demo points:
     - Run `prompt-sage tui` and live-demo navigation across projects and prompts using the reference layout.

3. **Async worker for reviews**
   - Tasks:
     - Add a Tokio-based background worker queue for "review" jobs so the main TUI loop remains non-blocking.
     - Define an internal `ReviewJob` representation (provider, prompt text, working directory, config) and a small
       state machine (queued, running, completed, failed).
   - Risks:
     - CLI invocation hangs or produces malformed JSON, blocking worker threads.
   - Success criteria:
     - TUI remains responsive (input latency under ~100ms) while multiple reviews are in flight.
   - Demo points:
     - Kick off several reviews and show status transitions (queued → running → completed/failed) without UI freezes.

4. **JSON contract + parser for AI CLIs**
   - Tasks:
     - Define a JSON "Prompt Review Coach" contract for Claude and Codex responses (improved prompt, explanation,
       optional score and tags) and document it in `contracts/`.
     - Implement a robust parser with `serde`/`serde_json` that validates the contract and surfaces clear errors.
     - Encapsulate provider-specific invocation and parsing behind a trait.
   - Risks:
     - Providers returning partial or non-conforming JSON breaking parsing.
   - Success criteria:
     - ≥95% of responses from a test suite of prompts parse successfully into the internal review type; failures are
       handled gracefully.
   - Demo points:
     - Show a successful review for each provider and an example of a handled parsing error.

   Example review commands (run outside the TUI, configured into the worker):

   ```bash
   claude -p --output-format json --max-turns 1 \
     --append-system-prompt "<constitution>" \
     "Review: <prompt>"

   codex exec --json --sandbox read-only --ask-for-approval on-request \
     --cd <dir> "<constitution + Review: <prompt>>"
   ```

5. **Copy/export of improvements**
   - Tasks:
     - Add TUI actions for copying improved prompts (and optionally explanations) to the clipboard or stdout.
     - Implement an export command to write selected improved prompts to a text file for reuse.
   - Risks:
     - Confusing distinction between original and improved prompts when exporting.
   - Success criteria:
     - Users can reliably export improved prompts and explanations without altering logs or source files.
   - Demo points:
     - Review a prompt, copy the improved version, and export a small set of improved prompts to a file.

6. **Config + filters**
   - Tasks:
     - Add a configuration layer (e.g., TOML/JSON) for provider paths, default filters, and safety settings, loaded
       from `~/.config/prompt-sage/config.*`.
     - Implement filters in the TUI (by provider, project, date range, review status, search).
   - Risks:
     - Complex configuration leading to confusing behavior across platforms.
   - Success criteria:
     - Users can narrow prompt lists by provider/project/date and retain settings across sessions.
   - Demo points:
     - Show configuring a default project and filter, then launching the TUI with those applied.

7. **Tests & release**
   - Tasks:
     - Add unit tests for indexers, parsers, and TUI state transitions.
     - Add integration tests for end-to-end flows (index → TUI navigation → review worker → parsed result).
     - Prepare release artifacts and basic documentation.
   - Risks:
     - Flaky tests tied to local file paths or environment.
   - Success criteria:
     - `cargo test` passes on macOS and Linux; core flows are covered by tests that do not depend on external CLIs.
   - Demo points:
     - Run the full test suite and show a minimal release artifact (`cargo build --release`) for distribution.

## Technical Context

<!--
  ACTION REQUIRED: Replace the content in this section with the technical details
  for the project. The structure here is presented in advisory capacity to guide
  the iteration process.
-->

**Language/Version**: Rust (stable, 1.79+)  
**Primary Dependencies**: ratatui, crossterm, tokio, clap, serde, serde_json, anyhow, glob, dirs  
**Storage**: Local filesystem JSONL logs (`~/.claude/projects/**.jsonl`, `~/.codex/sessions/**.jsonl`)  
**Testing**: cargo test, cargo fmt, cargo clippy  
**Target Platform**: macOS and Linux terminals; Windows via WSL for Codex workflows  
**Project Type**: single (CLI/TUI binary)  
**Performance Goals**: Prompt review turnaround <3s median on a modern laptop; TUI input latency low enough that navigation feels instant.  
**Constraints**: Read-only by default; no shell command execution without explicit user approval; portability across macOS/Linux and WSL; avoid leaking secrets in review calls or logs.  
**Scale/Scope**: Single-user local usage; up to tens of thousands of prompt records across projects per installation.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Planned work MUST satisfy the project constitution, at minimum:

- Transcript parsing for Claude Code and OpenAI Codex remains correct, deterministic, and fully tested with representative transcripts.
- No feature introduces unintended code execution; all commands and code paths are read-only by default or gated behind explicit confirmation flows.
- Prompt instructions become clearer and more specific, with explicit output contracts wherever models are involved.
- Changes preserve portability (macOS and Linux first; Windows supported via WSL for Codex workflows) without introducing hard platform dependencies.
- Long-running operations run in background workers or asynchronous flows so the TUI remains responsive to user input.
- Privacy is protected via redaction options and safe logging; no secrets or highly sensitive content are added to telemetry.
- Observability is maintained or improved through structured logs and feature flags for new or risky behavior.

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)
<!--
  ACTION REQUIRED: Replace the placeholder tree below with the concrete layout
  for this feature. Delete unused options and expand the chosen structure with
  real paths (e.g., apps/admin, packages/something). The delivered plan must
  not include Option labels.
-->

```text
# [REMOVE IF UNUSED] Option 1: Single project (DEFAULT)
src/
├── models/
├── services/
├── cli/
└── lib/

tests/
├── contract/
├── integration/
└── unit/

# [REMOVE IF UNUSED] Option 2: Web application (when "frontend" + "backend" detected)
backend/
├── src/
│   ├── models/
│   ├── services/
│   └── api/
└── tests/

frontend/
├── src/
│   ├── components/
│   ├── pages/
│   └── services/
└── tests/

# [REMOVE IF UNUSED] Option 3: Mobile + API (when "iOS/Android" detected)
api/
└── [same as backend above]

ios/ or android/
└── [platform-specific structure: feature modules, UI flows, platform tests]
```

**Structure Decision**: [Document the selected structure and reference the real
directories captured above]

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |
