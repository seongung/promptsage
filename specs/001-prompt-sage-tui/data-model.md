# Data Model: Prompt Sage TUI – Prompt History Review

**Feature**: specs/001-prompt-sage-tui/spec.md  
**Plan**: specs/001-prompt-sage-tui/plan.md  
**Date**: 2025-11-15

## Entities

### PromptSource

- **Description**: Physical log source for prompts.
- **Fields**:
  - `id`: internal identifier (string, derived from provider + root path).
  - `provider`: enum (`claude`, `codex`).
  - `root_path`: absolute filesystem path to JSONL logs.
  - `enabled`: boolean flag indicating whether this source is active.
- **Relationships**:
  - One `PromptSource` to many `ProjectSession`s.
- **Validation rules**:
  - `root_path` MUST resolve to an existing directory before indexing is attempted.

### ProjectSession

- **Description**: Grouping of prompts by provider, project, and date or time window.
- **Fields**:
  - `id`: internal identifier (provider + project + date + session hash).
  - `provider`: enum (`claude`, `codex`).
  - `project`: project identifier (string, from log metadata or derived path).
  - `date`: calendar date (YYYY-MM-DD) for grouping.
  - `session_id`: opaque session identifier (string) when present in raw logs.
- **Relationships**:
  - One `ProjectSession` to many `PromptRecord`s.
- **Validation rules**:
  - `date` MUST be derivable from the log timestamp; if not, session is skipped or marked with a clear fallback.

### PromptRecord

- **Description**: A single prompt extracted from history.
- **Fields**:
  - `id`: internal identifier (provider + project + session + timestamp or index).
  - `provider`: enum (`claude`, `codex`).
  - `project`: project identifier (string).
  - `date`: date (YYYY-MM-DD).
  - `session_id`: session identifier (string, optional).
  - `timestamp`: precise timestamp from the log (if available).
  - `snippet`: short preview of the prompt text.
  - `length`: number of characters or tokens in the prompt.
  - `status`: enum (`unreviewed`, `reviewed_ok`, `reviewed_error`) for display in the TUI.
  - `raw_text`: full prompt text used for review.
- **Relationships**:
  - One `PromptRecord` to zero or one `PromptReview`.
- **Validation rules**:
  - `raw_text` MUST be non-empty for a record to be considered reviewable.

### PromptReview

- **Description**: Outcome of a review for a given prompt.
- **Fields**:
  - `id`: internal identifier (matching `PromptRecord.id` plus provider).
  - `prompt_record_id`: foreign key reference to `PromptRecord`.
  - `provider`: enum (`claude`, `codex`) used for the review.
  - `improved_prompt`: improved prompt text per the "Prompt Review Coach" contract.
  - `explanation`: short explanation of changes and rationale.
  - `score`: optional numeric score (0–100) or qualitative label.
  - `tags`: optional list of labels (e.g., `clarity`, `safety`, `structure`).
  - `created_at`: timestamp when the review completed.
  - `status`: enum (`pending`, `completed`, `failed`).
- **Relationships**:
  - Many `PromptReview`s MAY exist for one `PromptRecord` over time, but the TUI will typically surface the latest.
- **Validation rules**:
  - `improved_prompt` MUST be non-empty on successful reviews.
  - `explanation` SHOULD be concise and understandable in isolation.

### ReviewJob

- **Description**: Internal job representation for the async worker.
- **Fields**:
  - `id`: opaque job identifier.
  - `prompt_record_id`: reference to `PromptRecord`.
  - `provider`: enum (`claude`, `codex`).
  - `working_dir`: optional directory path in which to run provider CLI commands.
  - `state`: enum (`queued`, `running`, `completed`, `failed`, `cancelled`).
  - `error`: optional structured error information when state is `failed`.
- **State transitions**:
  - `queued` → `running` → `completed` | `failed`.
  - `queued` | `running` → `cancelled` (if cancellation is supported).

## Derived Views and Filters

- Prompt list view:
  - Derived from `PromptRecord` and the latest `PromptReview` (if any).
  - Supports filters by provider, project, date range, and review status.
- Project view:
  - Derived from `ProjectSession`, showing counts of prompts and review coverage per session.

