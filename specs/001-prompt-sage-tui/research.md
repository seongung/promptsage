# Research: Prompt Sage TUI – Prompt History Review

**Feature**: specs/001-prompt-sage-tui/spec.md  
**Plan**: specs/001-prompt-sage-tui/plan.md  
**Date**: 2025-11-15

## Language and Runtime

- **Decision**: Use Rust (stable, 1.79+) for the Prompt Sage TUI binary.
- **Rationale**: Rust provides strong safety guarantees (no GC pauses, ownership model), good ecosystem support for
  async I/O and TUIs, and aligns with the need for a responsive, resource-efficient terminal application.
- **Alternatives considered**:
  - Go: simpler concurrency but weaker TUI ecosystem for rich layouts and less fine-grained control over terminal
    rendering.
  - Python: rapid iteration but higher runtime overhead and packaging complexity for cross-platform CLI distribution.

## TUI Stack (ratatui + crossterm)

- **Decision**: Use Ratatui for layout and widgets, and Crossterm for terminal I/O and event handling.
- **Rationale**: Ratatui offers a declarative layout system and common components (tables, split panes) well-suited to
  the single-screen multi-pane design; Crossterm provides cross-platform terminal control on macOS/Linux/WSL.
- **Alternatives considered**:
  - Termion: simpler but less actively maintained and with fewer high-level abstractions.
  - Cursive: good for form-based UIs but less aligned with the table-heavy, pane-based interaction model.

## Async Concurrency Model (Tokio Worker)

- **Decision**: Use Tokio to manage an async worker queue for prompt review jobs, keeping the main TUI loop synchronous
  and focused on rendering and input handling.
- **Rationale**: Tokio integrates well with external process spawning, timers, and channels; this supports non-blocking
  reviews while ensuring that TUI input remains responsive.
- **Alternatives considered**:
  - Thread-per-review without async runtime: simpler but less flexible under load and harder to coordinate with
    structured cancellation and timeouts.
  - Full async TUI loop: possible but adds complexity to rendering and event handling for limited benefit in this domain.

## Transcript Indexing Strategy

- **Decision**: Implement file-based indexers that scan JSONL files under `~/.claude/projects/**.jsonl` and
  `~/.codex/sessions/**.jsonl`, using `glob` and `serde_json`, and materialize `PromptRecord` instances with minimal
  derived metadata (provider, project, date, session id, snippet, length, review status).
- **Rationale**: This keeps indexing simple and portable, fully local, and avoids introducing a database for the initial
  scope; it also respects read-only constraints by never modifying source logs.
- **Alternatives considered**:
  - Embedding a lightweight database (e.g., SQLite) for indexing: better query performance but higher complexity and
    migration overhead.
  - On-demand streaming from JSONL without any intermediate index: simpler but risks repeated I/O and slower navigation
    for large histories.

## Review Invocation Contract (Claude and Codex)

- **Decision**: Treat Claude and Codex as external CLI tools invoked by the worker, sending only the selected prompt
  text plus a system prompt (constitution) and expecting a JSON response conforming to a shared "Prompt Review Coach"
  contract.
- **Rationale**: This preserves the read-only, auditable behavior required by the constitution, and keeps Prompt Sage
  TUI agnostic to network credentials and provider SDKs.
- **Alternatives considered**:
  - Direct HTTP API integrations: more control over retries and error handling but requires credential management and
    increases the privacy/safety surface.
  - Embedding model calls directly through SDKs: similar concerns plus tighter coupling to specific providers.

## Data Volume and Performance

- **Decision**: Target interactive performance for up to tens of thousands of prompt records across projects per user,
  with indexing done at startup and incremental updates thereafter.
- **Rationale**: This matches typical usage for frequent Claude Code/Codex users without prematurely optimizing for
  extreme scales; the performance goal of <3s median review turnaround and responsive navigation remains achievable.
- **Alternatives considered**:
  - Aggressive caching of derived views on disk: could improve startup but adds complexity and invalidation concerns.
  - Streaming-only design with no in-memory index: might avoid memory growth but complicates UX and filtering.

